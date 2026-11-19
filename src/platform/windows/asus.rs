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

//! Windows implementation of the hardware API that `services::dbus` exposes
//! on Linux (where it talks to `asusd` / `supergfxctl`). Everything here goes
//! through the ATKACPI driver; see [`super::atkacpi`].

use std::sync::atomic::{AtomicBool, Ordering};

use rust_i18n::t;

use super::atkacpi::{self as atk, *};
use super::state;
use crate::services::dbus::{FanProfile, GfxMode};

/// Error text for a firmware call that returned an unexpected status.
fn firmware_error(result: Option<u32>) -> String {
    match result {
        Some(code) => t!("win_error_atk_result", code = format!("0x{code:X}")).to_string(),
        None => t!("win_error_atk_missing").to_string(),
    }
}

// ── Platform availability (fan + battery) ────────────────────────────────────

fn thermal_policy_device() -> Option<(u32, bool)> {
    if atk::is_present(DEVID_THROTTLE_THERMAL_POLICY) {
        Some((DEVID_THROTTLE_THERMAL_POLICY, false))
    } else if atk::is_present(DEVID_THROTTLE_THERMAL_POLICY_VIVO) {
        Some((DEVID_THROTTLE_THERMAL_POLICY_VIVO, true))
    } else {
        None
    }
}

/// Counterpart of the Linux "is asusd reachable" probe: the ATKACPI driver is
/// installed and exposes either the thermal policy or the charge limit.
pub async fn check_asusd_available() -> bool {
    atk::is_available()
        && (thermal_policy_device().is_some() || atk::is_present(DEVID_BATTERY_CHARGE_LIMIT))
}

// ── Battery charge limit ─────────────────────────────────────────────────────

const STATE_CHARGE_LIMIT: &str = "charge_limit";
static CHARGE_LIMIT_REAPPLIED: AtomicBool = AtomicBool::new(false);

/// The firmware does not report the active limit and forgets it on reboot,
/// so the last value set by Ayuz is stored and re-applied once per session.
pub async fn get_charge_limit() -> Result<u8, String> {
    let value = state::get_u32(STATE_CHARGE_LIMIT)
        .map(|v| v.clamp(20, 100) as u8)
        .unwrap_or(100);
    if value < 100 && !CHARGE_LIMIT_REAPPLIED.swap(true, Ordering::SeqCst) {
        let result = atk::set(DEVID_BATTERY_CHARGE_LIMIT, value as u32);
        if result != Some(1) {
            tracing::warn!(
                "re-applying charge limit {value}%: {}",
                firmware_error(result)
            );
        }
    }
    Ok(value)
}

pub async fn set_charge_limit(value: u8) -> Result<u8, String> {
    let result = atk::set(DEVID_BATTERY_CHARGE_LIMIT, value as u32);
    if result != Some(1) {
        return Err(t!("error_charge_limit_write", error = firmware_error(result)).to_string());
    }
    state::set_u32(STATE_CHARGE_LIMIT, value as u32);
    CHARGE_LIMIT_REAPPLIED.store(true, Ordering::SeqCst);
    Ok(value)
}

// ── Fan / thermal policy ─────────────────────────────────────────────────────

const STATE_FAN_PROFILE: &str = "fan_profile";

/// Firmware value for a profile. Vivobook/Zenbook firmware swaps the
/// meaning of 1 and 2 (`ASUS_THROTTLE_THERMAL_POLICY_*_VIVO` in the kernel).
fn profile_to_firmware(profile: FanProfile, vivo: bool) -> u32 {
    match (profile, vivo) {
        (FanProfile::Balanced, _) => 0,
        (FanProfile::Performance, false) => 1,
        (FanProfile::Performance, true) => 2,
        (FanProfile::Quiet | FanProfile::LowPower, false) => 2,
        (FanProfile::Quiet | FanProfile::LowPower, true) => 1,
    }
}

fn profile_from_firmware(value: u32, vivo: bool) -> Option<FanProfile> {
    match (value, vivo) {
        (0, _) => Some(FanProfile::Balanced),
        (1, false) | (2, true) => Some(FanProfile::Performance),
        (2, false) | (1, true) => Some(FanProfile::Quiet),
        _ => None,
    }
}

pub async fn get_fan_profile() -> Result<FanProfile, String> {
    let Some((dev, vivo)) = thermal_policy_device() else {
        return Err(t!("error_fan_profile_read", error = firmware_error(None)).to_string());
    };
    // Not every firmware reports the active policy through DSTS; fall back to
    // the last profile Ayuz applied.
    if let Some(p) = atk::get(dev).and_then(|v| profile_from_firmware(v & 0xFF, vivo)) {
        return Ok(p);
    }
    Ok(state::get_u32(STATE_FAN_PROFILE)
        .map(FanProfile::from)
        .unwrap_or(FanProfile::Balanced))
}

