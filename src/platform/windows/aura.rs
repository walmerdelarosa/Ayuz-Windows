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

//! Aura keyboard lighting on Windows.
//!
//! Two transports are supported:
//! - **HID** (`hid:<device path>`): ROG/Strix/Zephyrus/Vivobook keyboards that
//!   expose the ASUS vendor collection (usage page `0xFF31`, report `0x5D`).
//!   The packet layout is the same one `rog-aura` uses on Linux.
//! - **ACPI** (`acpi:tuf`): TUF keyboards driven through ATKACPI
//!   (`DEVID_TUF_RGB_MODE[2]`), as in the kernel's `kbd_rgb_mode` attribute.
//!
//! Neither transport can be read back, so the last applied state is cached
//! per device and served to the UI.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Mutex;

use rust_i18n::t;

use super::atkacpi::{self as atk, DEVID_KBD_BACKLIGHT, DEVID_TUF_RGB_MODE, DEVID_TUF_RGB_MODE2};
use super::hid::{self, HidDevice, HidInfo};
use crate::services::dbus::{AuraDeviceInfo, AuraDeviceKind, AuraEffect, Colour, LaptopAuraPower};

const AURA_REPORT_ID: u8 = 0x5D;
const TUF_PATH: &str = "acpi:tuf";
const HID_PREFIX: &str = "hid:";

/// Modes every supported keyboard accepts: Static, Breathe, Colour cycle,
/// Rainbow wave and Pulse/Strobe.
const BASIC_MODES: &[u32] = &[0, 1, 2, 3, 10];

#[derive(Clone)]
struct CachedState {
    effect: AuraEffect,
    brightness: u32,
}

impl Default for CachedState {
    fn default() -> Self {
        Self {
            effect: AuraEffect {
                mode: 0,
                zone: 0,
                colour1: Colour { r: 166, g: 0, b: 0 },
                colour2: Colour::BLACK,
                speed: "Med".to_string(),
                direction: "Right".to_string(),
            },
            brightness: 2,
        }
    }
}

static CACHE: Mutex<Option<HashMap<String, CachedState>>> = Mutex::new(None);
static INITIALISED: Mutex<Option<HashSet<String>>> = Mutex::new(None);

fn cached(path: &str) -> CachedState {
    let mut guard = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    guard
        .get_or_insert_with(HashMap::new)
        .entry(path.to_string())
        .or_default()
        .clone()
}

fn update_cache(path: &str, f: impl FnOnce(&mut CachedState)) {
    let mut guard = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    f(guard
        .get_or_insert_with(HashMap::new)
        .entry(path.to_string())
        .or_default());
}

fn write_error(detail: impl Into<String>) -> String {
    t!("error_aura_write", error = detail.into()).to_string()
}

// ── Packet helpers (shared layout with rog-aura) ─────────────────────────────

fn speed_byte(speed: &str) -> u8 {
    match speed {
        "Low" => 0xE1,
        "High" => 0xF5,
        _ => 0xEB,
    }
}

fn direction_byte(direction: &str) -> u8 {
    match direction {
        "Left" => 1,
        "Up" => 2,
        "Down" => 3,
        _ => 0,
    }
}

fn hid_effect_packet(effect: &AuraEffect) -> [u8; 17] {
    let mut msg = [0u8; 17];
    msg[0] = AURA_REPORT_ID;
    msg[1] = 0xB3;
    msg[2] = effect.zone as u8;
    msg[3] = effect.mode as u8;
    msg[4] = effect.colour1.r;
    msg[5] = effect.colour1.g;
    msg[6] = effect.colour1.b;
    msg[7] = speed_byte(&effect.speed);
    msg[8] = direction_byte(&effect.direction);
    // Breathe with a second colour alternates between both colours.
    msg[9] = u8::from(effect.mode == 1);
    msg[10] = effect.colour2.r;
    msg[11] = effect.colour2.g;
    msg[12] = effect.colour2.b;
    msg
}

const HID_SET: [u8; 5] = [AURA_REPORT_ID, 0xB5, 0, 0, 0];
const HID_APPLY: [u8; 2] = [AURA_REPORT_ID, 0xB4];

