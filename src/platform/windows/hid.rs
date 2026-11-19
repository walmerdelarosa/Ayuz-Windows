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

//! Minimal HID enumeration and feature-report writing via SetupAPI/hid.dll.

use super::ffi::{self, OwnedHandle};

pub const ASUS_VENDOR_ID: u16 = 0x0B05;
/// ASUS vendor-defined usage page used by the Aura / hotkey collections.
pub const ASUS_USAGE_PAGE: u16 = 0xFF31;

#[derive(Debug, Clone)]
pub struct HidInfo {
    pub path: String,
    pub vendor_id: u16,
    pub product_id: u16,
    pub usage_page: u16,
    pub usage: u16,
    pub feature_report_len: u16,
}

/// Lists every present HID interface whose vendor ID matches `vendor_id`.
pub fn enumerate(vendor_id: u16) -> Vec<HidInfo> {
    let mut out = Vec::new();
    let mut guid = ffi::GUID::default();
    // SAFETY: `guid` is a valid out-pointer.
    unsafe { ffi::HidD_GetHidGuid(&mut guid) };

    // SAFETY: valid GUID pointer, null enumerator/parent are allowed.
    let set = unsafe {
        ffi::SetupDiGetClassDevsW(
            &guid,
            std::ptr::null(),
            std::ptr::null_mut(),
            ffi::DIGCF_PRESENT | ffi::DIGCF_DEVICEINTERFACE,
        )
    };
    if set == ffi::INVALID_HDEVINFO {
        return out;
    }

    let mut index = 0u32;
    loop {
        let mut iface = ffi::SP_DEVICE_INTERFACE_DATA {
            cbSize: std::mem::size_of::<ffi::SP_DEVICE_INTERFACE_DATA>() as u32,
            InterfaceClassGuid: ffi::GUID::default(),
            Flags: 0,
            Reserved: 0,
        };
        // SAFETY: `set` is valid, `iface.cbSize` is initialised.
        let ok = unsafe {
            ffi::SetupDiEnumDeviceInterfaces(set, std::ptr::null(), &guid, index, &mut iface)
        };
        if ok == 0 {
            break;
        }
        index += 1;

        if let Some(path) = interface_path(set, &iface)
            && let Some(info) = query(&path)
            && info.vendor_id == vendor_id
        {
            out.push(info);
        }
    }

    // SAFETY: `set` was returned by SetupDiGetClassDevsW.
    unsafe { ffi::SetupDiDestroyDeviceInfoList(set) };
    out
}

/// Resolves the `\\?\hid#...` path of one device interface.
fn interface_path(set: ffi::HDEVINFO, iface: &ffi::SP_DEVICE_INTERFACE_DATA) -> Option<String> {
    let mut required: u32 = 0;
    // SAFETY: size query with a null buffer.
    unsafe {
        ffi::SetupDiGetDeviceInterfaceDetailW(
            set,
            iface,
            std::ptr::null_mut(),
            0,
            &mut required,
            std::ptr::null_mut(),
        )
    };
    if required < 8 {
        return None;
    }

    // SP_DEVICE_INTERFACE_DETAIL_DATA_W: { DWORD cbSize; WCHAR DevicePath[1]; }
    // Allocate as u64 for 8-byte alignment.
    let mut buf: Vec<u64> = vec![0; (required as usize).div_ceil(8)];
    let base = buf.as_mut_ptr().cast::<u8>();
    let cb_size: u32 = if cfg!(target_pointer_width = "64") {
        8
    } else {
        6
    };
    // SAFETY: `buf` holds at least `required` bytes; the first DWORD is cbSize.
    unsafe {
        base.cast::<u32>().write(cb_size);
    }
    // SAFETY: buffer size matches `required`.
    let ok = unsafe {
        ffi::SetupDiGetDeviceInterfaceDetailW(
            set,
            iface,
            base.cast(),
            required,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if ok == 0 {
        return None;
    }

    // DevicePath starts right after the 4-byte cbSize field.
    let max_units = (required as usize - 4) / 2;
    // SAFETY: the path is a NUL-terminated UTF-16 string inside `buf`.
    let units = unsafe { std::slice::from_raw_parts(base.add(4).cast::<u16>(), max_units) };
    let len = units.iter().position(|&c| c == 0).unwrap_or(units.len());
    Some(String::from_utf16_lossy(&units[..len]))
}

/// Reads VID/PID and top-level collection caps of a HID path. Opens the
/// device with zero access rights, which is allowed even for collections the
/// OS owns exclusively (keyboards/mice).
fn query(path: &str) -> Option<HidInfo> {
    let handle = ffi::open_device(path, 0)?;

    let mut attrs = ffi::HIDD_ATTRIBUTES {
        Size: std::mem::size_of::<ffi::HIDD_ATTRIBUTES>() as u32,
        ..Default::default()
    };
    // SAFETY: valid handle and out-pointer.
    if unsafe { ffi::HidD_GetAttributes(handle.0, &mut attrs) } == 0 {
        return None;
    }

    let mut caps = ffi::HIDP_CAPS::default();
    let mut preparsed: isize = 0;
    // SAFETY: valid handle and out-pointer.
    if unsafe { ffi::HidD_GetPreparsedData(handle.0, &mut preparsed) } != 0 {
        // SAFETY: `preparsed` came from HidD_GetPreparsedData.
        let status = unsafe { ffi::HidP_GetCaps(preparsed, &mut caps) };
        // SAFETY: freed exactly once.
        unsafe { ffi::HidD_FreePreparsedData(preparsed) };
        if status != ffi::HIDP_STATUS_SUCCESS {
            caps = ffi::HIDP_CAPS::default();
        }
    }

    Some(HidInfo {
        path: path.to_string(),
        vendor_id: attrs.VendorID,
        product_id: attrs.ProductID,
        usage_page: caps.UsagePage,
        usage: caps.Usage,
        feature_report_len: caps.FeatureReportByteLength,
    })
}

/// An opened HID device that accepts feature reports.
pub struct HidDevice {
    handle: OwnedHandle,
    feature_len: usize,
}

impl HidDevice {
    pub fn open(info: &HidInfo) -> Option<Self> {
        let handle = ffi::open_device(&info.path, ffi::GENERIC_READ | ffi::GENERIC_WRITE)?;
        Some(Self {
            handle,
            feature_len: info.feature_report_len as usize,
        })
    }

    /// Sends `data` (first byte = report ID) as a feature report, zero-padded
    /// to the collection's feature report length.
    pub fn send_feature(&self, data: &[u8]) -> bool {
        let len = self.feature_len.max(data.len());
        let mut buf = vec![0u8; len];
        buf[..data.len()].copy_from_slice(data);
        // SAFETY: `buf` is valid for `len` bytes.
        unsafe { ffi::HidD_SetFeature(self.handle.0, buf.as_ptr().cast(), len as u32) != 0 }
    }
}
