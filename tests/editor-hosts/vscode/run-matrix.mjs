// The official runner owns host download, platform launch, and exit status.
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { createRequire } from "node:module";

const repo = process.cwd();
const require = createRequire(path.join(repo, "editors/vscode/package.json"));
const { runTests } = require("@vscode/test-electron");
const [projectArg, binaryArg, outputArg] = process.argv.slice(2);
if (!outputArg) throw new Error("Usage: run-matrix.mjs PROJECT LSP OUTPUT_JSON");
const root = path.resolve(projectArg);
const output = path.resolve(outputArg);
fs.mkdirSync(path.dirname(output), { recursive: true });
const profile = fs.mkdtempSync(path.join(os.tmpdir(), "recite-editor-session-"));
try {
  fs.mkdirSync(path.join(profile, "probe"));
  fs.writeFileSync(path.join(profile, "probe/package.json"), JSON.stringify({
    name: "recite-session-probe", version: "0.0.0", engines: { vscode: "^1.89.0" }
  }));
  for (const file of ["latency-probe.cjs", "render-probe.cjs", "session-probe.cjs"]) {
    fs.copyFileSync(path.join(repo, "tests/editor-hosts/vscode", file), path.join(profile, "probe", file));
  }
  await runTests({
    version: "1.136.1",
    vscodeExecutablePath: process.env.RECITE_TEST_VSCODE || undefined,
    extensionDevelopmentPath: [path.join(repo, "editors/vscode"), path.join(profile, "probe")],
    extensionTestsPath: path.join(profile, "probe/latency-probe.cjs"),
    extensionTestsEnv: {
      VSCODE_PORTABLE: path.join(profile, "portable"),
      XDG_CONFIG_HOME: path.join(profile, "config"), XDG_CACHE_HOME: path.join(profile, "cache"),
      XDG_STATE_HOME: path.join(profile, "state"),
      RECITE_PERF_ROOT: root, RECITE_PERF_BINARY: path.resolve(binaryArg), RECITE_PERF_OUTPUT: output,
      RECITE_PERF_SESSION_CYCLES: "20",
      RECITE_PERF_CDP_PORT_FILE: path.join(profile, "user-data/DevToolsActivePort")
    },
    launchArgs: [root, "--no-sandbox", "--disable-gpu", "--disable-updates", "--disable-telemetry",
      "--disable-crash-reporter", "--skip-welcome", "--skip-release-notes", "--disable-workspace-trust",
      "--remote-debugging-port=0", "--remote-debugging-address=127.0.0.1",
      `--user-data-dir=${path.join(profile, "user-data")}`, `--extensions-dir=${path.join(profile, "extensions")}`]
  });
  if (!fs.existsSync(output)) throw new Error("Editor report is missing");
} finally {
  fs.rmSync(profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 500 });
}
