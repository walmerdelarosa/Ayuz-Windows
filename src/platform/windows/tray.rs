// Ayuz - Unofficial Control Center for Asus Laptops
// Copyright (C) 2026 Guido Philipp
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see https://www.gnu.org/licenses/.

//! Notification-area (system tray) icon.
//!
//! Runs on its own thread with a hidden window and a plain Win32 message
//! loop, so it does not depend on GTK's event handling. The same window also
//! receives "show yourself" requests from a second Ayuz instance.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicIsize, AtomicU32, Ordering};

use super::ffi::{self, HWND, LPARAM, LRESULT, WPARAM};

pub const WINDOW_CLASS: &str = "AyuzTrayWindow";
const WM_TRAY_CALLBACK: u32 = ffi::WM_APP + 1;
/// Posted by a second instance to bring the main window to the front.
pub const WM_SHOW_REQUEST: u32 = ffi::WM_APP + 2;
const TRAY_UID: u32 = 1;
const MENU_SHOW: usize = 1;
const MENU_QUIT: usize = 2;

#[derive(Debug, Clone, Copy)]
pub enum TrayEvent {
    Show,
    Quit,
}

pub struct TrayConfig {
    pub tooltip: String,
    pub show_label: String,
    pub quit_label: String,
    /// Straight (non-premultiplied) RGBA pixels, row-major, top-down.
    pub icon_rgba: Vec<u8>,
    pub icon_width: u32,
    pub icon_height: u32,
}

struct Shared {
    handler: Box<dyn Fn(TrayEvent) + Send + Sync>,
    tooltip: String,
    show_label: Vec<u16>,
    quit_label: Vec<u16>,
}

static SHARED: OnceLock<Shared> = OnceLock::new();
static HWND_VALUE: AtomicIsize = AtomicIsize::new(0);
static ICON_VALUE: AtomicIsize = AtomicIsize::new(0);
static TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);

fn emit(event: TrayEvent) {
    if let Some(shared) = SHARED.get() {
        (shared.handler)(event);
    }
}

/// Removes the tray icon when dropped (i.e. when the app model goes away).
pub struct TrayGuard;

impl Drop for TrayGuard {
    fn drop(&mut self) {
        remove_icon();
    }
}

/// Starts the tray thread. Only the first call has an effect.
pub fn spawn(config: TrayConfig, handler: impl Fn(TrayEvent) + Send + Sync + 'static) -> TrayGuard {
    let shared = Shared {
        handler: Box::new(handler),
        tooltip: config.tooltip.clone(),
        show_label: ffi::wide(&config.show_label),
        quit_label: ffi::wide(&config.quit_label),
    };
    if SHARED.set(shared).is_err() {
        return TrayGuard;
    }
    let spawned = std::thread::Builder::new()
        .name("ayuz-tray".into())
        .spawn(move || run(config));
    if let Err(e) = spawned {
        tracing::warn!("tray: cannot start thread: {e}");
    }
    TrayGuard
}

/// Removes the icon immediately (call before exiting so no ghost icon stays).
pub fn remove_icon() {
    let hwnd = HWND_VALUE.load(Ordering::SeqCst) as HWND;
    if hwnd.is_null() {
        return;
    }
    let data = notify_data(hwnd);
    // SAFETY: `data` is fully initialised; NIM_DELETE only reads hWnd/uID.
    unsafe {
        ffi::Shell_NotifyIconW(ffi::NIM_DELETE, &data);
    }
}

fn notify_data(hwnd: HWND) -> ffi::NOTIFYICONDATAW {
    let mut data = ffi::NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<ffi::NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: TRAY_UID,
        uFlags: ffi::NIF_MESSAGE | ffi::NIF_ICON | ffi::NIF_TIP,
        uCallbackMessage: WM_TRAY_CALLBACK,
        hIcon: ICON_VALUE.load(Ordering::SeqCst) as ffi::HICON,
        szTip: [0; 128],
        dwState: 0,
        dwStateMask: 0,
        szInfo: [0; 256],
        uVersion: 0,
        szInfoTitle: [0; 64],
        dwInfoFlags: 0,
        guidItem: ffi::GUID::default(),
        hBalloonIcon: std::ptr::null_mut(),
    };
    if let Some(shared) = SHARED.get() {
        ffi::copy_wide(&mut data.szTip, &shared.tooltip);
    }
    data
}

fn add_icon(hwnd: HWND) {
    let data = notify_data(hwnd);
    // SAFETY: `data` is fully initialised and outlives the call.
    unsafe {
        if ffi::Shell_NotifyIconW(ffi::NIM_ADD, &data) == 0 {
            // Already present (e.g. Explorer restarted quickly): refresh it.
            ffi::Shell_NotifyIconW(ffi::NIM_MODIFY, &data);
        }
    }
}

/// Builds an HICON from straight RGBA pixels.
fn create_icon(rgba: &[u8], width: u32, height: u32) -> ffi::HICON {
    let (w, h) = (width as usize, height as usize);
    if rgba.len() < w * h * 4 || w == 0 || h == 0 {
        return std::ptr::null_mut();
    }
    let mut bgra = Vec::with_capacity(w * h * 4);
    // 1-bpp AND mask, rows padded to 16 bits; set bits are transparent.
    let stride = w.div_ceil(16) * 2;
    let mut mask = vec![0u8; stride * h];
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) * 4;
            let (r, g, b, a) = (rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]);
            bgra.extend_from_slice(&[b, g, r, a]);
            if a == 0 {
                mask[y * stride + x / 8] |= 0x80 >> (x % 8);
            }
        }
    }
    // SAFETY: buffers match the declared dimensions and bit depth.
    unsafe {
        ffi::CreateIcon(
            ffi::GetModuleHandleW(std::ptr::null()),
            w as i32,
            h as i32,
            1,
            32,
            mask.as_ptr(),
            bgra.as_ptr(),
        )
    }
}

