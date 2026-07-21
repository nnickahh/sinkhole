import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Activity,
  Check,
  Clock3,
  Copy,
  EyeOff,
  Gauge,
  ListFilter,
  LockKeyhole,
  Power,
  RefreshCw,
  Settings as SettingsIcon,
  ShieldCheck,
  Wifi,
  Zap,
} from "lucide-react";
import { Settings } from "./components/Settings";
import type { AdblockStats, AppSettings } from "./types";
import "./App.css";

const EMPTY_STATS: AdblockStats = {
  totalBlocked: 0,
  activeRules: 0,
  activeLists: 0,
  protectionEnabled: true,
  proxyRunning: false,
  proxyAddress: "127.0.0.1:8118",
  uptimeSeconds: 0,
  bandwidthSavedBytes: 0,
};

const EMPTY_SETTINGS: AppSettings = {
  protectionEnabled: true,
  filterListUrls: ["https://easylist.to/easylist/easylist.txt"],
  whitelistDomains: [],
};

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

function App() {
  const [stats, setStats] = useState(EMPTY_STATS);
  const [settings, setSettings] = useState(EMPTY_SETTINGS);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [updating, setUpdating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [testUrl, setTestUrl] = useState("http://ads.doubleclick.net/pagead.js");
  const [testResult, setTestResult] = useState<boolean | null>(null);
  const [copied, setCopied] = useState(false);

  const refreshStats = useCallback(async () => {
    try {
      setStats(await invoke<AdblockStats>("get_stats"));
    } catch (cause) {
      setError(String(cause));
    }
  }, []);

  useEffect(() => {
    void Promise.all([
      refreshStats(),
      invoke<AppSettings>("get_settings").then(setSettings),
    ]).catch((cause) => setError(String(cause)));
    const timer = window.setInterval(() => void refreshStats(), 2000);
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

  const checkLink = async () => {
    setError(null);
    try {
      setTestResult(await invoke<boolean>("check_link", { url: testUrl }));
    } catch (cause) {
      setError(String(cause));
    }
  };

  const copyProxy = async () => {
    await navigator.clipboard.writeText(stats.proxyAddress);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1500);
  };

  const active = stats.protectionEnabled;

  return (
    <main className="relative min-h-screen overflow-hidden bg-[#06100d] text-slate-100">
      <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_72%_8%,rgba(16,185,129,0.13),transparent_35%),radial-gradient(circle_at_10%_90%,rgba(14,116,144,0.12),transparent_30%)]" />
      <div className="pointer-events-none absolute inset-0 opacity-[0.035] app-grid" />

      <div className="relative mx-auto flex min-h-screen max-w-[1440px] flex-col px-8 py-6">
        <header className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="relative grid h-11 w-11 place-items-center rounded-2xl border border-emerald-300/20 bg-emerald-400/10 text-emerald-300 shadow-glow">
              <ShieldCheck size={24} strokeWidth={1.8} />
              <span className="absolute -right-0.5 -top-0.5 h-2.5 w-2.5 rounded-full border-2 border-[#06100d] bg-emerald-400" />
            </div>
            <div>
              <h1 className="text-lg font-semibold tracking-tight text-white">SinkHole</h1>
              <p className="text-xs tracking-wide text-slate-500">LOCAL PRIVACY NETWORK</p>
            </div>
          </div>

          <div className="flex items-center gap-3">
            <div className="hidden items-center gap-2 rounded-full border border-white/10 bg-white/[0.035] px-4 py-2 text-xs text-slate-400 sm:flex">
              <span className={`h-1.5 w-1.5 rounded-full ${stats.proxyRunning ? "bg-emerald-400" : "bg-amber-400"}`} />
              Proxy {stats.proxyRunning ? "listening" : "starting"}
            </div>
            <button
              aria-label="Open settings"
              className="rounded-xl border border-white/10 bg-white/[0.035] p-2.5 text-slate-400 transition hover:border-white/20 hover:bg-white/[0.07] hover:text-white"
              onClick={() => setSettingsOpen(true)}
            >
              <SettingsIcon size={20} />
            </button>
          </div>
        </header>

        {error && (
          <div className="mt-5 flex items-center justify-between rounded-xl border border-rose-400/20 bg-rose-400/10 px-4 py-3 text-sm text-rose-200">
            <span className="truncate">{error}</span>
            <button className="ml-4 text-rose-300 hover:text-white" onClick={() => setError(null)}>
              Dismiss
            </button>
          </div>
        )}

        <section className="grid flex-1 items-center gap-8 py-8 lg:grid-cols-[1.05fr_1fr]">
          <div className="flex flex-col items-center justify-center lg:items-start">
            <div className="mb-8 flex items-center gap-2 rounded-full border border-white/10 bg-black/20 px-3 py-1.5 text-xs font-medium text-slate-400">
              <span className={`h-1.5 w-1.5 rounded-full ${active ? "bg-emerald-400 status-pulse" : "bg-slate-500"}`} />
              {active ? "PROTECTION ACTIVE" : "PROTECTION PAUSED"}
            </div>

            <button
              aria-label={active ? "Disable protection" : "Enable protection"}
              className={`power-orb group relative grid h-52 w-52 place-items-center rounded-full border transition-all duration-500 ${
                active
                  ? "border-emerald-300/40 bg-emerald-400/[0.08] shadow-[0_0_100px_rgba(16,185,129,0.22)]"
                  : "border-white/10 bg-white/[0.025]"
              } ${busy ? "scale-95 opacity-70" : "hover:scale-[1.02]"}`}
              disabled={busy}
              onClick={toggleProtection}
            >
              <span className={`absolute inset-4 rounded-full border ${active ? "border-emerald-300/15" : "border-white/5"}`} />
              <span className={`grid h-24 w-24 place-items-center rounded-full border transition ${active ? "border-emerald-300/30 bg-emerald-400/15 text-emerald-300" : "border-white/10 bg-white/5 text-slate-500"}`}>
                {busy ? <RefreshCw className="animate-spin" size={36} /> : <Power size={38} strokeWidth={1.7} />}
              </span>
            </button>

            <div className="mt-8 text-center lg:text-left">
              <h2 className="text-4xl font-semibold tracking-[-0.04em] text-white">
                {active ? "Your connection is shielded" : "Protection is paused"}
              </h2>
              <p className="mt-3 max-w-lg leading-7 text-slate-400">
                {active
                  ? "Requests are checked locally against your active privacy rules. Nothing leaves this device for analysis."
                  : "Traffic passes through without filtering. Turn protection on when you are ready."}
              </p>
            </div>
          </div>

          <div className="space-y-4">
            <div className="grid gap-4 sm:grid-cols-2">
              <article className="rounded-2xl border border-white/10 bg-white/[0.035] p-5 backdrop-blur-xl">
                <div className="flex items-center justify-between">
                  <span className="text-sm text-slate-400">Ads & trackers blocked</span>
                  <EyeOff className="text-emerald-400" size={19} />
                </div>
                <p className="mt-5 text-4xl font-semibold tracking-tight text-white">{formatNumber(stats.totalBlocked)}</p>
                <p className="mt-2 text-xs text-slate-500">This session</p>
              </article>

              <article className="rounded-2xl border border-white/10 bg-white/[0.035] p-5 backdrop-blur-xl">
                <div className="flex items-center justify-between">
                  <span className="text-sm text-slate-400">Active filter rules</span>
                  <ListFilter className="text-sky-400" size={19} />
                </div>
                <p className="mt-5 text-4xl font-semibold tracking-tight text-white">{formatNumber(stats.activeRules)}</p>
                <p className="mt-2 text-xs text-slate-500">Across {stats.activeLists} source{stats.activeLists === 1 ? "" : "s"}</p>
              </article>

              <article className="rounded-2xl border border-white/10 bg-white/[0.035] p-5 backdrop-blur-xl">
                <div className="flex items-center justify-between">
                  <span className="text-sm text-slate-400">Estimated data saved</span>
                  <Zap className="text-amber-400" size={19} />
                </div>
                <p className="mt-5 text-3xl font-semibold tracking-tight text-white">{formatBytes(stats.bandwidthSavedBytes)}</p>
                <p className="mt-2 flex items-center gap-1.5 text-xs text-slate-500"><Clock3 size={13} /> Up for {formatUptime(stats.uptimeSeconds)}</p>
              </article>

              <article className="rounded-2xl border border-white/10 bg-white/[0.035] p-5 backdrop-blur-xl">
                <div className="flex items-center justify-between">
                  <span className="text-sm text-slate-400">Filter latency</span>
                  <Gauge className="text-violet-400" size={19} />
                </div>
                <p className="mt-5 text-3xl font-semibold tracking-tight text-white">&lt; 1 ms</p>
                <p className="mt-2 text-xs text-slate-500">In-memory domain index</p>
              </article>
            </div>

            <article className="rounded-2xl border border-white/10 bg-black/20 p-5">
              <div className="flex flex-wrap items-center justify-between gap-4">
                <div className="flex items-center gap-3">
                  <div className="rounded-xl bg-emerald-400/10 p-2.5 text-emerald-400"><Wifi size={19} /></div>
                  <div>
                    <p className="text-sm font-medium text-white">Local proxy endpoint</p>
                    <p className="mt-0.5 font-mono text-xs text-slate-500">{stats.proxyAddress}</p>
                  </div>
                </div>
                <button
                  className="flex items-center gap-2 rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-xs text-slate-300 transition hover:bg-white/10"
                  onClick={copyProxy}
                >
                  {copied ? <Check size={14} /> : <Copy size={14} />}
                  {copied ? "Copied" : "Copy address"}
                </button>
              </div>
            </article>

            <article className="rounded-2xl border border-white/10 bg-black/20 p-5">
              <div className="mb-3 flex items-center gap-2 text-sm font-medium text-white">
                <LockKeyhole className="text-slate-400" size={17} />
                Test a request
              </div>
              <div className="flex gap-2">
                <input
                  aria-label="URL to test"
                  className="min-w-0 flex-1 rounded-xl border border-white/10 bg-white/[0.035] px-4 py-3 text-sm text-slate-200 outline-none placeholder:text-slate-600 focus:border-emerald-400/40"
                  onChange={(event) => {
                    setTestUrl(event.target.value);
                    setTestResult(null);
                  }}
                  onKeyDown={(event) => {
                    if (event.key === "Enter") void checkLink();
                  }}
                  value={testUrl}
                />
                <button className="rounded-xl bg-white px-5 text-sm font-semibold text-slate-950 transition hover:bg-emerald-100" onClick={checkLink}>
                  Check
                </button>
              </div>
              {testResult !== null && (
                <p className={`mt-3 flex items-center gap-2 text-xs ${testResult ? "text-emerald-400" : "text-slate-400"}`}>
                  {testResult ? <Activity size={14} /> : <Check size={14} />}
                  {testResult ? "Blocked by an active rule" : "Allowed by current settings"}
                </p>
              )}
            </article>
          </div>
        </section>

        <footer className="flex items-center justify-between border-t border-white/[0.07] pt-5 text-xs text-slate-600">
          <span>SinkHole 0.1.0</span>
          <span className="flex items-center gap-1.5"><ShieldCheck size={13} /> Processing stays on device</span>
        </footer>
      </div>

      {settingsOpen && (
        <Settings
          onClose={() => setSettingsOpen(false)}
          onSave={saveSettings}
          onUpdate={updateRules}
          saving={busy}
          settings={settings}
          updating={updating}
        />
      )}
    </main>
  );
}

export default App;
