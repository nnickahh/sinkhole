import { useEffect, useState } from "react";
import {
  Globe2,
  Plus,
  RefreshCw,
  Save,
  ShieldCheck,
  Trash2,
  X,
} from "lucide-react";
import type { AppSettings } from "../types";

interface SettingsProps {
  settings: AppSettings;
  saving: boolean;
  updating: boolean;
  onClose: () => void;
  onSave: (settings: AppSettings) => Promise<void>;
  onUpdate: (settings: AppSettings) => Promise<void>;
}

export function Settings({
  settings,
  saving,
  updating,
  onClose,
  onSave,
  onUpdate,
}: SettingsProps) {
  const [draft, setDraft] = useState(settings);
  const [newList, setNewList] = useState("");
  const [newDomain, setNewDomain] = useState("");

  useEffect(() => setDraft(settings), [settings]);

  const addList = () => {
    const value = newList.trim();
    if (!value || draft.filterListUrls.includes(value)) return;
    setDraft({ ...draft, filterListUrls: [...draft.filterListUrls, value] });
    setNewList("");
  };

  const addDomain = () => {
    const value = newDomain.trim().toLowerCase();
    if (!value || draft.whitelistDomains.includes(value)) return;
    setDraft({ ...draft, whitelistDomains: [...draft.whitelistDomains, value] });
    setNewDomain("");
  };

  return (
    <div className="fixed inset-0 z-50 flex justify-end bg-[#020806]/70 backdrop-blur-sm">
      <button
        aria-label="Close settings"
        className="absolute inset-0 cursor-default"
        onClick={onClose}
      />
      <aside className="relative flex h-full w-full max-w-[560px] flex-col border-l border-white/10 bg-[#091411] shadow-2xl">
        <header className="flex items-center justify-between border-b border-white/10 px-7 py-6">
          <div>
            <p className="text-xs font-semibold uppercase tracking-[0.28em] text-emerald-400">
              Configuration
            </p>
            <h2 className="mt-1 text-2xl font-semibold text-white">Protection settings</h2>
          </div>
          <button
            aria-label="Close settings"
            className="rounded-xl border border-white/10 bg-white/5 p-2 text-slate-400 transition hover:bg-white/10 hover:text-white"
            onClick={onClose}
          >
            <X size={20} />
          </button>
        </header>

        <div className="flex-1 space-y-8 overflow-y-auto px-7 py-7">
          <section>
            <div className="mb-4 flex items-start gap-3">
              <div className="rounded-xl border border-emerald-400/20 bg-emerald-400/10 p-2.5 text-emerald-400">
                <Globe2 size={20} />
              </div>
              <div>
                <h3 className="font-semibold text-white">Filter lists</h3>
                <p className="mt-1 text-sm leading-5 text-slate-400">
                  EasyList and hosts-format sources are compiled locally.
                </p>
              </div>
            </div>

            <div className="space-y-2">
              {draft.filterListUrls.map((url) => (
                <div
                  className="flex items-center gap-3 rounded-xl border border-white/10 bg-white/[0.035] px-4 py-3"
                  key={url}
                >
                  <span className="min-w-0 flex-1 truncate text-sm text-slate-300">{url}</span>
                  <button
                    aria-label={`Remove ${url}`}
                    className="text-slate-500 transition hover:text-rose-400 disabled:cursor-not-allowed disabled:opacity-30"
                    disabled={draft.filterListUrls.length === 1}
                    onClick={() =>
                      setDraft({
                        ...draft,
                        filterListUrls: draft.filterListUrls.filter((item) => item !== url),
                      })
                    }
                  >
                    <Trash2 size={16} />
                  </button>
                </div>
              ))}
            </div>

            <div className="mt-3 flex gap-2">
              <input
                className="min-w-0 flex-1 rounded-xl border border-white/10 bg-black/20 px-4 py-3 text-sm text-white outline-none transition placeholder:text-slate-600 focus:border-emerald-400/50"
                onChange={(event) => setNewList(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === "Enter") addList();
                }}
                placeholder="https://example.com/filters.txt"
                value={newList}
              />
              <button
                className="rounded-xl border border-white/10 bg-white/5 px-4 text-slate-300 transition hover:bg-white/10 hover:text-white"
                onClick={addList}
              >
                <Plus size={18} />
              </button>
            </div>
          </section>

          <section>
            <div className="mb-4 flex items-start gap-3">
              <div className="rounded-xl border border-sky-400/20 bg-sky-400/10 p-2.5 text-sky-400">
                <ShieldCheck size={20} />
              </div>
              <div>
                <h3 className="font-semibold text-white">Allowed domains</h3>
                <p className="mt-1 text-sm leading-5 text-slate-400">
                  These domains and their subdomains always bypass filtering.
                </p>
              </div>
            </div>

            <div className="flex flex-wrap gap-2">
              {draft.whitelistDomains.length === 0 && (
                <span className="text-sm text-slate-600">No exceptions configured.</span>
              )}
              {draft.whitelistDomains.map((domain) => (
                <span
                  className="flex items-center gap-2 rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-sm text-slate-300"
                  key={domain}
                >
                  {domain}
                  <button
                    aria-label={`Remove ${domain}`}
                    className="text-slate-500 hover:text-rose-400"
                    onClick={() =>
                      setDraft({
                        ...draft,
                        whitelistDomains: draft.whitelistDomains.filter(
                          (item) => item !== domain,
                        ),
                      })
                    }
                  >
                    <X size={13} />
                  </button>
                </span>
              ))}
            </div>

            <div className="mt-3 flex gap-2">
              <input
                className="min-w-0 flex-1 rounded-xl border border-white/10 bg-black/20 px-4 py-3 text-sm text-white outline-none transition placeholder:text-slate-600 focus:border-sky-400/50"
                onChange={(event) => setNewDomain(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === "Enter") addDomain();
                }}
                placeholder="example.com"
                value={newDomain}
              />
              <button
                className="rounded-xl border border-white/10 bg-white/5 px-4 text-slate-300 transition hover:bg-white/10 hover:text-white"
                onClick={addDomain}
              >
                <Plus size={18} />
              </button>
            </div>
          </section>
        </div>

        <footer className="grid grid-cols-2 gap-3 border-t border-white/10 bg-black/10 px-7 py-5">
          <button
            className="flex items-center justify-center gap-2 rounded-xl border border-white/10 bg-white/5 px-4 py-3 text-sm font-semibold text-slate-200 transition hover:bg-white/10 disabled:opacity-50"
            disabled={saving || updating}
            onClick={() => onUpdate(draft)}
          >
            <RefreshCw className={updating ? "animate-spin" : ""} size={17} />
            Update rules
          </button>
          <button
            className="flex items-center justify-center gap-2 rounded-xl bg-emerald-400 px-4 py-3 text-sm font-bold text-emerald-950 transition hover:bg-emerald-300 disabled:opacity-50"
            disabled={saving || updating}
            onClick={() => onSave(draft)}
          >
            <Save size={17} />
            Save changes
          </button>
        </footer>
      </aside>
    </div>
  );
}
