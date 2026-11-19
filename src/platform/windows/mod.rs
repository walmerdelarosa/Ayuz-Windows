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

//! Windows backends. Hardware access goes through the ASUS ATKACPI driver
//! (fan profile, charge limit, GPU mode, FN lock, TUF lighting) and the
//! keyboard's vendor HID collection (Aura RGB).

pub mod asus;
pub mod atkacpi;
pub mod aura;
pub mod autostart;
pub mod ffi;
pub mod hid;
pub mod single_instance;
pub mod state;
pub mod sysinfo;
pub mod tray;
