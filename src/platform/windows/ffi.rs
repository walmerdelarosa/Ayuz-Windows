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

//! Minimal hand-written Win32 bindings.
//!
//! Only the handful of functions Ayuz needs are declared here, which keeps the
//! Windows build free of extra binding crates and lets the struct layouts be
//! checked against the Windows SDK headers in one place.

#![allow(non_camel_case_types, non_snake_case, clippy::upper_case_acronyms)]

use std::ffi::c_void;

pub type HANDLE = *mut c_void;
pub type HWND = *mut c_void;
pub type HMENU = *mut c_void;
pub type HICON = *mut c_void;
pub type HCURSOR = *mut c_void;
pub type HBRUSH = *mut c_void;
pub type HINSTANCE = *mut c_void;
pub type HKEY = *mut c_void;
pub type HDEVINFO = isize;
pub type BOOL = i32;
pub type BOOLEAN = u8;
pub type WPARAM = usize;
pub type LPARAM = isize;
pub type LRESULT = isize;
pub type WNDPROC = unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;

pub const INVALID_HANDLE_VALUE: HANDLE = -1isize as HANDLE;
pub const INVALID_HDEVINFO: HDEVINFO = -1;

// CreateFileW
pub const GENERIC_READ: u32 = 0x8000_0000;
pub const GENERIC_WRITE: u32 = 0x4000_0000;
pub const FILE_SHARE_READ: u32 = 0x0000_0001;
pub const FILE_SHARE_WRITE: u32 = 0x0000_0002;
pub const OPEN_EXISTING: u32 = 3;
pub const FILE_ATTRIBUTE_NORMAL: u32 = 0x80;

// Errors
pub const ERROR_ALREADY_EXISTS: u32 = 183;
pub const ERROR_SUCCESS: i32 = 0;

// Registry
pub const HKEY_CURRENT_USER: HKEY = 0x8000_0001u32 as i32 as isize as HKEY;
pub const HKEY_LOCAL_MACHINE: HKEY = 0x8000_0002u32 as i32 as isize as HKEY;
pub const REG_SZ: u32 = 1;
pub const RRF_RT_REG_SZ: u32 = 0x0000_0002;
pub const RRF_RT_REG_DWORD: u32 = 0x0000_0010;

// Window messages / styles
pub const WM_DESTROY: u32 = 0x0002;
pub const WM_CLOSE: u32 = 0x0010;
pub const WM_CONTEXTMENU: u32 = 0x007B;
pub const WM_LBUTTONUP: u32 = 0x0202;
pub const WM_LBUTTONDBLCLK: u32 = 0x0203;
pub const WM_RBUTTONUP: u32 = 0x0205;
pub const WM_APP: u32 = 0x8000;
pub const HWND_MESSAGE: HWND = -3isize as HWND;

// Menus
pub const MF_STRING: u32 = 0x0000_0000;
pub const MF_SEPARATOR: u32 = 0x0000_0800;
pub const TPM_RIGHTBUTTON: u32 = 0x0002;
pub const TPM_RETURNCMD: u32 = 0x0100;
pub const TPM_NONOTIFY: u32 = 0x0080;

// Shell_NotifyIconW
pub const NIM_ADD: u32 = 0x0000_0000;
pub const NIM_MODIFY: u32 = 0x0000_0001;
pub const NIM_DELETE: u32 = 0x0000_0002;
pub const NIF_MESSAGE: u32 = 0x0000_0001;
pub const NIF_ICON: u32 = 0x0000_0002;
pub const NIF_TIP: u32 = 0x0000_0004;

pub const SW_SHOWNORMAL: i32 = 1;

// SetupAPI
pub const DIGCF_PRESENT: u32 = 0x0000_0002;
pub const DIGCF_DEVICEINTERFACE: u32 = 0x0000_0010;

// HID
pub const HIDP_STATUS_SUCCESS: i32 = 0x0011_0000;