fn hid_brightness_packet(level: u32) -> [u8; 5] {
    [AURA_REPORT_ID, 0xBA, 0xC5, 0xC4, level.min(3) as u8]
}

// ── HID transport ────────────────────────────────────────────────────────────

fn hid_candidates() -> Vec<HidInfo> {
    hid::enumerate(hid::ASUS_VENDOR_ID)
        .into_iter()
        .filter(|d| d.usage_page == hid::ASUS_USAGE_PAGE && d.feature_report_len > 0)
        .collect()
}

fn hid_info_for(path: &str) -> Option<HidInfo> {
    let raw = path.strip_prefix(HID_PREFIX)?;
    hid_candidates().into_iter().find(|d| d.path == raw)
}

/// Sends the vendor "hello" sequence the keyboard expects before accepting
/// lighting commands. Returns `false` if the collection rejects report 0x5D.
fn hid_init(dev: &HidDevice) -> bool {
    let mut ok = dev.send_feature(&[AURA_REPORT_ID, 0xB9]);
    let mut hello = vec![AURA_REPORT_ID];
    hello.extend_from_slice(b"ASUS Tech.Inc.");
    ok &= dev.send_feature(&hello);
    ok &= dev.send_feature(&[AURA_REPORT_ID, 0x05, 0x20, 0x31, 0x00, 0x1A]);
    ok
}

fn with_hid_device<T>(
    path: &str,
    f: impl FnOnce(&HidDevice) -> Result<T, String>,
) -> Result<T, String> {
    let info = hid_info_for(path).ok_or_else(|| write_error(t!("win_error_aura_device_gone")))?;
    let dev =
        HidDevice::open(&info).ok_or_else(|| write_error(t!("win_error_aura_device_gone")))?;

    let needs_init = {
        let mut guard = INITIALISED.lock().unwrap_or_else(|e| e.into_inner());
        guard
            .get_or_insert_with(HashSet::new)
            .insert(path.to_string())
    };
    if needs_init {
        hid_init(&dev);
    }
    f(&dev)
}

fn hid_send_all(dev: &HidDevice, packets: &[&[u8]]) -> Result<(), String> {
    for p in packets {
        if !dev.send_feature(p) {
            return Err(write_error(t!("win_error_hid_write")));
        }
    }
    Ok(())
}

// ── ACPI (TUF) transport ─────────────────────────────────────────────────────

fn tuf_rgb_device() -> Option<u32> {
    [DEVID_TUF_RGB_MODE, DEVID_TUF_RGB_MODE2]
        .into_iter()
        .find(|&d| atk::is_present(d))
}

fn tuf_apply_effect(effect: &AuraEffect) -> Result<(), String> {
    let dev = tuf_rgb_device().ok_or_else(|| write_error(t!("win_error_atk_missing")))?;
    // The firmware only knows modes 0-8 and 10; anything else becomes Pulse.
    let mode = match effect.mode {
        m if m >= 12 || m == 9 => 10,
        m => m,
    };
    // 0xB4 = apply and keep after reboot (0xB3 would only apply).
    let payload = [
        0xB4,
        mode as u8,
        effect.colour1.r,
        effect.colour1.g,
        effect.colour1.b,
        speed_byte(&effect.speed),
    ];
    match atk::set_bytes(dev, &payload) {
        Some(_) => Ok(()),
        None => Err(write_error(t!("win_error_atk_missing"))),
    }
}

fn tuf_apply_brightness(level: u32) -> Result<(), String> {
    match atk::set(DEVID_KBD_BACKLIGHT, 0x80 | level.min(3)) {
        Some(_) => Ok(()),
        None => Err(write_error(t!("win_error_atk_missing"))),
    }
}

// ── Public API mirroring services::dbus ──────────────────────────────────────

/// On Windows there is no daemon; report "running" when either transport is
/// usable so the Aura page shows devices instead of the daemon warning.
pub async fn is_asusd_running() -> bool {
    atk::is_available()
        || tokio::task::spawn_blocking(|| !hid_candidates().is_empty())
            .await
            .unwrap_or(false)
}

