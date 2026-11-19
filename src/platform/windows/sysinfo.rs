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

//! Device information and live metrics for the home page.

use std::sync::Mutex;

use super::ffi;

const BIOS_KEY: &str = r"HARDWARE\DESCRIPTION\System\BIOS";
const OS_KEY: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";

pub struct DeviceInfo {
    pub product_name: String,
    pub board_name: String,
    pub bios_version: String,
    pub bios_date: String,
    pub os_version: String,
}

fn bios(value: &str) -> String {
    ffi::reg_get_string(ffi::RegRoot::LocalMachine, BIOS_KEY, value).unwrap_or_default()
}

/// Board name as reported by SMBIOS (e.g. `UM5606WA`), used for model detection.
pub fn board_name() -> String {
    bios("BaseBoardProduct")
}

/// "Windows 11 Pro 24H2 (26100.2314)".
pub fn os_version() -> String {
    let get = |v: &str| ffi::reg_get_string(ffi::RegRoot::LocalMachine, OS_KEY, v);
    let build = get("CurrentBuild").unwrap_or_default();
    let mut product = get("ProductName").unwrap_or_else(|| "Windows".to_string());
    // ProductName still says "Windows 10" on Windows 11 (build 22000+).
    if build.parse::<u32>().is_ok_and(|b| b >= 22000) {
        product = product.replace("Windows 10", "Windows 11");
    }
    let display = get("DisplayVersion").unwrap_or_default();
    let ubr = ffi::reg_get_dword(ffi::RegRoot::LocalMachine, OS_KEY, "UBR");
    let build_full = match ubr {
        Some(u) if !build.is_empty() => format!("{build}.{u}"),
        _ => build,
    };
    let mut out = product;
    if !display.is_empty() {
        out.push(' ');
        out.push_str(&display);
    }
    if !build_full.is_empty() {
        out.push_str(&format!(" ({build_full})"));
    }
    out
}

pub fn device_info() -> DeviceInfo {
    DeviceInfo {
        product_name: bios("SystemProductName"),
        board_name: board_name(),
        bios_version: bios("BIOSVersion"),
        bios_date: bios("BIOSReleaseDate"),
        os_version: os_version(),
    }
}

/// Serial number from SMBIOS via CIM. Runs PowerShell without a console window.
pub fn serial_number() -> Result<String, String> {
    let mut cmd = std::process::Command::new("powershell.exe");
    cmd.args([
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        "(Get-CimInstance -ClassName Win32_BIOS).SerialNumber",
    ]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(ffi::CREATE_NO_WINDOW);
    }
    let out = cmd.output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    let serial = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if serial.is_empty() {
        Err("N/A".to_string())
    } else {
        Ok(serial)
    }
}

/// Battery percentage, `None` on desktops / unknown state.
pub fn battery_percent() -> Option<u8> {
    let mut status = ffi::SYSTEM_POWER_STATUS::default();
    // SAFETY: valid out-pointer.
    if unsafe { ffi::GetSystemPowerStatus(&mut status) } == 0 {
        return None;
    }
    // 128 = no system battery, 255 = unknown.
    if status.BatteryFlag & 128 != 0 || status.BatteryLifePercent > 100 {
        return None;
    }
    Some(status.BatteryLifePercent)
}

/// `true` when running on AC power.
pub fn on_ac_power() -> Option<bool> {
    let mut status = ffi::SYSTEM_POWER_STATUS::default();
    // SAFETY: valid out-pointer.
    if unsafe { ffi::GetSystemPowerStatus(&mut status) } == 0 {
        return None;
    }
    match status.ACLineStatus {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    }
}

/// Physical memory in use, in percent.
pub fn memory_load_percent() -> Option<u32> {
    let mut mem = ffi::MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<ffi::MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    // SAFETY: dwLength initialised, valid out-pointer.
    (unsafe { ffi::GlobalMemoryStatusEx(&mut mem) } != 0).then_some(mem.dwMemoryLoad)
}

/// Used space of the system drive, in percent.
pub fn system_disk_used_percent() -> Option<u64> {
    let drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".to_string());
    let root = ffi::wide(&format!("{drive}\\"));
    let (mut avail, mut total, mut free) = (0u64, 0u64, 0u64);
    // SAFETY: NUL-terminated path, valid out-pointers.
    let ok = unsafe { ffi::GetDiskFreeSpaceExW(root.as_ptr(), &mut avail, &mut total, &mut free) };
    if ok == 0 || total == 0 {
        return None;
    }
    Some((total - free) * 100 / total)
}

static LAST_CPU_TIMES: Mutex<Option<(u64, u64)>> = Mutex::new(None);

/// CPU usage since the previous call, in percent (first call returns `None`).
pub fn cpu_usage_percent() -> Option<u32> {
    let (mut idle, mut kernel, mut user) = (
        ffi::FILETIME::default(),
        ffi::FILETIME::default(),
        ffi::FILETIME::default(),
    );
    // SAFETY: valid out-pointers.
    if unsafe { ffi::GetSystemTimes(&mut idle, &mut kernel, &mut user) } == 0 {
        return None;
    }
    let idle = idle.as_u64();
    // Kernel time includes idle time.
    let total = kernel.as_u64() + user.as_u64();

    let mut last = LAST_CPU_TIMES.lock().ok()?;
    let previous = last.replace((idle, total));
    let (prev_idle, prev_total) = previous?;
    let d_total = total.saturating_sub(prev_total);
    let d_idle = idle.saturating_sub(prev_idle);
    if d_total == 0 {
        return None;
    }
    Some((d_total.saturating_sub(d_idle) * 100 / d_total) as u32)
}
