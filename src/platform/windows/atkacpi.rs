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

//! Access to the ASUS ACPI/WMI interface through the `\\.\ATKACPI` device.
//!
//! The device is created by the "ASUS System Control Interface" driver that
//! ships with MyASUS / Armoury Crate (and can be installed on its own). Every
//! call is a `DeviceIoControl` with a buffer of the form
//! `[method_id: u32][args_len: u32][args...]`, where the method is `DSTS`
//! (read a device status) or `DEVS` (set a device status). Device IDs are the
//! same ones the Linux `asus-wmi` kernel driver uses.

use std::sync::{Mutex, OnceLock};

use super::ffi::{self, OwnedHandle};

const DEVICE_PATH: &str = r"\\.\ATKACPI";
const IOCTL_ATK_CALL: u32 = 0x0022_240C;

/// `"DSTS"` as little-endian u32.
const METHOD_DSTS: u32 = 0x5354_5344;
/// `"DEVS"` as little-endian u32.
const METHOD_DEVS: u32 = 0x5356_4544;

/// Bit set in every DSTS answer for a device that exists on this machine.
const DSTS_PRESENCE_BIT: u32 = 0x0001_0000;
/// Returned by the firmware for unknown device IDs.
const UNSUPPORTED: u32 = 0xFFFF_FFFE;

// ── Device IDs (see linux/include/linux/platform_data/x86/asus-wmi.h) ───────

pub const DEVID_THROTTLE_THERMAL_POLICY: u32 = 0x0012_0075;
pub const DEVID_THROTTLE_THERMAL_POLICY_VIVO: u32 = 0x0011_0019;
pub const DEVID_BATTERY_CHARGE_LIMIT: u32 = 0x0012_0057;
pub const DEVID_FNLOCK: u32 = 0x0010_0023;
pub const DEVID_DGPU_DISABLE: u32 = 0x0009_0020;
pub const DEVID_GPU_MUX: u32 = 0x0009_0016;
pub const DEVID_GPU_MUX_VIVO: u32 = 0x0009_0026;
pub const DEVID_KBD_BACKLIGHT: u32 = 0x0005_0021;
pub const DEVID_TUF_RGB_MODE: u32 = 0x0010_0056;
pub const DEVID_TUF_RGB_MODE2: u32 = 0x0010_005A;
pub const DEVID_CPU_TEMP: u32 = 0x0012_0094;

struct Device {
    handle: OwnedHandle,
}

static DEVICE: OnceLock<Option<Mutex<Device>>> = OnceLock::new();

fn device() -> Option<&'static Mutex<Device>> {
    DEVICE
        .get_or_init(|| {
            let handle = ffi::open_device(DEVICE_PATH, ffi::GENERIC_READ | ffi::GENERIC_WRITE)?;
            Some(Mutex::new(Device { handle }))
        })
        .as_ref()
}

/// `true` if the ATKACPI driver is installed and could be opened.
pub fn is_available() -> bool {
    device().is_some()
}

/// Performs one raw ATK method call and returns the first output DWORD.
fn call(method: u32, args: &[u8]) -> Option<u32> {
    let dev = device()?;
    let guard = dev.lock().ok()?;

    let mut input = Vec::with_capacity(8 + args.len());
    input.extend_from_slice(&method.to_le_bytes());
    input.extend_from_slice(&(args.len() as u32).to_le_bytes());
    input.extend_from_slice(args);

    let mut output = [0u8; 16];
    let mut returned: u32 = 0;
    // SAFETY: the handle is valid for the process lifetime; buffers are
    // correctly sized and outlive the synchronous call.
    let ok = unsafe {
        ffi::DeviceIoControl(
            guard.handle.0,
            IOCTL_ATK_CALL,
            input.as_ptr().cast(),
            input.len() as u32,
            output.as_mut_ptr().cast(),
            output.len() as u32,
            &mut returned,
            std::ptr::null_mut(),
        )
    };
    if ok == 0 {
        tracing::debug!(
            "ATKACPI call 0x{method:08X} failed (error {})",
            // SAFETY: trivially safe FFI call.
            unsafe { ffi::GetLastError() }
        );
        return None;
    }
    Some(u32::from_le_bytes([
        output[0], output[1], output[2], output[3],
    ]))
}

/// Raw `DSTS` value of a device, or `None` if the call itself failed.
pub fn dsts_raw(device_id: u32) -> Option<u32> {
    call(METHOD_DSTS, &device_id.to_le_bytes())
}

/// `true` if the firmware reports `device_id` as present.
pub fn is_present(device_id: u32) -> bool {
    dsts_raw(device_id).is_some_and(|v| v != UNSUPPORTED && v & DSTS_PRESENCE_BIT != 0)
}

/// Value (lower 16 bits) of a present device; `None` if absent.
pub fn get(device_id: u32) -> Option<u32> {
    let raw = dsts_raw(device_id)?;
    if raw == UNSUPPORTED || raw & DSTS_PRESENCE_BIT == 0 {
        return None;
    }
    Some(raw & 0xFFFF)
}

/// Sets a device to `value` with `DEVS` and returns the firmware result
/// (usually `1` on success).
pub fn set(device_id: u32, value: u32) -> Option<u32> {
    let mut args = [0u8; 8];
    args[..4].copy_from_slice(&device_id.to_le_bytes());
    args[4..].copy_from_slice(&value.to_le_bytes());
    call(METHOD_DEVS, &args)
}

/// `DEVS` with arbitrary payload after the device ID (used by TUF RGB).
pub fn set_bytes(device_id: u32, payload: &[u8]) -> Option<u32> {
    let mut args = Vec::with_capacity(4 + payload.len());
    args.extend_from_slice(&device_id.to_le_bytes());
    args.extend_from_slice(payload);
    call(METHOD_DEVS, &args)
}
