#!/usr/bin/env python3
"""Concurrent black-box stress test for a running SinkHole proxy."""

from __future__ import annotations

import argparse
import asyncio
import math
import statistics
import time
from collections import Counter, defaultdict
from dataclasses import dataclass


@dataclass(frozen=True)
class Target:
    category: str
    host: str
    path: str


TARGETS = (
    Target("ad", "ads.doubleclick.net", "/pagead.js"),
    Target("tracker", "google-analytics.com", "/collect"),
    Target("telemetry", "vortex.data.microsoft.com", "/events"),
)


async def request_once(
    proxy_host: str,
    proxy_port: int,
    target: Target,
    request_id: int,
    timeout_seconds: float,
) -> tuple[str, int | None, float, str | None]:
    started = time.perf_counter()
    writer: asyncio.StreamWriter | None = None
    try:
        reader, writer = await asyncio.wait_for(
            asyncio.open_connection(proxy_host, proxy_port),
            timeout=timeout_seconds,
        )
        path = f"{target.path}?sinkhole_stress={request_id}"
        request = (
            f"GET http://{target.host}{path} HTTP/1.1\r\n"
            f"Host: {target.host}\r\n"
            "User-Agent: SinkHole-Stress/1.0\r\n"
            "Connection: close\r\n\r\n"
        )
        writer.write(request.encode("ascii"))
        await asyncio.wait_for(writer.drain(), timeout=timeout_seconds)
        response_head = await asyncio.wait_for(
            reader.readuntil(b"\r\n\r\n"),
            timeout=timeout_seconds,
        )
        status = int(response_head.split(b"\r\n", 1)[0].split()[1])
        return target.category, status, (time.perf_counter() - started) * 1000, None
    except Exception as error:  # noqa: BLE001 - every request must be reported
        return (
            target.category,
            None,
            (time.perf_counter() - started) * 1000,
            f"{type(error).__name__}: {error}",
        )
    finally:
        if writer is not None:
            writer.close()
            try:
                await writer.wait_closed()
            except OSError:
                pass


def percentile(values: list[float], fraction: float) -> float:
    ordered = sorted(values)
    index = max(0, min(len(ordered) - 1, math.ceil(len(ordered) * fraction) - 1))
    return ordered[index]


async def run(args: argparse.Namespace) -> int:
    print(f"Preflight: {args.proxy_host}:{args.proxy_port}")
    for index, target in enumerate(TARGETS):
        category, status, latency, error = await request_once(
            args.proxy_host,
            args.proxy_port,
            target,
            -(index + 1),
            args.timeout,
        )
        print(f"  {category:9} status={status} latency={latency:.1f}ms")
        if error or status != 204:
            print(f"FAIL: {category} preflight was not sinkholed ({error or status}).")
            print("Load phase aborted to avoid forwarding test traffic upstream.")
            return 2

    semaphore = asyncio.Semaphore(args.concurrency)

    async def limited_request(target: Target, request_id: int):
        async with semaphore:
            return await request_once(
                args.proxy_host,
                args.proxy_port,
                target,
                request_id,
                args.timeout,
            )

    jobs = [
        limited_request(target, request_id)
        for target in TARGETS
        for request_id in range(args.requests_per_category)
    ]
    total_requests = len(jobs)
    print(
        f"Load: {total_requests} requests, concurrency={args.concurrency}, "
        f"timeout={args.timeout:.1f}s"
    )
    started = time.perf_counter()
    results = await asyncio.gather(*jobs)
    duration = time.perf_counter() - started

    statuses: Counter[int | None] = Counter()
    latencies: dict[str, list[float]] = defaultdict(list)
    errors: Counter[str] = Counter()
    category_successes: Counter[str] = Counter()
    for category, status, latency, error in results:
        statuses[status] += 1
        latencies[category].append(latency)
        if status == 204 and error is None:
            category_successes[category] += 1
        if error:
            errors[error] += 1

    for target in TARGETS:
        values = latencies[target.category]
        print(
            f"  {target.category:9} 204={category_successes[target.category]}/"
            f"{args.requests_per_category} p50={statistics.median(values):.1f}ms "
            f"p95={percentile(values, 0.95):.1f}ms p99={percentile(values, 0.99):.1f}ms"
        )

    throughput = total_requests / duration if duration else float("inf")
    print(f"Throughput: {throughput:.1f} requests/s over {duration:.2f}s")
    print(f"Statuses: {dict(sorted(statuses.items(), key=lambda item: str(item[0])))}")
    if errors:
        for error, count in errors.most_common(5):
            print(f"  error x{count}: {error}")

    passed = statuses == Counter({204: total_requests})
    print("PASS" if passed else "FAIL")
    return 0 if passed else 1


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--proxy-host", default="127.0.0.1")
    parser.add_argument("--proxy-port", type=int, default=8118)
    parser.add_argument("--requests-per-category", type=int, default=1000)
    parser.add_argument("--concurrency", type=int, default=128)
    parser.add_argument("--timeout", type=float, default=5.0)
    args = parser.parse_args()
    if args.requests_per_category < 1 or args.concurrency < 1 or args.timeout <= 0:
        parser.error("request count, concurrency, and timeout must be positive")
    return args


if __name__ == "__main__":
    raise SystemExit(asyncio.run(run(parse_args())))
