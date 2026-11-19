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

//! Platform-specific glue. Linux keeps using the existing services
//! (asusd, supergfxctl, evdev, KDE tools); Windows backends live in
//! [`windows`].

#[cfg(windows)]
pub mod windows;

/// Translation keys whose text names Linux-only tools, paired with the
/// Windows variant that talks about the ASUS driver instead.
const WINDOWS_KEY_OVERRIDES: &[(&str, &str)] = &[
    ("asusd_missing_warning", "asusd_missing_warning_win"),
    ("battery_group_desc", "battery_group_desc_win"),
    ("fan_group_desc", "fan_group_desc_win"),
    ("gpu_group_desc", "gpu_group_desc_win"),
    (
        "supergfxctl_missing_warning",
        "supergfxctl_missing_warning_win",
    ),
    ("apu_mem_group_desc", "apu_mem_group_desc_win"),
    ("aura_group_desc", "aura_group_desc_win"),
    (
        "aura_daemon_missing_warning",
        "aura_daemon_missing_warning_win",
    ),
    ("aura_no_devices_info", "aura_no_devices_info_win"),
    ("animatrix_group_desc", "animatrix_group_desc_win"),
    (
        "animatrix_daemon_missing_warning",
        "animatrix_daemon_missing_warning_win",
    ),
    (
        "fn_key_grubby_missing_warning",
        "fn_key_grubby_missing_warning_win",
    ),
    ("fn_key_hint_subtitle", "fn_key_hint_subtitle_win"),
    ("fn_key_saved", "fn_key_saved_win"),
    ("home_kernel_title", "home_kernel_title_win"),
];

/// Returns the platform-appropriate translation key for `key`.
///
/// Usage: `t!(crate::platform::pk("fan_group_desc"))`.
pub fn pk(key: &'static str) -> &'static str {
    if cfg!(windows)
        && let Some((_, win)) = WINDOWS_KEY_OVERRIDES.iter().find(|(k, _)| *k == key)
    {
        return win;
    }
    key
}

/// Opens a URL with the desktop's default handler.
pub fn open_url(url: &str) {
    #[cfg(windows)]
    {
        windows::ffi::shell_open(url);
    }
    #[cfg(not(windows))]
    {
        use std::os::unix::process::CommandExt;
        let _ = std::process::Command::new("xdg-open")
            .arg(url)
            .process_group(0)
            .spawn();
    }
}
