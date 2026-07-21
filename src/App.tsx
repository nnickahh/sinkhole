import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  Activity,
  Ban,
  Check,
  Clock3,
  Copy,
  ExternalLink,
  Fingerprint,
  ListFilter,
  Orbit,
  Power,
  Radio,
  RefreshCw,
  Satellite,
  Settings as SettingsIcon,
  ShieldCheck,
  Sparkles,
  Trash2,
  Unplug,
  Wifi,
  Zap,
} from "lucide-react";
import { Settings } from "./components/Settings";
import { QuickPanel } from "./components/QuickPanel";
import type { AdblockStats, AppSettings, LinkInspection, RuleCategory } from "./types";
import "./App.css";

const DEFAULT_LISTS = [
  "https://easylist.to/easylist/easylist.txt",
  "https://easylist.to/easylist/easyprivacy.txt",
  "https://raw.githubusercontent.com/hagezi/dns-blocklists/main/adblock/native.winoffice.txt",
];

const EMPTY_STATS: AdblockStats = {
  totalBlocked: 0,
  blockedAds: 0,
  blockedTrackers: 0,
  blockedTelemetry: 0,
  blockedCustom: 0,
  activeRules: 0,
  activeAdRules: 0,
  activeTrackerRules: 0,
  activeTelemetryRules: 0,
  activeCustomRules: 0,
  activeLists: 0,
  protectionEnabled: true,
  proxyRunning: false,
  proxyAddress: "127.0.0.1:8118",
  requestsProcessed: 0,
  lastRequestAgeSeconds: null,
  uptimeSeconds: 0,
  bandwidthSavedBytes: 0,
  recentBlocks: [],
};

const EMPTY_SETTINGS: AppSettings = {
  protectionEnabled: true,
  systemProxyEnabled: false,
  filterListUrls: DEFAULT_LISTS,
  whitelistDomains: [],
};

const TEST_TARGETS = [
  { label: "Ad", url: "http://ads.doubleclick.net/pagead.js" },
  { label: "Tracker", url: "https://www.google-analytics.com/collect" },
  { label: "Telemetry", url: "https://vortex.data.microsoft.com/collect" },
];

const IS_TAURI = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

function formatNumber(value: number) {
  return new Intl.NumberFormat().format(value);
}

function formatBytes(value: number) {
  if (value < 1024) return `${value} B`;
  const units = ["KB", "MB", "GB"];
  let size = value / 1024;
  let unit = units[0];
  for (const nextUnit of units.slice(1)) {
    if (size < 1024) break;
    size /= 1024;
    unit = nextUnit;
  }
  return `${size.toFixed(size >= 10 ? 0 : 1)} ${unit}`;
}

