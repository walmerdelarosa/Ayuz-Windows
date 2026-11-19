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

//! "Launch on system startup" through the per-user `Run` registry key.

use super::ffi;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "Ayuz";

pub fn is_enabled() -> bool {
    ffi::reg_get_string(ffi::RegRoot::CurrentUser, RUN_KEY, VALUE_NAME).is_some()
}

pub fn set_enabled(enable: bool) {
    if !enable {
        ffi::reg_delete_value(ffi::RegRoot::CurrentUser, RUN_KEY, VALUE_NAME);
        return;
    }
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let command = format!("\"{}\" --hidden", exe.display());
    if !ffi::reg_set_string(ffi::RegRoot::CurrentUser, RUN_KEY, VALUE_NAME, &command) {
        tracing::warn!("autostart: cannot write HKCU\\{RUN_KEY}\\{VALUE_NAME}");
    }
}