fn show_menu(hwnd: HWND) {
    let Some(shared) = SHARED.get() else {
        return;
    };
    // SAFETY: standard popup-menu sequence on the window's own thread.
    let chosen = unsafe {
        let menu = ffi::CreatePopupMenu();
        if menu.is_null() {
            return;
        }
        ffi::AppendMenuW(menu, ffi::MF_STRING, MENU_SHOW, shared.show_label.as_ptr());
        ffi::AppendMenuW(menu, ffi::MF_SEPARATOR, 0, std::ptr::null());
        ffi::AppendMenuW(menu, ffi::MF_STRING, MENU_QUIT, shared.quit_label.as_ptr());

        let mut pt = ffi::POINT::default();
        ffi::GetCursorPos(&mut pt);
        // Required so the menu closes when the user clicks elsewhere.
        ffi::SetForegroundWindow(hwnd);
        let cmd = ffi::TrackPopupMenu(
            menu,
            ffi::TPM_RETURNCMD | ffi::TPM_RIGHTBUTTON | ffi::TPM_NONOTIFY,
            pt.x,
            pt.y,
            0,
            hwnd,
            std::ptr::null(),
        );
        ffi::PostMessageW(hwnd, 0, 0, 0);
        ffi::DestroyMenu(menu);
        cmd as usize
    };
    match chosen {
        MENU_SHOW => emit(TrayEvent::Show),
        MENU_QUIT => emit(TrayEvent::Quit),
        _ => {}
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_TRAY_CALLBACK => {
            match (lparam as u32) & 0xFFFF {
                ffi::WM_LBUTTONUP | ffi::WM_LBUTTONDBLCLK => emit(TrayEvent::Show),
                ffi::WM_RBUTTONUP | ffi::WM_CONTEXTMENU => show_menu(hwnd),
                _ => {}
            }
            0
        }
        WM_SHOW_REQUEST => {
            emit(TrayEvent::Show);
            0
        }
        ffi::WM_DESTROY => {
            remove_icon();
            // SAFETY: called on the thread that owns the message loop.
            unsafe { ffi::PostQuitMessage(0) };
            0
        }
        m if m != 0 && m == TASKBAR_CREATED.load(Ordering::SeqCst) => {
            // Explorer restarted: the icon has to be added again.
            add_icon(hwnd);
            0
        }
        // SAFETY: default handling for every other message.
        _ => unsafe { ffi::DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

fn run(config: TrayConfig) {
    let class_name = ffi::wide(WINDOW_CLASS);
    let window_name = ffi::wide("Ayuz");
    let taskbar_created = ffi::wide("TaskbarCreated");

    // SAFETY: plain Win32 window creation on this thread; all strings are
    // NUL-terminated and live until the loop exits.
    unsafe {
        let hinstance = ffi::GetModuleHandleW(std::ptr::null());
        TASKBAR_CREATED.store(
            ffi::RegisterWindowMessageW(taskbar_created.as_ptr()),
            Ordering::SeqCst,
        );

        let class = ffi::WNDCLASSEXW {
            cbSize: std::mem::size_of::<ffi::WNDCLASSEXW>() as u32,
            style: 0,
            lpfnWndProc: Some(window_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance,
            hIcon: std::ptr::null_mut(),
            hCursor: std::ptr::null_mut(),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: std::ptr::null_mut(),
        };
        if ffi::RegisterClassExW(&class) == 0 {
            tracing::warn!("tray: RegisterClassExW failed ({})", ffi::GetLastError());
            return;
        }

        // A hidden top-level window (not message-only) so that it receives
        // the TaskbarCreated broadcast and can be found by FindWindowW.
        let hwnd = ffi::CreateWindowExW(
            0,
            class_name.as_ptr(),
            window_name.as_ptr(),
            0,
            0,
            0,
            0,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            hinstance,
            std::ptr::null_mut(),
        );
        if hwnd.is_null() {
            tracing::warn!("tray: CreateWindowExW failed ({})", ffi::GetLastError());
            return;
        }
        HWND_VALUE.store(hwnd as isize, Ordering::SeqCst);

        let icon = create_icon(&config.icon_rgba, config.icon_width, config.icon_height);
        ICON_VALUE.store(icon as isize, Ordering::SeqCst);
        add_icon(hwnd);

        let mut msg = std::mem::zeroed::<ffi::MSG>();
        while ffi::GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            ffi::TranslateMessage(&msg);
            ffi::DispatchMessageW(&msg);
        }

        remove_icon();
        HWND_VALUE.store(0, Ordering::SeqCst);
        if !icon.is_null() {
            ffi::DestroyIcon(icon);
        }
    }
}

/// Asks an already running instance to show its window. Returns `true` if
/// such an instance was found.
pub fn signal_running_instance() -> bool {
    let class_name = ffi::wide(WINDOW_CLASS);
    // SAFETY: NUL-terminated class name; PostMessageW is thread-safe.
    unsafe {
        let hwnd = ffi::FindWindowW(class_name.as_ptr(), std::ptr::null());
        if hwnd.is_null() {
            return false;
        }
        ffi::PostMessageW(hwnd, WM_SHOW_REQUEST, 0, 0) != 0
    }
}
