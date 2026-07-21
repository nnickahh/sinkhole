use crate::engine::PROXY_ADDRESS;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

const SETTINGS_FILE: &str = "settings.json";
const BACKUP_KEY: &str = "systemProxyBackup";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProxySnapshot {
    proxy_enable: Option<u32>,
    proxy_server: Option<String>,
    proxy_override: Option<String>,
}

#[cfg(windows)]
mod platform {
    use super::*;
    use std::{ffi::c_void, io};
    use windows_sys::Win32::Networking::WinInet::{
        InternetSetOptionW, INTERNET_OPTION_REFRESH, INTERNET_OPTION_SETTINGS_CHANGED,
    };
    use winreg::{
        enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE},
        RegKey,
    };

    const INTERNET_SETTINGS: &str = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";

    pub(super) fn snapshot() -> Result<ProxySnapshot, String> {
        let key = open_key()?;
        Ok(ProxySnapshot {
            proxy_enable: key.get_value("ProxyEnable").ok(),
            proxy_server: key.get_value("ProxyServer").ok(),
            proxy_override: key.get_value("ProxyOverride").ok(),
        })
    }

    pub(super) fn apply() -> Result<(), String> {
        let key = open_key()?;
        key.set_value("ProxyEnable", &1_u32)
            .map_err(|error| error.to_string())?;
        key.set_value("ProxyServer", &PROXY_ADDRESS)
            .map_err(|error| error.to_string())?;

        let current_override: String = key.get_value("ProxyOverride").unwrap_or_default();
        let mut entries = current_override
            .split(';')
            .filter(|entry| !entry.trim().is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>();
        if !entries
            .iter()
            .any(|entry| entry.eq_ignore_ascii_case("<local>"))
        {
            entries.push("<local>".to_owned());
        }
        key.set_value("ProxyOverride", &entries.join(";"))
            .map_err(|error| error.to_string())?;
        notify_windows()
    }

    pub(super) fn restore(snapshot: &ProxySnapshot) -> Result<(), String> {
        let key = open_key()?;
        restore_value(&key, "ProxyEnable", snapshot.proxy_enable.as_ref())?;
        restore_value(&key, "ProxyServer", snapshot.proxy_server.as_ref())?;
        restore_value(&key, "ProxyOverride", snapshot.proxy_override.as_ref())?;
        notify_windows()
    }

    fn open_key() -> Result<RegKey, String> {
        RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey_with_flags(INTERNET_SETTINGS, KEY_READ | KEY_WRITE)
            .map_err(|error| format!("Could not open Windows proxy settings: {error}"))
    }

    fn restore_value<T: winreg::types::ToRegValue>(
        key: &RegKey,
        name: &str,
        value: Option<&T>,
    ) -> Result<(), String> {
        match value {
            Some(value) => key
                .set_value(name, value)
                .map_err(|error| error.to_string()),
            None => match key.delete_value(name) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(error.to_string()),
            },
        }
    }

    fn notify_windows() -> Result<(), String> {
        let changed = unsafe {
            InternetSetOptionW(
                std::ptr::null::<c_void>(),
                INTERNET_OPTION_SETTINGS_CHANGED,
                std::ptr::null::<c_void>(),
                0,
            )
        };
        let refreshed = unsafe {
            InternetSetOptionW(
                std::ptr::null::<c_void>(),
                INTERNET_OPTION_REFRESH,
                std::ptr::null::<c_void>(),
                0,
            )
        };
        if changed == 0 || refreshed == 0 {
            return Err(format!(
                "Windows accepted the proxy values but did not refresh them: {}",
                io::Error::last_os_error()
            ));
        }
        Ok(())
    }
}

#[cfg(not(windows))]
mod platform {
    use super::*;

    pub(super) fn snapshot() -> Result<ProxySnapshot, String> {
        Err("One-click browser connection is currently available on Windows only".to_owned())
    }

    pub(super) fn apply() -> Result<(), String> {
        Err("One-click browser connection is currently available on Windows only".to_owned())
    }

    pub(super) fn restore(_snapshot: &ProxySnapshot) -> Result<(), String> {
        Err("One-click browser connection is currently available on Windows only".to_owned())
    }
}

pub fn enable(app: &AppHandle) -> Result<(), String> {
    let store = app
        .store(SETTINGS_FILE)
        .map_err(|error| error.to_string())?;
    if store.get(BACKUP_KEY).is_none() {
        let snapshot = platform::snapshot()?;
        store.set(
            BACKUP_KEY,
            serde_json::to_value(snapshot).map_err(|error| error.to_string())?,
        );
        store.save().map_err(|error| error.to_string())?;
    }
    platform::apply()
}

pub fn disable(app: &AppHandle) -> Result<(), String> {
    let store = app
        .store(SETTINGS_FILE)
        .map_err(|error| error.to_string())?;
    let Some(value) = store.get(BACKUP_KEY) else {
        return Ok(());
    };
    let snapshot: ProxySnapshot =
        serde_json::from_value(value).map_err(|error| error.to_string())?;
    platform::restore(&snapshot)?;
    store.delete(BACKUP_KEY);
    store.save().map_err(|error| error.to_string())
}
