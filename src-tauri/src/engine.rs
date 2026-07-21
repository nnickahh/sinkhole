use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        Arc, RwLock,
    },
    time::Instant,
};
use url::Url;

pub const EASYLIST_URL: &str = "https://easylist.to/easylist/easylist.txt";
pub const EASYPRIVACY_URL: &str = "https://easylist.to/easylist/easyprivacy.txt";
pub const WINDOWS_TELEMETRY_URL: &str =
    "https://raw.githubusercontent.com/hagezi/dns-blocklists/main/adblock/native.winoffice.txt";
pub const PROXY_ADDRESS: &str = "127.0.0.1:8118";
pub const CONNECTION_TEST_HOST: &str = "sinkhole.test";

const FALLBACK_AD_RULES: &str = r#"
||doubleclick.net^
||googlesyndication.com^
||googleadservices.com^
||adnxs.com^
||amazon-adsystem.com^
||taboola.com^
||outbrain.com^
"#;

const FALLBACK_TRACKER_RULES: &str = r#"
||scorecardresearch.com^
||google-analytics.com^
||mixpanel.com^
||segment.io^
||hotjar.com^
"#;

const FALLBACK_TELEMETRY_RULES: &str = r#"
||vortex.data.microsoft.com^
||telemetry.microsoft.com^
||settings-win.data.microsoft.com^
||watson.telemetry.microsoft.com^
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RuleCategory {
    Ad,
    Tracker,
    Telemetry,
    Custom,
}

impl RuleCategory {
    fn priority(self) -> u8 {
        match self {
            Self::Custom => 0,
            Self::Ad => 1,
            Self::Tracker => 2,
            Self::Telemetry => 3,
        }
    }