pub async fn set_fan_profile(profile: FanProfile) -> Result<FanProfile, String> {
    let Some((dev, vivo)) = thermal_policy_device() else {
        return Err(t!("error_fan_profile_write", error = firmware_error(None)).to_string());
    };
    let result = atk::set(dev, profile_to_firmware(profile, vivo));
    if result != Some(1) {
        return Err(t!("error_fan_profile_write", error = firmware_error(result)).to_string());
    }
    state::set_u32(STATE_FAN_PROFILE, profile as u32);
    Ok(profile)
}

// ── GPU mode (Eco / Standard / Ultimate) ─────────────────────────────────────

fn mux_device() -> Option<u32> {
    [DEVID_GPU_MUX, DEVID_GPU_MUX_VIVO]
        .into_iter()
        .find(|&d| atk::is_present(d))
}

pub async fn check_supergfxctl_available() -> bool {
    atk::is_available() && (atk::is_present(DEVID_DGPU_DISABLE) || mux_device().is_some())
}

fn current_gpu_mode() -> Option<GfxMode> {
    if let Some(mux) = mux_device() {
        // MUX = 0 routes the internal panel to the dGPU ("Ultimate").
        if atk::get(mux) == Some(0) {
            return Some(GfxMode::AsusMuxDiscreet);
        }
    }
    match atk::get(DEVID_DGPU_DISABLE) {
        Some(1) => Some(GfxMode::Integrated),
        Some(_) => Some(GfxMode::Hybrid),
        None if mux_device().is_some() => Some(GfxMode::Hybrid),
        None => None,
    }
}

pub async fn get_gpu_mode() -> Result<GfxMode, String> {
    current_gpu_mode()
        .ok_or_else(|| t!("error_gpu_mode_read", error = firmware_error(None)).to_string())
}

pub async fn get_supported_gpu_modes() -> Result<Vec<GfxMode>, String> {
    let mut modes = vec![GfxMode::Hybrid];
    if atk::is_present(DEVID_DGPU_DISABLE) {
        modes.push(GfxMode::Integrated);
    }
    if mux_device().is_some() {
        modes.push(GfxMode::AsusMuxDiscreet);
    }
    Ok(modes)
}

/// Firmware results above 1 signal a rejected GPU/MUX change.
fn check_gpu_result(result: Option<u32>) -> Result<(), String> {
    match result {
        Some(0 | 1) => Ok(()),
        other => Err(t!("error_gpu_mode_write", error = firmware_error(other)).to_string()),
    }
}

/// Switches between Standard (Hybrid), Eco (Integrated) and Ultimate (MUX).
///
/// MUX changes only take effect after a reboot. Leaving Ultimate flips the
/// MUX back first; selecting Eco again after the reboot disables the dGPU.
pub async fn set_gpu_mode(mode: GfxMode) -> Result<GfxMode, String> {
    let current = current_gpu_mode();
    if current == Some(mode) {
        return Ok(mode);
    }
    match mode {
        GfxMode::AsusMuxDiscreet => {
            let mux = mux_device().ok_or_else(|| {
                t!("error_gpu_mode_write", error = firmware_error(None)).to_string()
            })?;
            if current == Some(GfxMode::Integrated) {
                check_gpu_result(atk::set(DEVID_DGPU_DISABLE, 0))?;
            }
            check_gpu_result(atk::set(mux, 0))?;
        }
        GfxMode::Hybrid | GfxMode::Integrated => {
            if current == Some(GfxMode::AsusMuxDiscreet) {
                if let Some(mux) = mux_device() {
                    check_gpu_result(atk::set(mux, 1))?;
                }
                return Ok(mode);
            }
            let disable = u32::from(mode == GfxMode::Integrated);
            check_gpu_result(atk::set(DEVID_DGPU_DISABLE, disable))?;
        }
        _ => {
            return Err(t!(
                "error_gpu_mode_write",
                error = t!("win_error_unsupported").to_string()
            )
            .to_string());
        }
    }
    Ok(mode)
}

// ── APU memory: not exposed through ATKACPI in a documented way ─────────────

pub async fn get_apu_mem() -> Result<i32, String> {
    Err(t!("win_error_unsupported").to_string())
}

pub async fn set_apu_mem(_value: i32) -> Result<i32, String> {
    Err(t!("win_error_unsupported").to_string())
}

pub async fn get_apu_mem_options() -> Result<Vec<i32>, String> {
    Err(t!("win_error_unsupported").to_string())
}

// ── FN lock ──────────────────────────────────────────────────────────────────

pub fn fn_lock_available() -> bool {
    atk::is_available() && atk::is_present(DEVID_FNLOCK)
}

/// `locked` = F1-F12 trigger the printed shortcut functions (firmware
/// FN-lock off), matching `asus_wmi.fnlock_default=0` on Linux.
pub fn set_fn_lock(locked: bool) -> Result<(), String> {
    let value = if locked { 0 } else { 1 };
    match atk::set(DEVID_FNLOCK, value) {
        Some(_) => Ok(()),
        None => Err(firmware_error(None)),
    }
}

// ── Sensors used by the home page ────────────────────────────────────────────

/// CPU temperature in °C as reported by the embedded controller.
pub fn cpu_temperature() -> Option<u32> {
    atk::get(DEVID_CPU_TEMP).filter(|&t| t > 0 && t < 120)
}
