#!/usr/bin/env python3
"""Compare actual Astro builds and cached static assets; no deployment or CDN simulation."""

import argparse
import concurrent.futures
import hashlib
import http.client
import json
import os
import platform
import re
import resource
import shutil
import statistics
import subprocess
import tempfile
import time
from contextlib import closing
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
NODE_SERVER = """import http from "node:http";
import fs from "node:fs";
const assets = new Map(Object.entries(JSON.parse(fs.readFileSync(process.argv[2], "utf8"))));
http.createServer((request, response) => {
  const asset = assets.get(new URL(request.url, "http://localhost").pathname);
  if (!asset) { response.writeHead(404); response.end(); return; }
  response.writeHead(200, {"content-type": asset.type, "content-length": asset.bytes});
  fs.createReadStream(asset.file).on("error", error => response.destroy(error)).pipe(response);
}).listen(8087, "127.0.0.1");
"""
BUN_SERVER = """import fs from "node:fs";
const assets = new Map(Object.entries(JSON.parse(fs.readFileSync(process.argv[2], "utf8"))));
Bun.serve({hostname: "127.0.0.1", port: 8087, fetch(request) {
  const asset = assets.get(new URL(request.url).pathname);
  return asset ? new Response(Bun.file(asset.file), {headers: {"content-type": asset.type}}) : new Response(null, {status: 404});
}});
"""
WORKER = """export default {async fetch(request, env) {
  const asset = env.assets[new URL(request.url).pathname];
  if (!asset) return new Response(null, {status: 404});
  const response = await env.files.fetch("http://disk/" + asset.relative);
  return new Response(response.body, {status: response.status, headers: {"content-type": asset.type, "content-length": String(asset.bytes)}});
}};
"""
CONFIG = """using Workerd = import "/workerd/workerd.capnp";
const config :Workerd.Config = (
 services = [(name = "app", worker = (
  compatibilityDate = "2026-10-14",
  modules = [(name = "worker.mjs", esModule = embed "worker.mjs")],
  bindings = [(name = "assets", json = embed "assets.json"), (name = "files", service = "site-files")]
 )), (name = "site-files", disk = "content-dir")],
 sockets = [(name = "http", address = "127.0.0.1:8087", http = (), service = "app")]
);
"""


def timed(command, cwd, log):
    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    start = time.perf_counter()
    with log.open("w") as stream:
        subprocess.run(command, cwd=cwd, stdout=stream, stderr=subprocess.STDOUT, check=True)
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    return {
        "seconds": time.perf_counter() - start,
        "cpuSeconds": after.ru_utime + after.ru_stime - before.ru_utime - before.ru_stime,
    }


def manifest(dist):
    page = (dist / "index.html").read_text()
    paths = {"/": dist / "index.html"}
    # Resources declared by the real landing page, plus its lazily loaded WASM.
    for url in re.findall(r'(?:href|src)="(/[^"?#]+)', page):
        path = dist / url.lstrip("/")
        if path.is_file() and path.suffix in {".js", ".css", ".svg", ".woff2"}:
            paths[url] = path
    for path in dist.rglob("*.wasm"):
        paths["/" + path.relative_to(dist).as_posix()] = path
    types = {
        ".html": "text/html",
        ".js": "text/javascript",
        ".css": "text/css",
        ".svg": "image/svg+xml",
        ".woff2": "font/woff2",
        ".wasm": "application/wasm",
    }
    return {
        url: {
            "file": str(path),
            "relative": path.relative_to(dist).as_posix(),
            "type": types[path.suffix],
            "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
            "bytes": path.stat().st_size,
        }
        for url, path in paths.items()
    }


def request(connection, path):
    connection.request("GET", path)
    response = connection.getresponse()
    body = response.read()
    if response.status != 200:
        raise RuntimeError(f"{path}: HTTP {response.status}")
    return body


