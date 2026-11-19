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

//! Windows stand-in for the i2c/uinput NumberPad driver. On Windows the
//! NumberPad is handled by the touchpad's own (ASUS Precision Touchpad) driver.

use tokio::sync::{mpsc, watch};

/// Same variants as the Linux implementation so the UI code is shared.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum NumberpadStatus {
    Ok,
    NoHardware,
    I2cUnavailable(String),
    PermissionDenied { device: String },
}

pub async fn probe() -> NumberpadStatus {
    NumberpadStatus::NoHardware
}

pub async fn run_loop(
    _shutdown: watch::Receiver<bool>,
    _active_rx: watch::Receiver<bool>,
    _feedback_tx: mpsc::UnboundedSender<bool>,
) {
}
