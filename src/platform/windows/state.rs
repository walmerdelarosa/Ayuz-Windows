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

//! Small key/value store for hardware settings that the firmware does not
//! report back (e.g. the battery charge limit). On Linux `asusd` persists
//! these; on Windows Ayuz keeps them itself and re-applies them on start.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

static LOCK: Mutex<()> = Mutex::new(());

fn state_path() -> Option<PathBuf> {
    crate::services::config::AppConfig::config_dir().map(|d| d.join("windows_state.txt"))
}

fn read_all() -> BTreeMap<String, String> {
    let Some(path) = state_path() else {
        return BTreeMap::new();
    };
    let Ok(text) = std::fs::read_to_string(path) else {
        return BTreeMap::new();
    };
    text.lines()
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .collect()
}

pub fn get_u32(key: &str) -> Option<u32> {
    let _guard = LOCK.lock().ok()?;
    read_all().get(key)?.parse().ok()
}

pub fn set_u32(key: &str, value: u32) {
    let Ok(_guard) = LOCK.lock() else {
        return;
    };
    let Some(path) = state_path() else {
        return;
    };
    let mut all = read_all();
    all.insert(key.to_string(), value.to_string());
    let text: String = all.iter().map(|(k, v)| format!("{k}={v}\n")).collect();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Err(e) = std::fs::write(&path, text) {
        tracing::warn!("windows state: cannot write {}: {}", path.display(), e);
    }
}
