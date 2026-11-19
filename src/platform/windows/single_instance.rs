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

//! One Ayuz per user session. A second launch forwards a "show window"
//! request to the running instance through its tray window and exits.

use super::ffi;

const MUTEX_NAME: &str = r"Local\Ayuz.SingleInstance";

/// Returns `true` if this process is the first instance. The mutex handle is
/// intentionally leaked so it lives until the process exits.
pub fn acquire() -> bool {
    let name = ffi::wide(MUTEX_NAME);
    // SAFETY: NUL-terminated name; null security attributes are allowed.
    unsafe {
        let handle = ffi::CreateMutexW(std::ptr::null_mut(), 0, name.as_ptr());
        if handle.is_null() {
            // Could not create the mutex at all: do not block startup.
            return true;
        }
        ffi::GetLastError() != ffi::ERROR_ALREADY_EXISTS
    }
}
