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

pub mod commands;
pub mod config;
pub mod dbus;
pub mod dbus_animatrix;
pub mod kde_brightness;
pub mod migration;
pub mod touchpad_ctl;

// evdev / uinput / i2c based listeners exist only on Linux. On Windows the
// same module paths resolve to inert stand-ins with identical public APIs,
// so the UI components compile unchanged and simply report "unavailable".
#[cfg(not(windows))]
pub mod edge_gestures;
#[cfg(not(windows))]
pub(crate) mod evdev_runner;
#[cfg(not(windows))]
pub mod fan_hotkey;
#[cfg(not(windows))]
pub mod numberpad;
#[cfg(not(windows))]
pub mod numberpad_layouts;
#[cfg(not(windows))]
pub mod numberpad_pointer;
#[cfg(not(windows))]
pub mod typing_watch;

#[cfg(windows)]
#[path = "unsupported/edge_gestures.rs"]
pub mod edge_gestures;
#[cfg(windows)]
#[path = "unsupported/fan_hotkey.rs"]
pub mod fan_hotkey;
#[cfg(windows)]
#[path = "unsupported/numberpad.rs"]
pub mod numberpad;
#[cfg(windows)]
#[path = "unsupported/typing_watch.rs"]
pub mod typing_watch;