// Process creation (std::os::windows::process::CommandExt::creation_flags)
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct GUID {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct POINT {
    pub x: i32,
    pub y: i32,
}

#[repr(C)]
pub struct MSG {
    pub hwnd: HWND,
    pub message: u32,
    pub wParam: WPARAM,
    pub lParam: LPARAM,
    pub time: u32,
    pub pt: POINT,
    pub lPrivate: u32,
}

#[repr(C)]
pub struct WNDCLASSEXW {
    pub cbSize: u32,
    pub style: u32,
    pub lpfnWndProc: Option<WNDPROC>,
    pub cbClsExtra: i32,
    pub cbWndExtra: i32,
    pub hInstance: HINSTANCE,
    pub hIcon: HICON,
    pub hCursor: HCURSOR,
    pub hbrBackground: HBRUSH,
    pub lpszMenuName: *const u16,
    pub lpszClassName: *const u16,
    pub hIconSm: HICON,
}

/// `NOTIFYICONDATAW` as declared for `NTDDI_VERSION >= NTDDI_VISTA`.
#[repr(C)]
pub struct NOTIFYICONDATAW {
    pub cbSize: u32,
    pub hWnd: HWND,
    pub uID: u32,
    pub uFlags: u32,
    pub uCallbackMessage: u32,
    pub hIcon: HICON,
    pub szTip: [u16; 128],
    pub dwState: u32,
    pub dwStateMask: u32,
    pub szInfo: [u16; 256],
    /// Union of `uTimeout` / `uVersion`.
    pub uVersion: u32,
    pub szInfoTitle: [u16; 64],
    pub dwInfoFlags: u32,
    pub guidItem: GUID,
    pub hBalloonIcon: HICON,
}

#[repr(C)]
#[derive(Default)]
pub struct SYSTEM_POWER_STATUS {
    pub ACLineStatus: u8,
    pub BatteryFlag: u8,
    pub BatteryLifePercent: u8,
    pub SystemStatusFlag: u8,
    pub BatteryLifeTime: u32,
    pub BatteryFullLifeTime: u32,
}

#[repr(C)]
#[derive(Default)]
pub struct MEMORYSTATUSEX {
    pub dwLength: u32,
    pub dwMemoryLoad: u32,
    pub ullTotalPhys: u64,
    pub ullAvailPhys: u64,
    pub ullTotalPageFile: u64,
    pub ullAvailPageFile: u64,
    pub ullTotalVirtual: u64,
    pub ullAvailVirtual: u64,
    pub ullAvailExtendedVirtual: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct FILETIME {
    pub dwLowDateTime: u32,
    pub dwHighDateTime: u32,
}

impl FILETIME {
    pub fn as_u64(self) -> u64 {
        ((self.dwHighDateTime as u64) << 32) | self.dwLowDateTime as u64
    }
}

#[repr(C)]
pub struct SP_DEVICE_INTERFACE_DATA {
    pub cbSize: u32,
    pub InterfaceClassGuid: GUID,
    pub Flags: u32,
    pub Reserved: usize,
}

#[repr(C)]
#[derive(Default)]
pub struct HIDD_ATTRIBUTES {
    pub Size: u32,
    pub VendorID: u16,
    pub ProductID: u16,
    pub VersionNumber: u16,
}

#[repr(C)]
#[derive(Default)]
pub struct HIDP_CAPS {
    pub Usage: u16,
    pub UsagePage: u16,
    pub InputReportByteLength: u16,
    pub OutputReportByteLength: u16,
    pub FeatureReportByteLength: u16,
    pub Reserved: [u16; 17],
    pub NumberLinkCollectionNodes: u16,
    pub NumberInputButtonCaps: u16,
    pub NumberInputValueCaps: u16,
    pub NumberInputDataIndices: u16,
    pub NumberOutputButtonCaps: u16,
    pub NumberOutputValueCaps: u16,
    pub NumberOutputDataIndices: u16,
    pub NumberFeatureButtonCaps: u16,
    pub NumberFeatureValueCaps: u16,
    pub NumberFeatureDataIndices: u16,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    pub fn CreateFileW(
        lpFileName: *const u16,
        dwDesiredAccess: u32,
        dwShareMode: u32,
        lpSecurityAttributes: *mut c_void,
        dwCreationDisposition: u32,
        dwFlagsAndAttributes: u32,
        hTemplateFile: HANDLE,
    ) -> HANDLE;
    pub fn DeviceIoControl(
        hDevice: HANDLE,
        dwIoControlCode: u32,
        lpInBuffer: *const c_void,
        nInBufferSize: u32,
        lpOutBuffer: *mut c_void,
        nOutBufferSize: u32,
        lpBytesReturned: *mut u32,
        lpOverlapped: *mut c_void,
    ) -> BOOL;
    pub fn CloseHandle(hObject: HANDLE) -> BOOL;
    pub fn GetLastError() -> u32;
    pub fn GetSystemPowerStatus(lpSystemPowerStatus: *mut SYSTEM_POWER_STATUS) -> BOOL;
    pub fn GlobalMemoryStatusEx(lpBuffer: *mut MEMORYSTATUSEX) -> BOOL;
    pub fn GetSystemTimes(
        lpIdleTime: *mut FILETIME,
        lpKernelTime: *mut FILETIME,
        lpUserTime: *mut FILETIME,
    ) -> BOOL;
    pub fn GetDiskFreeSpaceExW(
        lpDirectoryName: *const u16,
        lpFreeBytesAvailableToCaller: *mut u64,
        lpTotalNumberOfBytes: *mut u64,
        lpTotalNumberOfFreeBytes: *mut u64,
    ) -> BOOL;
    pub fn CreateMutexW(
        lpMutexAttributes: *mut c_void,
        bInitialOwner: BOOL,
        lpName: *const u16,
    ) -> HANDLE;
    pub fn GetModuleHandleW(lpModuleName: *const u16) -> HINSTANCE;
}

#[link(name = "advapi32")]
unsafe extern "system" {
    pub fn RegGetValueW(
        hkey: HKEY,
        lpSubKey: *const u16,
        lpValue: *const u16,
        dwFlags: u32,
        pdwType: *mut u32,
        pvData: *mut c_void,
        pcbData: *mut u32,
    ) -> i32;
    pub fn RegSetKeyValueW(
        hKey: HKEY,
        lpSubKey: *const u16,
        lpValueName: *const u16,
        dwType: u32,
        lpData: *const c_void,
        cbData: u32,
    ) -> i32;
    pub fn RegDeleteKeyValueW(hKey: HKEY, lpSubKey: *const u16, lpValueName: *const u16) -> i32;
}

#[link(name = "user32")]
unsafe extern "system" {
    pub fn RegisterClassExW(lpwcx: *const WNDCLASSEXW) -> u16;
    pub fn CreateWindowExW(
        dwExStyle: u32,
        lpClassName: *const u16,
        lpWindowName: *const u16,
        dwStyle: u32,
        x: i32,
        y: i32,
        nWidth: i32,
        nHeight: i32,
        hWndParent: HWND,
        hMenu: HMENU,
        hInstance: HINSTANCE,
        lpParam: *mut c_void,
    ) -> HWND;
    pub fn DefWindowProcW(hWnd: HWND, Msg: u32, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    pub fn DestroyWindow(hWnd: HWND) -> BOOL;
    pub fn GetMessageW(lpMsg: *mut MSG, hWnd: HWND, wMsgFilterMin: u32, wMsgFilterMax: u32)
    -> BOOL;
    pub fn TranslateMessage(lpMsg: *const MSG) -> BOOL;
    pub fn DispatchMessageW(lpMsg: *const MSG) -> LRESULT;
    pub fn PostMessageW(hWnd: HWND, Msg: u32, wParam: WPARAM, lParam: LPARAM) -> BOOL;
    pub fn PostQuitMessage(nExitCode: i32);
    pub fn FindWindowW(lpClassName: *const u16, lpWindowName: *const u16) -> HWND;
    pub fn RegisterWindowMessageW(lpString: *const u16) -> u32;
    pub fn CreatePopupMenu() -> HMENU;
    pub fn AppendMenuW(hMenu: HMENU, uFlags: u32, uIDNewItem: usize, lpNewItem: *const u16)
    -> BOOL;
    pub fn TrackPopupMenu(
        hMenu: HMENU,
        uFlags: u32,
        x: i32,
        y: i32,
        nReserved: i32,
        hWnd: HWND,
        prcRect: *const c_void,
    ) -> BOOL;
    pub fn DestroyMenu(hMenu: HMENU) -> BOOL;
    pub fn GetCursorPos(lpPoint: *mut POINT) -> BOOL;
    pub fn SetForegroundWindow(hWnd: HWND) -> BOOL;
    pub fn CreateIcon(
        hInstance: HINSTANCE,
        nWidth: i32,
        nHeight: i32,
        cPlanes: u8,
        cBitsPixel: u8,
        lpbANDbits: *const u8,
        lpbXORbits: *const u8,
    ) -> HICON;
    pub fn DestroyIcon(hIcon: HICON) -> BOOL;
}

#[link(name = "shell32")]
unsafe extern "system" {
    pub fn Shell_NotifyIconW(dwMessage: u32, lpData: *const NOTIFYICONDATAW) -> BOOL;
    pub fn ShellExecuteW(
        hwnd: HWND,
        lpOperation: *const u16,
        lpFile: *const u16,
        lpParameters: *const u16,
        lpDirectory: *const u16,
        nShowCmd: i32,
    ) -> HINSTANCE;
}

#[link(name = "setupapi")]
unsafe extern "system" {
    pub fn SetupDiGetClassDevsW(
        ClassGuid: *const GUID,
        Enumerator: *const u16,
        hwndParent: HWND,
        Flags: u32,
    ) -> HDEVINFO;
    pub fn SetupDiEnumDeviceInterfaces(
        DeviceInfoSet: HDEVINFO,
        DeviceInfoData: *const c_void,
        InterfaceClassGuid: *const GUID,
        MemberIndex: u32,
        DeviceInterfaceData: *mut SP_DEVICE_INTERFACE_DATA,
    ) -> BOOL;
    pub fn SetupDiGetDeviceInterfaceDetailW(
        DeviceInfoSet: HDEVINFO,
        DeviceInterfaceData: *const SP_DEVICE_INTERFACE_DATA,
        DeviceInterfaceDetailData: *mut c_void,
        DeviceInterfaceDetailDataSize: u32,
        RequiredSize: *mut u32,
        DeviceInfoData: *mut c_void,
    ) -> BOOL;
    pub fn SetupDiDestroyDeviceInfoList(DeviceInfoSet: HDEVINFO) -> BOOL;
}

#[link(name = "hid")]
unsafe extern "system" {
    pub fn HidD_GetHidGuid(HidGuid: *mut GUID);
    pub fn HidD_GetAttributes(HidDeviceObject: HANDLE, Attributes: *mut HIDD_ATTRIBUTES)
    -> BOOLEAN;
    pub fn HidD_GetPreparsedData(HidDeviceObject: HANDLE, PreparsedData: *mut isize) -> BOOLEAN;
    pub fn HidD_FreePreparsedData(PreparsedData: isize) -> BOOLEAN;
    pub fn HidP_GetCaps(PreparsedData: isize, Capabilities: *mut HIDP_CAPS) -> i32;
    pub fn HidD_SetFeature(
        HidDeviceObject: HANDLE,
        ReportBuffer: *const c_void,
        ReportBufferLength: u32,
    ) -> BOOLEAN;
}

/// Encodes `s` as a NUL-terminated UTF-16 string for the `*W` APIs.
pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Copies `s` into a fixed-size UTF-16 buffer, truncating and NUL-terminating.
pub fn copy_wide<const N: usize>(dst: &mut [u16; N], s: &str) {
    let mut i = 0;
    for unit in s.encode_utf16() {
        if i + 1 >= N {
            break;
        }
        dst[i] = unit;
        i += 1;
    }
    dst[i] = 0;
}

/// Owned Win32 handle that is closed on drop.
pub struct OwnedHandle(pub HANDLE);

// SAFETY: kernel object handles may be used from any thread.
unsafe impl Send for OwnedHandle {}
unsafe impl Sync for OwnedHandle {}

impl OwnedHandle {
    pub fn is_valid(&self) -> bool {
        !self.0.is_null() && self.0 != INVALID_HANDLE_VALUE
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if self.is_valid() {
            // SAFETY: we own the handle and close it exactly once.
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
}

/// Opens a device path (`\\.\ATKACPI`, a HID interface path, ...) with the
/// given access mask. Returns `None` if the device cannot be opened.
pub fn open_device(path: &str, access: u32) -> Option<OwnedHandle> {
    let wpath = wide(path);
    // SAFETY: `wpath` is NUL-terminated and outlives the call.
    let handle = unsafe {
        CreateFileW(
            wpath.as_ptr(),
            access,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null_mut(),
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            std::ptr::null_mut(),
        )
    };
    let owned = OwnedHandle(handle);
    owned.is_valid().then_some(owned)
}

/// Predefined registry roots used by the safe helpers below.
#[derive(Clone, Copy)]
pub enum RegRoot {
    CurrentUser,
    LocalMachine,
}

impl RegRoot {
    fn hkey(self) -> HKEY {
        match self {
            Self::CurrentUser => HKEY_CURRENT_USER,
            Self::LocalMachine => HKEY_LOCAL_MACHINE,
        }
    }
}

/// Reads a `REG_SZ` value. Returns `None` if the key/value does not exist.
pub fn reg_get_string(root: RegRoot, subkey: &str, value: &str) -> Option<String> {
    let wsub = wide(subkey);
    let wval = wide(value);
    let mut size: u32 = 0;
    // SAFETY: first call only queries the required size.
    let status = unsafe {
        RegGetValueW(
            root.hkey(),
            wsub.as_ptr(),
            wval.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut size,
        )
    };
    if status != ERROR_SUCCESS || size == 0 {
        return None;
    }
    let mut buf: Vec<u16> = vec![0; (size as usize).div_ceil(2)];
    let mut size_bytes = (buf.len() * 2) as u32;
    // SAFETY: `buf` is large enough for `size_bytes` bytes.
    let status = unsafe {
        RegGetValueW(
            root.hkey(),
            wsub.as_ptr(),
            wval.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            buf.as_mut_ptr().cast(),
            &mut size_bytes,
        )
    };
    if status != ERROR_SUCCESS {
        return None;
    }
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    Some(String::from_utf16_lossy(&buf[..len]).trim().to_string())
}

/// Reads a `REG_DWORD` value.
pub fn reg_get_dword(root: RegRoot, subkey: &str, value: &str) -> Option<u32> {
    let wsub = wide(subkey);
    let wval = wide(value);
    let mut data: u32 = 0;
    let mut size = std::mem::size_of::<u32>() as u32;
    // SAFETY: `data` is a valid 4-byte buffer.
    let status = unsafe {
        RegGetValueW(
            root.hkey(),
            wsub.as_ptr(),
            wval.as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            (&mut data as *mut u32).cast(),
            &mut size,
        )
    };
    (status == ERROR_SUCCESS).then_some(data)
}

/// Writes a `REG_SZ` value, creating the key if necessary.
pub fn reg_set_string(root: RegRoot, subkey: &str, value: &str, data: &str) -> bool {
    let wsub = wide(subkey);
    let wval = wide(value);
    let wdata = wide(data);
    // SAFETY: all buffers are NUL-terminated and outlive the call.
    let status = unsafe {
        RegSetKeyValueW(
            root.hkey(),
            wsub.as_ptr(),
            wval.as_ptr(),
            REG_SZ,
            wdata.as_ptr().cast(),
            (wdata.len() * 2) as u32,
        )
    };
    status == ERROR_SUCCESS
}

/// Deletes a registry value. Missing values are not treated as an error.
pub fn reg_delete_value(root: RegRoot, subkey: &str, value: &str) {
    let wsub = wide(subkey);
    let wval = wide(value);
    // SAFETY: buffers are NUL-terminated.
    unsafe {
        RegDeleteKeyValueW(root.hkey(), wsub.as_ptr(), wval.as_ptr());
    }
}

/// Opens `target` (URL or file) with the default handler, like `xdg-open`.
pub fn shell_open(target: &str) {
    let op = wide("open");
    let file = wide(target);
    // SAFETY: buffers are NUL-terminated and outlive the call.
    unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            op.as_ptr(),
            file.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        );
    }
}
