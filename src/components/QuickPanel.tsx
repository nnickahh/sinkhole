import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  Activity,
  ArrowUpRight,
  Orbit,
  Power,
  RefreshCw,
  Settings,
  X,
} from "lucide-react";
import type { AdblockStats, AppSettings } from "../types";

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
  protectionEnabled: false,
  proxyRunning: false,
  proxyAddress: "127.0.0.1:8118",
  requestsProcessed: 0,
  lastRequestAgeSeconds: null,
  uptimeSeconds: 0,
  bandwidthSavedBytes: 0,
};

const EMPTY_SETTINGS: AppSettings = {
  protectionEnabled: false,
  systemProxyEnabled: false,
  filterListUrls: [],
  whitelistDomains: [],
};

export function QuickPanel() {
  const [stats, setStats] = useState(EMPTY_STATS);
  const [settings, setSettings] = useState(EMPTY_SETTINGS);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    const [nextStats, nextSettings] = await Promise.all([
      invoke<AdblockStats>("get_stats"),
      invoke<AppSettings>("get_settings"),
    ]);
    setStats(nextStats);
    setSettings(nextSettings);
  }, []);

  useEffect(() => {
    void refresh().catch((cause) => setError(String(cause)));
    const timer = window.setInterval(() => void refresh(), 1500);
    return () => window.clearInterval(timer);
  }, [refresh]);

  const connected = stats.proxyRunning && settings.systemProxyEnabled;
  const enabled = connected && stats.protectionEnabled;
  const observed = enabled && stats.requestsProcessed > 0;

  const toggle = async () => {
    setBusy(true);
    setError(null);
    try {
      const saved = await invoke<AppSettings>("set_quick_protection", {
        enable: !enabled,
      });
      setSettings(saved);
      await refresh();
    } catch (cause) {
      setError(String(cause));
    } finally {
      setBusy(false);
    }
  };

  const openDashboard = async () => {
    await invoke("open_dashboard");
  };

  return (
    <main className="quick-cosmos relative flex h-screen flex-col overflow-hidden border border-violet-300/20 text-slate-100">
      <div className="stars pointer-events-none absolute inset-0" />
      <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_50%_40%,rgba(109,40,217,0.2),transparent_48%)]" />

      <header className="relative flex items-center justify-between border-b border-white/[0.07] bg-white/[0.025] px-5 py-4">
        <div className="flex items-center gap-2.5">
          <div className="grid h-9 w-9 place-items-center rounded-xl border border-violet-300/20 bg-violet-400/10 text-violet-200">
            <Orbit size={21} />
          </div>
          <div>
            <p className="font-semibold leading-tight text-white">SinkHole</p>
            <p className="text-[9px] font-medium tracking-[0.22em] text-violet-300/65">QUICK CONTROL</p>
          </div>
        </div>
        <div className="flex gap-1">
          <button aria-label="Open dashboard" className="rounded-lg p-2 text-slate-500 hover:bg-white/5 hover:text-violet-200" onClick={openDashboard}><Settings size={18} /></button>
          <button aria-label="Close quick control" className="rounded-lg p-2 text-slate-500 hover:bg-white/5 hover:text-white" onClick={() => getCurrentWindow().hide()}><X size={18} /></button>
        </div>
      </header>

      <section className="relative flex flex-1 flex-col items-center justify-center px-7 pb-6 pt-5 text-center">
        <p className={`text-[11px] font-semibold tracking-[0.2em] ${enabled ? "text-cyan-300" : "text-slate-500"}`}>
          {enabled ? (observed ? "TRAFFIC CAPTURED" : "CONNECTED") : "DISCONNECTED"}
        </p>

        <button
          aria-label={enabled ? "Disconnect SinkHole" : "Connect SinkHole"}
          className={`quick-switch mt-5 flex h-16 w-32 items-center rounded-full p-1.5 transition-all duration-300 ${enabled ? "is-on" : "is-off"}`}
          disabled={busy || !stats.proxyRunning}
          onClick={toggle}
        >
          <span className={`grid h-[52px] w-[52px] place-items-center rounded-full bg-white text-[#100a24] shadow-xl transition-transform duration-300 ${enabled ? "translate-x-16" : "translate-x-0"}`}>
            {busy ? <RefreshCw className="animate-spin" size={22} /> : <Power size={22} />}
          </span>
        </button>

        <h1 className="mt-5 text-3xl font-semibold tracking-[-0.04em] text-white">
          {enabled ? "Event horizon online" : "Privacy orbit open"}
        </h1>
        <p className="mt-2 max-w-[280px] text-sm leading-6 text-slate-400">
          {enabled
            ? "Known ad, tracker, and telemetry hosts are being filtered locally."
            : "Switch on to connect Windows browser traffic and start filtering."}
        </p>

        <div className="mt-5 flex w-full items-center justify-between rounded-xl border border-white/[0.08] bg-white/[0.035] px-4 py-3 text-left">
          <div>
            <p className="text-[10px] uppercase tracking-wider text-slate-600">This session</p>
            <p className="mt-0.5 text-sm font-medium text-white">{stats.totalBlocked.toLocaleString()} blocked · {stats.requestsProcessed.toLocaleString()} seen</p>
          </div>
          <Activity className={observed ? "text-cyan-300" : "text-slate-600"} size={18} />
        </div>

        {error ? (
          <p className="mt-3 max-w-full truncate text-xs text-rose-300">{error}</p>
        ) : (
          <button className="mt-3 flex items-center gap-1 text-[11px] text-slate-600 hover:text-violet-200" onClick={openDashboard}>
            Open full dashboard <ArrowUpRight size={12} />
          </button>
        )}
      </section>
    </main>
  );
}