def serve_once(command, cwd, assets, concurrency, count):
    with tempfile.TemporaryFile() as log:
        start = time.perf_counter()
        process = subprocess.Popen(command, cwd=cwd, stdout=log, stderr=subprocess.STDOUT)
        try:
            while True:
                try:
                    with closing(
                        http.client.HTTPConnection("127.0.0.1", 8087, timeout=1)
                    ) as connection:
                        body = request(connection, "/")
                    if hashlib.sha256(body).hexdigest() != assets["/"]["sha256"]:
                        raise RuntimeError("landing body mismatch")
                    break
                except OSError:
                    if process.poll() is not None or time.perf_counter() - start > 30:
                        log.seek(0)
                        raise RuntimeError(log.read().decode())
                    time.sleep(0.005)
            ready = time.perf_counter() - start
            with closing(http.client.HTTPConnection("127.0.0.1", 8087)) as connection:
                for path, asset in assets.items():
                    body = request(connection, path)
                    assert hashlib.sha256(body).hexdigest() == asset["sha256"], path
            paths = list(assets)

            def client(index):
                durations = []
                with closing(
                    http.client.HTTPConnection("127.0.0.1", 8087, timeout=10)
                ) as connection:
                    for iteration in range(count // concurrency):
                        path = paths[(index + iteration) % len(paths)]
                        before = time.perf_counter()
                        body = request(connection, path)
                        assert len(body) == assets[path]["bytes"]
                        durations.append((time.perf_counter() - before) * 1000)
                return durations

            with closing(http.client.HTTPConnection("127.0.0.1", 8087)) as connection:
                for iteration in range(100):
                    request(connection, paths[iteration % len(paths)])
            rss_before = rss_kib(process.pid)
            cpu_before = cpu_seconds(process.pid)
            start = time.perf_counter()
            with concurrent.futures.ThreadPoolExecutor(max_workers=concurrency) as pool:
                samples = sorted(
                    value for values in pool.map(client, range(concurrency)) for value in values
                )
            elapsed = time.perf_counter() - start
            return {
                "readySeconds": ready,
                "concurrency": concurrency,
                "requests": len(samples),
                "seconds": elapsed,
                "requestsPerSecond": len(samples) / elapsed,
                "p50Ms": statistics.median(samples),
                "p95Ms": samples[int(len(samples) * 0.95)],
                "cpuSeconds": cpu_seconds(process.pid) - cpu_before,
                "warmRssKiB": rss_before,
                "afterRssKiB": rss_kib(process.pid),
            }
        finally:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()


def rss_kib(pid):
    for line in Path(f"/proc/{pid}/status").read_text().splitlines():
        if line.startswith("VmRSS:"):
            return int(line.split()[1])
    raise RuntimeError("missing RSS")


def cpu_seconds(pid):
    fields = Path(f"/proc/{pid}/stat").read_text().split()
    return (int(fields[13]) + int(fields[14])) / os.sysconf("SC_CLK_TCK")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("node", "node-lts", "bun", "workerd"):
        parser.add_argument(f"--{name}", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--rounds", type=int, default=5)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    versions = {
        name: subprocess.check_output(
            [str(getattr(args, name.replace("-", "_"))), "--version"], text=True
        ).strip()
        for name in ("node", "node-lts", "bun", "workerd")
    }
    report = {
        "versions": versions,
        "system": platform.platform(),
        "cpu": next(
            line.split(":", 1)[1].strip()
            for line in Path("/proc/cpuinfo").read_text().splitlines()
            if line.startswith("model name")
        ),
        "loadStart": os.getloadavg(),
        "build": [],
        "serve": [],
        "notes": [
            "Warm filesystem/dependency caches; each build removes dist. WASM generation and package installation excluded.",
            "Alternating runtimes across rounds. No other task builds/tests during sampling.",
            "Actual built files with warm OS page cache, uncompressed HTTP/1.1 loopback; no CDN, TLS, or remote Workers cold starts.",
            "Node streams, Bun.file/native serve and workerd read-only disk binding/fetch use different native transports; this compares complete serving options.",
            "Python client shares the machine; throughput is client-limited, not a saturation or production ranking.",
            "RSS is live process memory, not allocation churn. CPU is sampled process time, not stacks.",
        ],
    }
    runtimes = {"node": args.node, "node-lts": args.node_lts, "bun": args.bun}
    astro = REPO / "docs-site/node_modules/astro/bin/astro.mjs"
    for round_index in range(args.rounds):
        names = list(runtimes)
        if round_index % 2:
            names.reverse()
        for name in names:
            shutil.rmtree(REPO / "docs-site/dist", ignore_errors=True)
            command = [str(runtimes[name]), str(astro), "build"]
            if name == "bun":
                command.insert(1, "--bun")
            result = timed(
                command, REPO / "docs-site", args.output / f"build-{round_index}-{name}.log"
            )
            built = REPO / "docs-site/dist"
            hashes = {
                path.relative_to(built).as_posix(): hashlib.sha256(path.read_bytes()).hexdigest()
                for path in sorted(built.rglob("*"))
                if path.is_file()
            }
            result["artifactSha256"] = hashlib.sha256(
                json.dumps(hashes, sort_keys=True).encode()
            ).hexdigest()
            result["landingSha256"] = hashes["index.html"]
            result["files"] = len(hashes)
            report["build"].append({"runtime": name, "round": round_index, **result})
            (args.output / "results.json").write_text(json.dumps(report, indent=2))
            print("build", name, result, flush=True)
    assets = manifest(REPO / "docs-site/dist")
    report["assets"] = {
        key: {field: value for field, value in item.items() if field != "body"}
        for key, item in assets.items()
    }
    with tempfile.TemporaryDirectory(prefix="recite-site-runtime-") as temporary:
        scratch = Path(temporary)
        (scratch / "assets.json").write_text(json.dumps(assets))
        for name, source in [
            ("node.mjs", NODE_SERVER),
            ("bun.mjs", BUN_SERVER),
            ("worker.mjs", WORKER),
            ("config.capnp", CONFIG),
        ]:
            (scratch / name).write_text(source)
        commands = {
            "node": [str(args.node), "node.mjs", "assets.json"],
            "node-lts": [str(args.node_lts), "node.mjs", "assets.json"],
            "bun": [str(args.bun), "--bun", "bun.mjs", "assets.json"],
            "workerd": [
                str(args.workerd),
                "serve",
                "config.capnp",
                "--directory-path",
                "site-files=" + str(REPO / "docs-site/dist"),
            ],
        }
        for round_index in range(args.rounds):
            names = list(commands)
            if round_index % 2:
                names.reverse()
            for concurrency in (1, 16):
                for name in names:
                    result = serve_once(commands[name], scratch, assets, concurrency, 1600)
                    report["serve"].append({"runtime": name, "round": round_index, **result})
                    print("serve", name, concurrency, result["requestsPerSecond"], flush=True)
                    (args.output / "results.json").write_text(json.dumps(report, indent=2))
    report["loadEnd"] = os.getloadavg()
    (args.output / "results.json").write_text(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
