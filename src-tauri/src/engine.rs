use serde::Serialize;
use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        Arc, RwLock,
    },
    time::Instant,
};
use url::Url;

pub const DEFAULT_FILTER_LIST: &str = "https://easylist.to/easylist/easylist.txt";
pub const PROXY_ADDRESS: &str = "127.0.0.1:8118";

const FALLBACK_RULES: &str = r#"
||doubleclick.net^
||googlesyndication.com^
||googleadservices.com^
||adnxs.com^
||amazon-adsystem.com^
||scorecardresearch.com^
||taboola.com^
||outbrain.com^
"#;

#[derive(Debug, Clone, Default)]
pub struct RuleSet {
    blocked_domains: HashSet<String>,
    allowed_domains: HashSet<String>,
}

impl RuleSet {
    pub fn parse(contents: &str) -> Self {
        let mut rules = Self::default();

        for raw_line in contents.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('!') || line.starts_with('[') {
                continue;
            }

            if let Some(domain) = parse_hosts_line(line) {
                rules.blocked_domains.insert(domain);
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
                    rules.blocked_domains.insert(domain);
                }
            }
        }

        rules
    }

    pub fn merge(&mut self, other: Self) {
        self.blocked_domains.extend(other.blocked_domains);
        self.allowed_domains.extend(other.allowed_domains);
    }

    pub fn len(&self) -> usize {
        self.blocked_domains.len() + self.allowed_domains.len()
    }

    fn blocks_host(&self, host: &str, user_whitelist: &HashSet<String>) -> bool {
        if domain_or_parent_is_listed(host, user_whitelist)
            || domain_or_parent_is_listed(host, &self.allowed_domains)
        {
            return false;
        }

        domain_or_parent_is_listed(host, &self.blocked_domains)
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
    blocked_count: AtomicU64,
    active_lists: AtomicUsize,
    proxy_running: AtomicBool,
    started_at: Instant,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdblockStats {
    pub total_blocked: u64,
    pub active_rules: usize,
    pub active_lists: usize,
    pub protection_enabled: bool,
    pub proxy_running: bool,
    pub proxy_address: &'static str,
    pub uptime_seconds: u64,
    pub bandwidth_saved_bytes: u64,
}

impl AdblockEngine {
    pub fn new(enabled: bool, whitelist: &[String]) -> Self {
        let engine = Self {
            inner: Arc::new(EngineInner {
                rules: RwLock::new(RuleSet::parse(FALLBACK_RULES)),
                user_whitelist: RwLock::new(HashSet::new()),
                enabled: AtomicBool::new(enabled),
                blocked_count: AtomicU64::new(0),
                active_lists: AtomicUsize::new(1),
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

    pub fn replace_rules(&self, sources: &[String]) -> usize {
        let mut combined = RuleSet::default();
        for source in sources {
            combined.merge(RuleSet::parse(source));
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
        if !self.is_enabled() {
            return false;
        }

        let Some(host) = host_from_input(url) else {
            return false;
        };
        self.check_host(&host)
    }

    pub fn check_host(&self, host: &str) -> bool {
        if !self.is_enabled() {
            return false;
        }

        let Some(host) = normalize_domain(host) else {
            return false;
        };
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
        rules.blocks_host(&host, &whitelist)
    }

    pub fn record_block(&self) {
        self.inner.blocked_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn stats(&self) -> AdblockStats {
        let total_blocked = self.inner.blocked_count.load(Ordering::Relaxed);
        let active_rules = self
            .inner
            .rules
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len();

        AdblockStats {
            total_blocked,
            active_rules,
            active_lists: self.inner.active_lists.load(Ordering::Relaxed),
            protection_enabled: self.is_enabled(),
            proxy_running: self.inner.proxy_running.load(Ordering::Relaxed),
            proxy_address: PROXY_ADDRESS,
            uptime_seconds: self.inner.started_at.elapsed().as_secs(),
            bandwidth_saved_bytes: total_blocked.saturating_mul(42_000),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_easylist_and_hosts_rules() {
        let rules = RuleSet::parse(
            "! comment\n||ads.example.com^$third-party\n@@||safe.ads.example.com^\n0.0.0.0 tracker.test\n",
        );
        assert_eq!(rules.len(), 3);
        assert!(rules.blocks_host("cdn.ads.example.com", &HashSet::new()));
        assert!(!rules.blocks_host("safe.ads.example.com", &HashSet::new()));
        assert!(rules.blocks_host("tracker.test", &HashSet::new()));
    }

    #[test]
    fn toggle_and_user_whitelist_are_authoritative() {
        let engine = AdblockEngine::new(true, &["doubleclick.net".to_owned()]);
        assert!(!engine.check_url("http://ads.doubleclick.net/pagead.js"));
        engine.set_whitelist(&[]);
        assert!(engine.check_url("http://ads.doubleclick.net/pagead.js"));
        engine.set_enabled(false);
        assert!(!engine.check_url("http://ads.doubleclick.net/pagead.js"));
    }

    #[test]
    fn only_sinkholed_requests_increment_stats() {
        let engine = AdblockEngine::new(true, &[]);
        assert!(engine.check_url("https://doubleclick.net/ad"));
        assert_eq!(engine.stats().total_blocked, 0);
        engine.record_block();
        assert_eq!(engine.stats().total_blocked, 1);
    }

    #[test]
    #[ignore = "requires live EasyList access"]
    fn live_easylist_contains_more_than_thirty_thousand_supported_rules() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime should build");
        let source = runtime.block_on(async {
            reqwest::get(DEFAULT_FILTER_LIST)
                .await
                .expect("EasyList should respond")
                .error_for_status()
                .expect("EasyList should return a successful status")
                .text()
                .await
                .expect("EasyList should contain text")
        });
        assert!(RuleSet::parse(&source).len() > 30_000);
    }
}
