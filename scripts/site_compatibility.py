#!/usr/bin/env python3
"""Confirm that representative ad-block test pages still load through SinkHole."""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import time


DEFAULT_SITES = (
    "https://adblock.turtlecute.org/",
    "https://adblock-tester.com/",
)


def load_site(curl: str, proxy: str, site: str, timeout: float) -> tuple[bool, str]:
    result = subprocess.run(
        [
            curl,
            "--proxy",
            proxy,
            "--location",
            "--silent",
            "--show-error",
            "--output",
            os.devnull,
            "--write-out",
            "%{http_code}",
            "--max-time",
            str(timeout),
            site,
        ],
        capture_output=True,
        check=False,
        text=True,
    )
    status = result.stdout.strip()
    if result.returncode != 0:
        return False, result.stderr.strip() or f"curl exited {result.returncode}"
    try:
        status_code = int(status)
    except ValueError:
        return False, f"invalid HTTP status {status!r}"
    if 200 <= status_code < 500:
        return True, f"HTTP {status_code}"
    return False, f"HTTP {status_code}"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("sites", nargs="*", default=DEFAULT_SITES)
    parser.add_argument("--proxy", default="http://127.0.0.1:8118")
    parser.add_argument("--timeout", type=float, default=20.0)
    parser.add_argument("--attempts", type=int, default=3)
    args = parser.parse_args()
    if args.timeout <= 0 or args.attempts < 1:
        parser.error("timeout and attempts must be positive")

    curl = shutil.which("curl.exe") or shutil.which("curl")
    if curl is None:
        print("FAIL: curl was not found.")
        return 2

    failures: list[tuple[str, str]] = []
    for site in args.sites:
        outcome = "not attempted"
        for attempt in range(1, args.attempts + 1):
            passed, outcome = load_site(curl, args.proxy, site, args.timeout)
            if passed:
                print(f"PASS {site} ({outcome})")
                break
            if attempt < args.attempts:
                time.sleep(1)
        else:
            print(f"FAIL {site} ({outcome})")
            failures.append((site, outcome))

    if failures:
        print("Compatibility smoke test failed: an allowed top-level page did not respond through the proxy.")
        return 1
    transport = f"through {args.proxy}" if args.proxy else "over a direct connection"
    print(f"PASS: all representative test pages responded {transport}.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