function formatUptime(seconds: number) {
  if (seconds < 60) return `${seconds}s`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m`;
  return `${Math.floor(seconds / 3600)}h ${Math.floor((seconds % 3600) / 60)}m`;
}

function formatAge(seconds: number) {
  if (seconds < 5) return "now";
  if (seconds < 60) return `${seconds}s ago`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
  return `${Math.floor(seconds / 3600)}h ago`;
}

function categoryTone(category: RuleCategory) {
  return {
    ad: "border-violet-300/20 bg-violet-400/10 text-violet-200",
    tracker: "border-cyan-300/20 bg-cyan-400/10 text-cyan-200",
    telemetry: "border-rose-300/20 bg-rose-400/10 text-rose-200",
    custom: "border-amber-300/20 bg-amber-400/10 text-amber-200",
  }[category];
}

function useOrbitDeceleration(active: boolean) {
  const ringRef = useRef<HTMLSpanElement>(null);
  const animationRef = useRef<Animation | null>(null);
  const frameRef = useRef<number | null>(null);
  const playbackRateRef = useRef(active ? 1 : 0);
  const activeRef = useRef(active);
  const reducedMotionRef = useRef(false);

  useEffect(() => {
    activeRef.current = active;
  }, [active]);

  useEffect(() => {
    const ring = ringRef.current;
    if (!ring) return;

    const motionPreference = window.matchMedia("(prefers-reduced-motion: reduce)");
    const animation = ring.animate(
      [{ transform: "rotate(0deg)" }, { transform: "rotate(360deg)" }],
      { duration: 9000, iterations: Infinity, easing: "linear" },
    );
    animationRef.current = animation;

    const applyMotionPreference = () => {
      reducedMotionRef.current = motionPreference.matches;
      if (frameRef.current !== null) window.cancelAnimationFrame(frameRef.current);
      const rate = motionPreference.matches || !activeRef.current ? 0 : 1;
      playbackRateRef.current = rate;
      animation.updatePlaybackRate(rate || 0.001);
      if (rate === 0) animation.pause();
      else animation.play();
    };

    applyMotionPreference();
    motionPreference.addEventListener("change", applyMotionPreference);
    return () => {
      motionPreference.removeEventListener("change", applyMotionPreference);
      if (frameRef.current !== null) window.cancelAnimationFrame(frameRef.current);
      animation.cancel();
      animationRef.current = null;
    };
  }, []);

  useEffect(() => {
    const animation = animationRef.current;
    if (!animation || reducedMotionRef.current) return;
    if (frameRef.current !== null) window.cancelAnimationFrame(frameRef.current);

    const startRate = playbackRateRef.current;
    const targetRate = active ? 1 : 0;
    const duration = active ? 420 : 1250;
    const startedAt = performance.now();
    if (targetRate > 0) animation.play();

    const step = (now: number) => {
      const progress = Math.min((now - startedAt) / duration, 1);
      const eased = 1 - Math.pow(1 - progress, 3);
      const nextRate = startRate + (targetRate - startRate) * eased;
      playbackRateRef.current = nextRate;
      animation.updatePlaybackRate(Math.max(nextRate, 0.001));

      if (progress < 1) {
        frameRef.current = window.requestAnimationFrame(step);
      } else {
        playbackRateRef.current = targetRate;
        animation.updatePlaybackRate(targetRate || 0.001);
        if (targetRate === 0) animation.pause();
        frameRef.current = null;
      }
    };

    frameRef.current = window.requestAnimationFrame(step);
    return () => {
      if (frameRef.current !== null) window.cancelAnimationFrame(frameRef.current);
    };
  }, [active]);

  return ringRef;
}

function Dashboard() {
  const [stats, setStats] = useState(EMPTY_STATS);
  const [settings, setSettings] = useState(EMPTY_SETTINGS);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [updating, setUpdating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [testUrl, setTestUrl] = useState(TEST_TARGETS[0].url);
  const [testResult, setTestResult] = useState<LinkInspection | null>(null);
  const [copied, setCopied] = useState(false);
  const [clearingLog, setClearingLog] = useState(false);
  const ringRef = useOrbitDeceleration(stats.protectionEnabled);

  const refreshStats = useCallback(async () => {
    try {
      setStats(await invoke<AdblockStats>("get_stats"));
    } catch (cause) {
      setError(String(cause));
    }
  }, []);

  useEffect(() => {
    if (!IS_TAURI) return;
    void Promise.all([
      refreshStats(),
      invoke<AppSettings>("get_settings").then(setSettings),
    ]).catch((cause) => setError(String(cause)));
    const timer = window.setInterval(() => void refreshStats(), 1500);
    return () => window.clearInterval(timer);
  }, [refreshStats]);

  const toggleProtection = async () => {
    setBusy(true);
    setError(null);
    try {
      const enabled = await invoke<boolean>("toggle_adblocker", {
        enable: !stats.protectionEnabled,
      });
      setSettings((current) => ({ ...current, protectionEnabled: enabled }));
      await refreshStats();
    } catch (cause) {
      setError(String(cause));
    } finally {
      setBusy(false);
    }
  };

  const toggleBrowserConnection = async () => {
    setBusy(true);
    setError(null);
    try {
      const saved = await invoke<AppSettings>("set_system_proxy", {
        enable: !settings.systemProxyEnabled,
      });
      setSettings(saved);
      await refreshStats();
    } catch (cause) {
      setError(String(cause));
    } finally {
      setBusy(false);
    }
  };

  const saveSettings = async (draft: AppSettings) => {
    setBusy(true);
    setError(null);
    try {
      const saved = await invoke<AppSettings>("update_settings", { settings: draft });
      setSettings(saved);
      await refreshStats();
      setSettingsOpen(false);
    } catch (cause) {
      setError(String(cause));
    } finally {
      setBusy(false);
    }
  };

  const updateRules = async (draft: AppSettings) => {
    setUpdating(true);
    setError(null);
    try {
      const saved = await invoke<AppSettings>("update_settings", { settings: draft });
      setSettings(saved);
      await invoke<number>("update_blocklists");
      await refreshStats();
    } catch (cause) {
      setError(String(cause));
    } finally {
      setUpdating(false);
    }
  };

  const checkLink = async (url = testUrl) => {
    setError(null);
    setTestUrl(url);
    try {
      setTestResult(await invoke<LinkInspection>("inspect_link", { url }));
    } catch (cause) {
      setError(String(cause));
    }
  };

  const copyProxy = async () => {
    await navigator.clipboard.writeText(stats.proxyAddress);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1500);
  };

  const clearActivityLog = async () => {
    setClearingLog(true);
    setError(null);
    try {
      await invoke<number>("clear_activity_log");
      await refreshStats();
    } catch (cause) {
      setError(String(cause));
    } finally {
      setClearingLog(false);
    }
  };

  const connected = settings.systemProxyEnabled && stats.proxyRunning;
  const observed = connected && stats.requestsProcessed > 0;
  const active = stats.protectionEnabled;
  const statusLabel = !stats.proxyRunning
    ? "PROXY STARTING"
    : !connected
      ? "BROWSER NOT CONNECTED"
      : observed
        ? "TRAFFIC CAPTURED"
        : "CONNECTED — WAITING FOR TRAFFIC";

  return (
    <main className="cosmos relative min-h-screen overflow-x-hidden text-slate-100">
      <div className="stars pointer-events-none absolute inset-0" />
      <div className="nebula pointer-events-none absolute inset-0" />

      <div className="relative mx-auto flex min-h-screen max-w-[1480px] flex-col px-5 py-5 sm:px-8">
        <header className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="brand-orbit relative grid h-12 w-12 place-items-center rounded-2xl border border-violet-300/25 bg-violet-500/10 text-violet-200">
              <Orbit size={27} strokeWidth={1.7} />
              <span className="absolute right-1 top-1 h-2 w-2 rounded-full bg-cyan-300 shadow-[0_0_14px_#67e8f9]" />
            </div>
            <div>
              <h1 className="text-xl font-semibold tracking-[-0.03em] text-white">SinkHole</h1>
              <p className="text-[10px] font-medium tracking-[0.28em] text-violet-300/70">LOCAL PRIVACY GRAVITY</p>
            </div>
          </div>

          <div className="flex items-center gap-3">
            <div className="hidden items-center gap-2 rounded-full border border-white/10 bg-[#0b0820]/70 px-4 py-2 text-xs text-slate-400 sm:flex">
              <span className={`h-1.5 w-1.5 rounded-full ${connected ? "bg-cyan-300 shadow-[0_0_10px_#67e8f9]" : "bg-amber-300"}`} />
              {statusLabel}
            </div>
            <button
              aria-label="Open settings"
              className="rounded-xl border border-white/10 bg-white/[0.04] p-2.5 text-slate-400 transition hover:border-violet-300/30 hover:bg-violet-400/10 hover:text-white"
              onClick={() => setSettingsOpen(true)}
            >
              <SettingsIcon size={20} />
            </button>
          </div>
        </header>

        {error && (
          <div role="alert" className="mt-5 flex items-center justify-between rounded-xl border border-rose-400/25 bg-rose-400/10 px-4 py-3 text-sm text-rose-100">
            <span className="truncate">{error}</span>
            <button className="ml-4 text-rose-300 hover:text-white" onClick={() => setError(null)}>Dismiss</button>
          </div>
        )}

        <section className="grid flex-1 items-center gap-8 py-7 lg:grid-cols-[0.92fr_1.08fr]">
          <div className="flex flex-col items-center justify-center lg:items-start">
            <div className="mb-7 flex items-center gap-2 rounded-full border border-white/10 bg-[#09061a]/70 px-3 py-1.5 text-xs font-semibold tracking-wider text-slate-400">
              <span className={`h-1.5 w-1.5 rounded-full ${active ? "bg-violet-300 status-pulse" : "bg-slate-600"}`} />
              {active ? "FILTER ENGINE ONLINE" : "FILTER ENGINE PAUSED"}
            </div>

            <button
              aria-label={active ? "Disable filtering" : "Enable filtering"}
              aria-pressed={active}
              aria-busy={busy}
              className={`event-horizon group relative grid h-52 w-52 place-items-center rounded-full ${active ? "is-active" : "is-paused"} ${busy ? "is-busy" : ""}`}
              disabled={busy}
              onClick={toggleProtection}
            >
              <span ref={ringRef} aria-hidden="true" className="accretion-ring absolute inset-2 rounded-full" />
              <span className="black-hole grid h-28 w-28 place-items-center rounded-full border border-violet-200/20 text-violet-100">
                {busy ? <RefreshCw aria-hidden="true" className="animate-spin" size={36} /> : <Power aria-hidden="true" size={38} strokeWidth={1.6} />}
              </span>
            </button>

            <div className="mt-8 text-center lg:text-left">
              <h2 className="max-w-xl text-4xl font-semibold tracking-[-0.045em] text-white">
                {active ? "Pull invasive traffic out of orbit." : "The event horizon is dormant."}
              </h2>
              <p className="mt-3 max-w-lg leading-7 text-slate-400">
                {active
                  ? "Known ads, trackers, and Windows telemetry hosts are matched locally. HTTPS stays encrypted; SinkHole only sees the destination hostname."
                  : "Traffic can still pass through the proxy, but no hosts are blocked until the filter engine is online."}
              </p>
            </div>

            <div className={`mt-6 w-full max-w-lg rounded-2xl border p-4 ${connected ? "border-cyan-300/20 bg-cyan-300/[0.055]" : "border-amber-300/20 bg-amber-300/[0.055]"}`}>
              <div className="flex flex-wrap items-center justify-between gap-3">
                <div className="flex items-center gap-3">
                  <div className={`rounded-xl p-2.5 ${connected ? "bg-cyan-300/10 text-cyan-300" : "bg-amber-300/10 text-amber-300"}`}>
                    {connected ? <Satellite size={19} /> : <Unplug size={19} />}
                  </div>
                  <div>
                    <p className="text-sm font-medium text-white">{connected ? "Windows browser connected" : "Connect browser traffic"}</p>
                    <p className="mt-0.5 text-xs text-slate-500">{connected ? `${formatNumber(stats.requestsProcessed)} requests observed` : "Nothing is filtered until traffic uses the local proxy."}</p>
                  </div>
                </div>
                <button
                  className={`rounded-xl px-4 py-2.5 text-xs font-bold transition ${connected ? "border border-white/10 bg-white/5 text-slate-200 hover:bg-white/10" : "bg-violet-300 text-[#110a2c] hover:bg-violet-200"}`}
                  disabled={busy || !stats.proxyRunning}
                  onClick={toggleBrowserConnection}
                >
                  {connected ? "Disconnect" : "Connect Windows"}
                </button>
              </div>
              {connected && (
                <button className="mt-3 flex items-center gap-1.5 text-xs font-medium text-cyan-300 hover:text-cyan-200" onClick={() => openUrl("http://sinkhole.test")}>
                  <ExternalLink size={13} /> Open connection test
                </button>
              )}
            </div>
          </div>

          <div className="space-y-4">
            <article className="cosmic-card rounded-2xl p-5">
              <div className="flex items-start justify-between gap-4">
                <div>
                  <p className="text-sm text-slate-400">Objects sent into the void</p>
                  <p className="mt-2 text-5xl font-semibold tracking-[-0.05em] text-white">{formatNumber(stats.totalBlocked)}</p>
                </div>
                <div className="rounded-xl bg-violet-400/10 p-2.5 text-violet-300"><Ban size={20} /></div>
              </div>
              <div className="mt-5 grid grid-cols-3 gap-2">
                <StatPill icon={<Zap size={14} />} label="Ads" value={stats.blockedAds} tone="violet" />
                <StatPill icon={<Fingerprint size={14} />} label="Trackers" value={stats.blockedTrackers} tone="cyan" />
                <StatPill icon={<Radio size={14} />} label="Telemetry" value={stats.blockedTelemetry} tone="rose" />
              </div>
            </article>

            <div className="grid gap-4 sm:grid-cols-3">
              <Metric icon={<ListFilter size={18} />} label="Privacy rules" value={formatNumber(stats.activeRules)} detail={`${stats.activeLists} live lists`} />
              <Metric icon={<Wifi size={18} />} label="Requests seen" value={formatNumber(stats.requestsProcessed)} detail={stats.lastRequestAgeSeconds === null ? "No traffic yet" : `${stats.lastRequestAgeSeconds}s ago`} />
              <Metric icon={<Clock3 size={18} />} label="Data avoided" value={formatBytes(stats.bandwidthSavedBytes)} detail={`Up ${formatUptime(stats.uptimeSeconds)}`} />
            </div>

            <article className="cosmic-card rounded-2xl p-5">
              <div className="flex items-start justify-between gap-4">
                <div>
                  <div className="flex items-center gap-2 text-sm font-medium text-white">
                    <Activity aria-hidden="true" className="text-cyan-300" size={17} /> Local activity
                  </div>
                  <p className="mt-1 text-xs text-slate-500">Hostnames only · memory only · newest first</p>
                </div>
                <button
                  aria-label="Clear local activity log"
                  className="flex items-center gap-1.5 rounded-lg border border-white/10 bg-white/[0.035] px-2.5 py-1.5 text-[11px] text-slate-400 hover:border-rose-300/30 hover:text-rose-200 disabled:opacity-40"
                  disabled={clearingLog || stats.recentBlocks.length === 0}
                  onClick={() => void clearActivityLog()}
                >
                  <Trash2 aria-hidden="true" size={12} /> {clearingLog ? "Clearing" : "Clear"}
                </button>
              </div>

              {stats.recentBlocks.length > 0 ? (
                <ol aria-label="Recently blocked hostnames" className="activity-list mt-4 max-h-44 space-y-1.5 overflow-y-auto pr-1">
                  {stats.recentBlocks.map((event) => (
                    <li key={`${event.category}:${event.host}`} className="activity-row grid grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-2.5 rounded-xl border border-white/[0.06] bg-black/15 px-3 py-2">
                      <span className={`rounded-md border px-1.5 py-0.5 text-[9px] font-bold uppercase tracking-wider ${categoryTone(event.category)}`}>
                        {event.category}
                      </span>
                      <span className="truncate font-mono text-[11px] text-slate-300" title={event.host}>{event.host}</span>
                      <span className="flex items-center gap-2 text-[10px] text-slate-600">
                        {event.count > 1 && <span aria-label={`${event.count} blocks`}>×{formatNumber(event.count)}</span>}
                        <time>{formatAge(event.ageSeconds)}</time>
                      </span>
                    </li>
                  ))}
                </ol>
              ) : (
                <div className="mt-4 rounded-xl border border-dashed border-white/10 px-4 py-5 text-center text-xs text-slate-600">
                  Blocked destinations will appear here without URLs, payloads, or browsing history.
                </div>
              )}
            </article>

            <article className="cosmic-card rounded-2xl p-5">
              <div className="flex flex-wrap items-center justify-between gap-3">
                <div>
                  <div className="flex items-center gap-2 text-sm font-medium text-white"><Sparkles className="text-violet-300" size={17} /> Rule scanner</div>
                  <p className="mt-1 text-xs text-slate-500">Preview why a destination would be blocked.</p>
                </div>
                <div className="flex gap-1.5">
                  {TEST_TARGETS.map((target) => (
                    <button key={target.label} className="rounded-lg border border-white/10 bg-white/[0.035] px-2.5 py-1.5 text-[11px] text-slate-400 hover:border-violet-300/30 hover:text-violet-200" onClick={() => void checkLink(target.url)}>{target.label}</button>
                  ))}
                </div>
              </div>
              <div className="mt-4 flex gap-2">
                <input
                  aria-label="URL to test"
                  className="min-w-0 flex-1 rounded-xl border border-white/10 bg-black/20 px-4 py-3 text-sm text-slate-200 outline-none placeholder:text-slate-600 focus:border-violet-400/50"
                  onChange={(event) => { setTestUrl(event.target.value); setTestResult(null); }}
                  onKeyDown={(event) => { if (event.key === "Enter") void checkLink(); }}
                  value={testUrl}
                />
                <button className="rounded-xl bg-white px-5 text-sm font-semibold text-slate-950 transition hover:bg-violet-100" onClick={() => void checkLink()}>Scan</button>
              </div>
              {testResult && (
                <p role="status" className={`mt-3 flex items-center gap-2 text-xs ${testResult.blocked ? "text-violet-200" : "text-slate-400"}`}>
                  {testResult.blocked ? <Activity size={14} /> : <Check size={14} />}
                  {testResult.blocked ? `Blocked as ${testResult.category}` : "Allowed by current settings"}
                </p>
              )}
            </article>

            <article className="rounded-2xl border border-white/[0.07] bg-black/15 px-5 py-4">
              <div className="flex flex-wrap items-center justify-between gap-3">
                <div className="flex items-center gap-3">
                  <ShieldCheck className="text-slate-500" size={18} />
                  <div><p className="text-xs text-slate-400">Manual proxy</p><p className="font-mono text-xs text-slate-600">{stats.proxyAddress}</p></div>
                </div>
                <button className="flex items-center gap-2 text-xs text-slate-500 hover:text-white" onClick={copyProxy}>{copied ? <Check size={13} /> : <Copy size={13} />}{copied ? "Copied" : "Copy"}</button>
              </div>
            </article>
          </div>
        </section>

        <footer className="flex items-center justify-between border-t border-white/[0.06] pt-4 text-[11px] text-slate-600">
          <span>SinkHole 0.1.0 · local-first</span>
          <span>Blocks known endpoints; it cannot stop local data collection.</span>
        </footer>
      </div>

      {settingsOpen && (
        <Settings onClose={() => setSettingsOpen(false)} onSave={saveSettings} onUpdate={updateRules} saving={busy} settings={settings} updating={updating} />
      )}
    </main>
  );
}

function StatPill({ icon, label, value, tone }: { icon: React.ReactNode; label: string; value: number; tone: "violet" | "cyan" | "rose" }) {
  const tones = { violet: "border-violet-300/15 bg-violet-400/[0.06] text-violet-200", cyan: "border-cyan-300/15 bg-cyan-400/[0.06] text-cyan-200", rose: "border-rose-300/15 bg-rose-400/[0.06] text-rose-200" };
  return <div className={`rounded-xl border px-3 py-2.5 ${tones[tone]}`}><div className="flex items-center gap-1.5 text-[11px] opacity-70">{icon}{label}</div><p className="mt-1 text-lg font-semibold text-white">{formatNumber(value)}</p></div>;
}

function Metric({ icon, label, value, detail }: { icon: React.ReactNode; label: string; value: string; detail: string }) {
  return <article className="cosmic-card rounded-2xl p-4"><div className="text-violet-300">{icon}</div><p className="mt-3 text-xs text-slate-500">{label}</p><p className="mt-1 text-xl font-semibold text-white">{value}</p><p className="mt-1 text-[11px] text-slate-600">{detail}</p></article>;
}

function App() {
  return IS_TAURI && getCurrentWindow().label === "quick" ? <QuickPanel /> : <Dashboard />;
}

export default App;
