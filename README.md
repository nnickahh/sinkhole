# SinkHole

SinkHole is a cosmic-themed, local-first privacy proxy for Windows. It routes browser traffic through a Rust loopback proxy and sends known ad, tracker, and telemetry destinations into an event horizon before they load.

Built for OpenAI Build Week with Tauri v2, Rust, React, TypeScript, and Codex/GPT-5.6. The project is intentionally small enough to reuse and explain in a school project.

## What works

- One-click Windows browser connection through the current-user system proxy.
- A WARP-style tray popover with one truthful power switch for filtering plus browser connection.
- Previous Windows proxy values are backed up and restored on disconnect or tray quit.
- EasyList blocks advertising hosts.
- EasyPrivacy blocks common analytics and tracking hosts.
- HaGeZi's native Windows/Office list blocks known telemetry endpoints.
- Live counters separate ads, trackers, telemetry, and custom rules.
- A bounded, memory-only activity view shows recently blocked hostnames without storing full URLs or request contents.
- A local `http://sinkhole.test` page proves that the browser is actually connected.
- HTTPS filtering happens at the hostname boundary without installing a root certificate or decrypting content.
- Protection, lists, and allowed domains persist across launches.

## Honest limits

SinkHole is a hostname-level privacy proxy, not a browser extension. It deliberately ignores path-specific and conditional browser filter rules rather than risk blocking an entire website. It cannot remove empty ad boxes or other page elements after HTML loads, inspect paths inside encrypted HTTPS requests, prevent an application from collecting data locally, or guarantee that every telemetry endpoint appears in a public list. Some browsers and apps can ignore the Windows proxy or use proxy-bypassing transports.

Turning the filter engine on is not the same as connecting traffic. The dashboard reports these states separately and only says traffic was captured after the proxy observes a request.

## Run locally

Windows prerequisites: Node.js 18+, pnpm, Rust stable, WebView2, and Visual Studio 2022 Build Tools with the C++ desktop workload.

```powershell
pnpm install
pnpm tauri dev
```

In the app:

1. Select **Connect Windows**.
2. Open **connection test** and confirm the SinkHole page appears.
3. Browse normally and watch **Requests seen** and category counters update.
4. Use **Disconnect** or **Quit SinkHole** from the tray to restore the previous proxy.

Left-click the SinkHole tray icon for quick control; right-click it for the tray menu. The compact switch enables or disables both the filter engine and Windows browser routing. Select the gear to open the full dashboard.

Manual setup remains available at `127.0.0.1:8118` for browsers with independent proxy settings.

If SinkHole is force-killed while connected and browsing stops, open Windows **Settings → Network & internet → Proxy** and turn **Use a proxy server** off, then relaunch SinkHole. Normal disconnect and tray-quit paths restore the previous values automatically.

## Verification

```powershell
pnpm check
pnpm build
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml live_default_lists_contain_more_than_one_hundred_thousand_supported_rules -- --ignored
pnpm tauri build
```

The live list test requires internet access. Installers are written below `src-tauri/target/release/bundle/`.

With SinkHole running, exercise the packaged proxy with 3,000 concurrent-safe ad, tracker, and telemetry requests:

```powershell
python scripts/stress_test.py
```

The harness first requires a `204` sinkhole response from every category. It aborts before the load phase if any test destination would be forwarded upstream. Use `--requests-per-category` and `--concurrency` to increase or reduce the load.

Also verify that representative ad-block test pages still return a real response through the proxy:

```powershell
python scripts/site_compatibility.py
```

This is a page-load smoke test, not a promise of a perfect score. Browser-extension-only checks such as cosmetic element removal, scriptlets, and path-level HTTPS filtering remain outside the hostname proxy's scope.

## OpenAI Build Week submission

Suggested track: **Apps for Your Life**.

Suggested tagline: **A local privacy event horizon for ads, trackers, and Windows telemetry.**

Three-minute demo outline:

1. **Problem (0:00–0:25):** desktop privacy tools often hide whether traffic is actually connected.
2. **Build (0:25–0:55):** show the Tauri/Rust proxy, React dashboard, and categorized filter sources.
3. **Proof (0:55–1:50):** connect Windows, open `sinkhole.test`, browse, and show request/category counters rising.
4. **Controls (1:50–2:20):** pause filtering, scan sample destinations, allowlist a domain, then disconnect.
5. **Codex/GPT-5.6 (2:20–2:45):** explain how Codex helped inspect, implement, test, and document the app.
6. **Limits/future (2:45–3:00):** state hostname-only filtering and the future browser-extension companion.

The Devpost entry also needs a public video under three minutes, a repository link, the requested Codex session ID from `/feedback`, and a project thumbnail. If the repository is public, keep this license file in it.

Copy-ready Devpost fields and the final checklist are in [`docs/DEVPOST_SUBMISSION.md`](docs/DEVPOST_SUBMISSION.md).
The ready-to-record 2:35 voiceover and shot list are in [`docs/DEMO_SCRIPT.md`](docs/DEMO_SCRIPT.md).

## License

MIT — see [LICENSE](LICENSE).
