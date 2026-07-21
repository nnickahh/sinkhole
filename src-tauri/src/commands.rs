use crate::{
    engine::{
        category_for_url, default_filter_list_urls, normalize_domain, AdblockEngine, AdblockStats,
        RuleCategory, EASYLIST_URL,
    },
    system_proxy,
};
use serde::{Deserialize, Serialize};
use std::{sync::RwLock, time::Duration};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_store::StoreExt;
use url::Url;

const SETTINGS_FILE: &str = "settings.json";
const SETTINGS_KEY: &str = "settings";
const MAX_FILTER_LISTS: usize = 12;
const MAX_FILTER_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub protection_enabled: bool,
    #[serde(default)]
    pub system_proxy_enabled: bool,
    pub filter_list_urls: Vec<String>,
    pub whitelist_domains: Vec<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            protection_enabled: true,
            system_proxy_enabled: false,
            filter_list_urls: default_filter_list_urls(),
            whitelist_domains: Vec::new(),
        }
    }
}

pub struct AppState {
    pub engine: AdblockEngine,
    pub settings: RwLock<AppSettings>,
}

impl AppState {
    pub fn new(settings: AppSettings) -> Self {
        Self {
            engine: AdblockEngine::new(settings.protection_enabled, &settings.whitelist_domains),
            settings: RwLock::new(settings),
        }
    }