    fn most_specific(self, other: Self) -> Self {
        if other.priority() > self.priority() {
            other
        } else {
            self
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct RuleCounts {
    ads: usize,
    trackers: usize,
    telemetry: usize,
    custom: usize,
}

impl RuleCounts {
    fn total(self) -> usize {
        self.ads + self.trackers + self.telemetry + self.custom
    }
}

#[derive(Debug, Clone, Default)]
pub struct RuleSet {
    blocked_domains: HashMap<String, RuleCategory>,
    allowed_domains: HashSet<String>,
}

impl RuleSet {
    pub fn parse(contents: &str, category: RuleCategory) -> Self {
        let mut rules = Self::default();

        for raw_line in contents.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('!') || line.starts_with('[') {
                continue;
            }

            if let Some(domain) = parse_hosts_line(line) {
                rules.insert_blocked(domain, category);
                continue;
            }

            let (is_exception, candidate) = if let Some(rest) = line.strip_prefix("@@") {
                (true, rest)
            } else {
                (false, line)
            };

            if let Some(domain) = parse_adblock_domain(candidate) {
                if is_exception {
                    rules.allowed_domains.insert(domain);
                } else {
                    rules.insert_blocked(domain, category);
                }
            }
        }

        rules
    }

    fn insert_blocked(&mut self, domain: String, category: RuleCategory) {
        self.blocked_domains
            .entry(domain)
            .and_modify(|current| *current = current.most_specific(category))
            .or_insert(category);
    }

    pub fn merge(&mut self, other: Self) {
        for (domain, category) in other.blocked_domains {
            self.insert_blocked(domain, category);
        }
        self.allowed_domains.extend(other.allowed_domains);
    }

    pub fn len(&self) -> usize {
        self.blocked_domains.len() + self.allowed_domains.len()
    }

    fn counts(&self) -> RuleCounts {
        let mut counts = RuleCounts::default();
        for category in self.blocked_domains.values() {
            match category {
                RuleCategory::Ad => counts.ads += 1,
                RuleCategory::Tracker => counts.trackers += 1,
                RuleCategory::Telemetry => counts.telemetry += 1,
                RuleCategory::Custom => counts.custom += 1,
            }
        }
        counts
    }

    fn classify_host(&self, host: &str, user_whitelist: &HashSet<String>) -> Option<RuleCategory> {
        if domain_or_parent_is_listed(host, user_whitelist)
            || domain_or_parent_is_listed(host, &self.allowed_domains)
        {
            return None;
        }

        domain_or_parent_category(host, &self.blocked_domains)
    }
}

#[derive(Clone)]
pub struct AdblockEngine {
    inner: Arc<EngineInner>,
}

struct EngineInner {
    rules: RwLock<RuleSet>,
    user_whitelist: RwLock<HashSet<String>>,
    enabled: AtomicBool,
    blocked_ads: AtomicU64,
    blocked_trackers: AtomicU64,
    blocked_telemetry: AtomicU64,
    blocked_custom: AtomicU64,
    requests_processed: AtomicU64,
    last_request_millis: AtomicU64,
    active_lists: AtomicUsize,
    proxy_running: AtomicBool,
    started_at: Instant,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdblockStats {
    pub total_blocked: u64,
    pub blocked_ads: u64,
    pub blocked_trackers: u64,
    pub blocked_telemetry: u64,
    pub blocked_custom: u64,
    pub active_rules: usize,
    pub active_ad_rules: usize,
    pub active_tracker_rules: usize,
    pub active_telemetry_rules: usize,
    pub active_custom_rules: usize,
    pub active_lists: usize,
    pub protection_enabled: bool,
    pub proxy_running: bool,
    pub proxy_address: &'static str,
    pub requests_processed: u64,
    pub last_request_age_seconds: Option<u64>,
    pub uptime_seconds: u64,
    pub bandwidth_saved_bytes: u64,
}

impl AdblockEngine {
    pub fn new(enabled: bool, whitelist: &[String]) -> Self {
        let mut fallback_rules = RuleSet::parse(FALLBACK_AD_RULES, RuleCategory::Ad);
        fallback_rules.merge(RuleSet::parse(
            FALLBACK_TRACKER_RULES,
            RuleCategory::Tracker,
        ));
        fallback_rules.merge(RuleSet::parse(
            FALLBACK_TELEMETRY_RULES,
            RuleCategory::Telemetry,
        ));

        let engine = Self {
            inner: Arc::new(EngineInner {
                rules: RwLock::new(fallback_rules),
                user_whitelist: RwLock::new(HashSet::new()),
                enabled: AtomicBool::new(enabled),
                blocked_ads: AtomicU64::new(0),
                blocked_trackers: AtomicU64::new(0),
                blocked_telemetry: AtomicU64::new(0),
                blocked_custom: AtomicU64::new(0),
                requests_processed: AtomicU64::new(0),
                last_request_millis: AtomicU64::new(0),
                active_lists: AtomicUsize::new(3),
                proxy_running: AtomicBool::new(false),
                started_at: Instant::now(),
            }),
        };
        engine.set_whitelist(whitelist);
        engine
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.inner.enabled.store(enabled, Ordering::Relaxed);
    }

    pub fn is_enabled(&self) -> bool {
        self.inner.enabled.load(Ordering::Relaxed)
    }

    pub fn set_proxy_running(&self, running: bool) {
        self.inner.proxy_running.store(running, Ordering::Relaxed);
    }

    pub fn set_whitelist(&self, domains: &[String]) {
        let normalized = domains
            .iter()
            .filter_map(|domain| normalize_domain(domain))
            .collect();
        *self
            .inner
            .user_whitelist
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = normalized;
    }

    pub fn replace_rules(&self, sources: &[(String, RuleCategory)]) -> usize {
        let mut combined = RuleSet::default();
        for (source, category) in sources {
            combined.merge(RuleSet::parse(source, *category));
        }

        let count = combined.len();
        if count > 0 {
            *self
                .inner
                .rules
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = combined;
            self.inner
                .active_lists
                .store(sources.len(), Ordering::Relaxed);
        }
        count
    }

    pub fn check_url(&self, url: &str) -> bool {
        self.classify_url(url).is_some()
    }

    pub fn classify_url(&self, url: &str) -> Option<RuleCategory> {
        if !self.is_enabled() {
            return None;
        }
        let host = host_from_input(url)?;
        self.classify_host(&host)
    }

    pub fn classify_host(&self, host: &str) -> Option<RuleCategory> {
        if !self.is_enabled() {
            return None;
        }

        let host = normalize_domain(host)?;
        let rules = self
            .inner
            .rules
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let whitelist = self
            .inner
            .user_whitelist
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        rules.classify_host(&host, &whitelist)
    }

    pub fn record_request(&self) {
        self.inner
            .requests_processed
            .fetch_add(1, Ordering::Relaxed);
        let elapsed_millis = self
            .inner
            .started_at
            .elapsed()
            .as_millis()
            .min(u64::MAX as u128) as u64;
        self.inner
            .last_request_millis
            .store(elapsed_millis.saturating_add(1), Ordering::Relaxed);
    }

    pub fn record_block(&self, category: RuleCategory) {
        let counter = match category {
            RuleCategory::Ad => &self.inner.blocked_ads,
            RuleCategory::Tracker => &self.inner.blocked_trackers,
            RuleCategory::Telemetry => &self.inner.blocked_telemetry,
            RuleCategory::Custom => &self.inner.blocked_custom,
        };
        counter.fetch_add(1, Ordering::Relaxed);
    }

    pub fn stats(&self) -> AdblockStats {
        let blocked_ads = self.inner.blocked_ads.load(Ordering::Relaxed);
        let blocked_trackers = self.inner.blocked_trackers.load(Ordering::Relaxed);
        let blocked_telemetry = self.inner.blocked_telemetry.load(Ordering::Relaxed);
        let blocked_custom = self.inner.blocked_custom.load(Ordering::Relaxed);
        let total_blocked = blocked_ads
            .saturating_add(blocked_trackers)
            .saturating_add(blocked_telemetry)
            .saturating_add(blocked_custom);
        let rule_counts = self
            .inner
            .rules
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .counts();
        let elapsed_millis = self
            .inner
            .started_at
            .elapsed()
            .as_millis()
            .min(u64::MAX as u128) as u64;
        let last_request_millis = self.inner.last_request_millis.load(Ordering::Relaxed);
        let last_request_age_seconds = (last_request_millis > 0)
            .then(|| elapsed_millis.saturating_sub(last_request_millis.saturating_sub(1)) / 1_000);

        AdblockStats {
            total_blocked,
            blocked_ads,
            blocked_trackers,
            blocked_telemetry,
            blocked_custom,
            active_rules: rule_counts.total(),
            active_ad_rules: rule_counts.ads,
            active_tracker_rules: rule_counts.trackers,
            active_telemetry_rules: rule_counts.telemetry,
            active_custom_rules: rule_counts.custom,
            active_lists: self.inner.active_lists.load(Ordering::Relaxed),
            protection_enabled: self.is_enabled(),
            proxy_running: self.inner.proxy_running.load(Ordering::Relaxed),
            proxy_address: PROXY_ADDRESS,
            requests_processed: self.inner.requests_processed.load(Ordering::Relaxed),
            last_request_age_seconds,
            uptime_seconds: self.inner.started_at.elapsed().as_secs(),
            bandwidth_saved_bytes: total_blocked.saturating_mul(42_000),
        }
    }
}

pub fn category_for_url(url: &str) -> RuleCategory {
    match url {
        EASYLIST_URL => RuleCategory::Ad,
        EASYPRIVACY_URL => RuleCategory::Tracker,
        WINDOWS_TELEMETRY_URL => RuleCategory::Telemetry,
        _ => RuleCategory::Custom,
    }
}

pub fn default_filter_list_urls() -> Vec<String> {
    vec![
        EASYLIST_URL.to_owned(),
        EASYPRIVACY_URL.to_owned(),
        WINDOWS_TELEMETRY_URL.to_owned(),
    ]
}

fn parse_hosts_line(line: &str) -> Option<String> {
    let mut parts = line.split_whitespace();
    let address = parts.next()?;
    if address != "0.0.0.0" && address != "127.0.0.1" && address != "::" {
        return None;
    }
    normalize_domain(parts.next()?)
}

fn parse_adblock_domain(line: &str) -> Option<String> {
    let anchored = line.strip_prefix("||")?;
    let end = anchored
        .find(['^', '/', '$', '*', '|'])
        .unwrap_or(anchored.len());
    normalize_domain(&anchored[..end])
}

pub(crate) fn normalize_domain(value: &str) -> Option<String> {
    let mut domain = value
        .trim()
        .trim_end_matches('.')
        .trim_start_matches('.')
        .to_ascii_lowercase();

    if let Some((host, port)) = domain.rsplit_once(':') {
        if !host.contains(':') && port.parse::<u16>().is_ok() {
            domain = host.to_owned();
        }
    }

    if domain.is_empty()
        || domain == "localhost"
        || domain.contains('/')
        || domain.contains(' ')
        || !domain
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '-'))
    {
        return None;
    }

    Some(domain)
}

fn host_from_input(input: &str) -> Option<String> {
    if let Ok(url) = Url::parse(input) {
        return url.host_str().and_then(normalize_domain);
    }
    normalize_domain(input)
}

fn domain_or_parent_is_listed(host: &str, domains: &HashSet<String>) -> bool {
    let mut candidate = host;
    loop {
        if domains.contains(candidate) {
            return true;
        }
        let Some(dot_index) = candidate.find('.') else {
            return false;
        };
        candidate = &candidate[dot_index + 1..];
    }
}

fn domain_or_parent_category(
    host: &str,
    domains: &HashMap<String, RuleCategory>,
) -> Option<RuleCategory> {
    let mut candidate = host;
    loop {
        if let Some(category) = domains.get(candidate) {
            return Some(*category);
        }
        let dot_index = candidate.find('.')?;
        candidate = &candidate[dot_index + 1..];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_easylist_hosts_exceptions_and_categories() {
        let mut rules = RuleSet::parse(
            "! comment\n||ads.example.com^$third-party\n@@||safe.ads.example.com^\n",
            RuleCategory::Ad,
        );
        rules.merge(RuleSet::parse(
            "0.0.0.0 tracker.test\n||ads.example.com^\n",
            RuleCategory::Tracker,
        ));

        assert_eq!(rules.len(), 3);
        assert_eq!(
            rules.classify_host("cdn.ads.example.com", &HashSet::new()),
            Some(RuleCategory::Tracker)
        );
        assert_eq!(
            rules.classify_host("safe.ads.example.com", &HashSet::new()),
            None
        );
        assert_eq!(
            rules.classify_host("tracker.test", &HashSet::new()),
            Some(RuleCategory::Tracker)
        );
    }

    #[test]
    fn toggle_and_user_whitelist_are_authoritative() {
        let engine = AdblockEngine::new(true, &["doubleclick.net".to_owned()]);
        assert!(!engine.check_url("http://ads.doubleclick.net/pagead.js"));
        engine.set_whitelist(&[]);
        assert_eq!(
            engine.classify_url("http://ads.doubleclick.net/pagead.js"),
            Some(RuleCategory::Ad)
        );
        engine.set_enabled(false);
        assert!(!engine.check_url("http://ads.doubleclick.net/pagead.js"));
    }

    #[test]
    fn requests_and_block_categories_are_counted_separately() {
        let engine = AdblockEngine::new(true, &[]);
        engine.record_request();
        engine.record_block(RuleCategory::Tracker);
        engine.record_block(RuleCategory::Telemetry);
        let stats = engine.stats();
        assert_eq!(stats.requests_processed, 1);
        assert_eq!(stats.total_blocked, 2);
        assert_eq!(stats.blocked_trackers, 1);
        assert_eq!(stats.blocked_telemetry, 1);
        assert!(stats.last_request_age_seconds.is_some());
    }

    #[test]
    #[ignore = "requires live filter-list access"]
    fn live_default_lists_contain_more_than_one_hundred_thousand_supported_rules() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime should build");
        let count = runtime.block_on(async {
            let mut combined = RuleSet::default();
            for url in default_filter_list_urls() {
                let source = reqwest::get(&url)
                    .await
                    .expect("filter list should respond")
                    .error_for_status()
                    .expect("filter list should return a successful status")
                    .text()
                    .await
                    .expect("filter list should contain text");
                combined.merge(RuleSet::parse(&source, category_for_url(&url)));
            }
            combined.len()
        });
        assert!(count > 100_000, "loaded {count} supported privacy rules");
    }
}