pub async fn discover_aura_devices() -> Result<Vec<AuraDeviceInfo>, String> {
    let hid_paths = tokio::task::spawn_blocking(|| {
        // Several vendor collections can belong to one keyboard; keep the
        // first one per product that actually accepts the Aura report.
        let mut seen_products = HashSet::new();
        let mut paths = Vec::new();
        for info in hid_candidates() {
            if seen_products.contains(&info.product_id) {
                continue;
            }
            let Some(dev) = HidDevice::open(&info) else {
                continue;
            };
            if hid_init(&dev) {
                seen_products.insert(info.product_id);
                let path = format!("{HID_PREFIX}{}", info.path);
                INITIALISED
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .get_or_insert_with(HashSet::new)
                    .insert(path.clone());
                paths.push(path);
            }
        }
        paths
    })
    .await
    .unwrap_or_default();

    let mut devices: Vec<AuraDeviceInfo> = hid_paths
        .into_iter()
        .map(|path| AuraDeviceInfo {
            path,
            kind: AuraDeviceKind::Keyboard,
        })
        .collect();

    if devices.is_empty() && tuf_rgb_device().is_some() {
        devices.push(AuraDeviceInfo {
            path: TUF_PATH.to_string(),
            kind: AuraDeviceKind::KeyboardTuf,
        });
    }
    Ok(devices)
}

pub async fn get_aura_brightness(path: &str) -> Result<u32, String> {
    Ok(cached(path).brightness)
}

pub async fn set_aura_brightness(path: &str, value: u32) -> Result<u32, String> {
    let value = value.min(3);
    if path == TUF_PATH {
        tuf_apply_brightness(value)?;
    } else {
        let packet = hid_brightness_packet(value);
        let p = path.to_string();
        tokio::task::spawn_blocking(move || {
            with_hid_device(&p, |dev| hid_send_all(dev, &[&packet]))
        })
        .await
        .map_err(|e| write_error(e.to_string()))??;
    }
    update_cache(path, |s| s.brightness = value);
    Ok(value)
}

pub async fn get_aura_effect(path: &str) -> Result<AuraEffect, String> {
    Ok(cached(path).effect)
}

pub async fn set_aura_effect(path: &str, effect: AuraEffect) -> Result<(), String> {
    if path == TUF_PATH {
        tuf_apply_effect(&effect)?;
    } else {
        let packet = hid_effect_packet(&effect);
        let p = path.to_string();
        tokio::task::spawn_blocking(move || {
            with_hid_device(&p, |dev| {
                hid_send_all(dev, &[&packet, &HID_SET, &HID_APPLY])
            })
        })
        .await
        .map_err(|e| write_error(e.to_string()))??;
    }
    update_cache(path, |s| s.effect = effect);
    Ok(())
}

pub async fn get_aura_supported_modes(_path: &str) -> Result<Vec<u32>, String> {
    Ok(BASIC_MODES.to_vec())
}

pub async fn get_aura_supported_zones(_path: &str) -> Result<Vec<u32>, String> {
    Ok(Vec::new())
}

pub async fn get_aura_supported_power_zones(_path: &str) -> Result<Vec<u32>, String> {
    Ok(Vec::new())
}

pub async fn get_aura_supported_brightness(_path: &str) -> Result<Vec<u32>, String> {
    Ok(vec![0, 1, 2, 3])
}

pub async fn get_aura_all_mode_data(path: &str) -> Result<BTreeMap<u32, AuraEffect>, String> {
    let current = cached(path).effect;
    Ok(BASIC_MODES
        .iter()
        .map(|&mode| {
            let mut e = current.clone();
            e.mode = mode;
            (mode, e)
        })
        .collect())
}

pub async fn get_aura_led_power(_path: &str) -> Result<LaptopAuraPower, String> {
    Ok(LaptopAuraPower { states: Vec::new() })
}

pub async fn set_aura_led_power(_path: &str, _power: LaptopAuraPower) -> Result<(), String> {
    Err(write_error(t!("win_error_unsupported")))
}