    pub fn settings_snapshot(&self) -> AppSettings {
        self.settings
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}

pub fn load_settings(app: &AppHandle) -> AppSettings {
    let Ok(store) = app.store(SETTINGS_FILE) else {
        return AppSettings::default();
    };
    let Some(value) = store.get(SETTINGS_KEY) else {
        return AppSettings::default();
    };

    let mut settings: AppSettings = serde_json::from_value(value).unwrap_or_default();
    if settings.filter_list_urls.len() == 1
        && settings
            .filter_list_urls
            .first()
            .is_some_and(|url| url == EASYLIST_URL)
    {
        settings.filter_list_urls = default_filter_list_urls();
    }
    settings
}

fn persist_settings(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    let store = app
        .store(SETTINGS_FILE)
        .map_err(|error| error.to_string())?;
    let value = serde_json::to_value(settings).map_err(|error| error.to_string())?;
    store.set(SETTINGS_KEY, value);
    store.save().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn toggle_adblocker(
    app: AppHandle,
    state: State<'_, AppState>,
    enable: bool,
) -> Result<bool, String> {
    set_protection(&app, &state, enable)
}

pub fn set_protection(app: &AppHandle, state: &AppState, enable: bool) -> Result<bool, String> {
    state.engine.set_enabled(enable);
    let settings = {
        let mut settings = state
            .settings
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        settings.protection_enabled = enable;
        settings.clone()
    };
    persist_settings(app, &settings)?;
    Ok(enable)
}

#[tauri::command]
pub fn get_stats(state: State<'_, AppState>) -> AdblockStats {
    state.engine.stats()
}

#[tauri::command]
pub fn clear_activity_log(state: State<'_, AppState>) -> usize {
    state.engine.clear_recent_blocks()
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> AppSettings {
    state.settings_snapshot()
}

#[tauri::command]
pub fn set_system_proxy(
    app: AppHandle,
    state: State<'_, AppState>,
    enable: bool,
) -> Result<AppSettings, String> {
    if enable {
        system_proxy::enable(&app)?;
    } else {
        system_proxy::disable(&app)?;
    }

    let settings = {
        let mut settings = state
            .settings
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        settings.system_proxy_enabled = enable;
        settings.clone()
    };
    persist_settings(&app, &settings)?;
    Ok(settings)
}

#[tauri::command]
pub fn set_quick_protection(
    app: AppHandle,
    state: State<'_, AppState>,
    enable: bool,
) -> Result<AppSettings, String> {
    if enable {
        system_proxy::enable(&app)?;
    } else {
        system_proxy::disable(&app)?;
    }

    state.engine.set_enabled(enable);
    let settings = {
        let mut settings = state
            .settings
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        settings.protection_enabled = enable;
        settings.system_proxy_enabled = enable;
        settings.clone()
    };
    persist_settings(&app, &settings)?;
    Ok(settings)
}

#[tauri::command]
pub fn open_dashboard(app: AppHandle) -> Result<(), String> {
    let dashboard = app
        .get_webview_window("main")
        .ok_or_else(|| "Dashboard window is unavailable".to_owned())?;
    dashboard.unminimize().map_err(|error| error.to_string())?;
    dashboard.show().map_err(|error| error.to_string())?;
    dashboard.set_focus().map_err(|error| error.to_string())?;
    if let Some(quick) = app.get_webview_window("quick") {
        let _ = quick.hide();
    }
    Ok(())
}

#[tauri::command]
pub fn update_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<AppSettings, String> {
    let settings = validate_settings(settings)?;
    state.engine.set_enabled(settings.protection_enabled);
    state.engine.set_whitelist(&settings.whitelist_domains);
    persist_settings(&app, &settings)?;
    *state
        .settings
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = settings.clone();
    Ok(settings)
}

#[tauri::command]
pub async fn update_blocklists(state: State<'_, AppState>) -> Result<usize, String> {
    let settings = state.settings_snapshot();
    fetch_and_replace_rules(&state.engine, &settings).await
}

#[tauri::command]
pub fn check_link(state: State<'_, AppState>, url: String) -> bool {
    state.engine.check_url(&url)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkInspection {
    pub blocked: bool,
    pub category: Option<RuleCategory>,
}

#[tauri::command]
pub fn inspect_link(state: State<'_, AppState>, url: String) -> LinkInspection {
    let category = state.engine.classify_url(&url);
    LinkInspection {
        blocked: category.is_some(),
        category,
    }
}

pub async fn fetch_and_replace_rules(
    engine: &AdblockEngine,
    settings: &AppSettings,
) -> Result<usize, String> {
    let client = reqwest::Client::builder()
        .user_agent("SinkHole/0.1")
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|error| error.to_string())?;

    let mut sources = Vec::new();
    let mut failures = Vec::new();

    for list_url in &settings.filter_list_urls {
        match fetch_filter_source(&client, list_url).await {
            Ok(source) => sources.push((source, category_for_url(list_url))),
            Err(error) => failures.push(format!("{list_url}: {error}")),
        }
    }

    if sources.is_empty() {
        return Err(format!(
            "No filter lists could be updated. {}",
            failures.join(" | ")
        ));
    }

    let count = engine.replace_rules(&sources);
    if count == 0 {
        return Err("Downloaded filter lists did not contain supported domain rules".to_owned());
    }
    Ok(count)
}

async fn fetch_filter_source(client: &reqwest::Client, list_url: &str) -> Result<String, String> {
    let response = client
        .get(list_url)
        .send()
        .await
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?;

    if response
        .content_length()
        .is_some_and(|length| length > MAX_FILTER_BYTES as u64)
    {
        return Err("filter list exceeds the 16 MiB limit".to_owned());
    }

    let source = response.text().await.map_err(|error| error.to_string())?;
    if source.len() > MAX_FILTER_BYTES {
        return Err("filter list exceeds the 16 MiB limit".to_owned());
    }
    Ok(source)
}

fn validate_settings(mut settings: AppSettings) -> Result<AppSettings, String> {
    if settings.filter_list_urls.is_empty() {
        return Err("At least one filter list is required".to_owned());
    }
    if settings.filter_list_urls.len() > MAX_FILTER_LISTS {
        return Err(format!(
            "At most {MAX_FILTER_LISTS} filter lists are supported"
        ));
    }

    for list_url in &mut settings.filter_list_urls {
        *list_url = list_url.trim().to_owned();
        let parsed =
            Url::parse(list_url).map_err(|_| format!("Invalid filter list URL: {list_url}"))?;
        if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
            return Err(format!("Filter list must use HTTP or HTTPS: {list_url}"));
        }
    }
    settings.filter_list_urls.sort();
    settings.filter_list_urls.dedup();

    settings.whitelist_domains = settings
        .whitelist_domains
        .iter()
        .map(|domain| {
            normalize_domain(domain).ok_or_else(|| format!("Invalid whitelist domain: {domain}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    settings.whitelist_domains.sort();
    settings.whitelist_domains.dedup();
    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip_and_normalize() {
        let settings = validate_settings(AppSettings {
            protection_enabled: false,
            system_proxy_enabled: false,
            filter_list_urls: vec![EASYLIST_URL.to_owned(), EASYLIST_URL.to_owned()],
            whitelist_domains: vec![".Example.COM.".to_owned()],
        })
        .expect("settings should validate");

        assert_eq!(settings.filter_list_urls.len(), 1);
        assert_eq!(settings.whitelist_domains, vec!["example.com"]);
        let encoded = serde_json::to_string(&settings).expect("settings should serialize");
        let decoded: AppSettings =
            serde_json::from_str(&encoded).expect("settings should deserialize");
        assert_eq!(decoded, settings);
    }

    #[test]
    fn settings_reject_non_http_filter_sources() {
        let result = validate_settings(AppSettings {
            filter_list_urls: vec!["file:///tmp/list.txt".to_owned()],
            ..AppSettings::default()
        });
        assert!(result.is_err());
    }
}
