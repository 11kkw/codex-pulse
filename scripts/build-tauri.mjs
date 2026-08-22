import { spawn } from "node:child_process";
import { cp, mkdir } from "node:fs/promises";
import os from "node:os";
import path from "node:path";

const projectRoot = path.resolve(import.meta.dirname, "..");
const targetDir = path.join(os.tmpdir(), "codex-pulse-cargo-target-v2");
const noBundle = process.argv.includes("--no-bundle");
const tauriCommand = path.join(
  projectRoot,
  "node_modules",
  ".bin",
  process.platform === "win32" ? "tauri.cmd" : "tauri",
);
const args = ["build"];
if (noBundle) {
  args.push("--no-bundle");
} else {
  const bundleTypes = process.platform === "win32"
    ? ["nsis"]
    : process.platform === "darwin"
      ? ["app", "dmg"]
      : ["appimage"];
  args.push("--bundles", ...bundleTypes);
}

const exitCode = await new Promise((resolve, reject) => {
  const child = spawn(tauriCommand, args, {
    cwd: projectRoot,
    stdio: "inherit",
    shell: process.platform === "win32",
    env: {
      ...process.env,
      CARGO_TARGET_DIR: targetDir,
    },
  });
  child.on("error", reject);
  child.on("exit", (code) => resolve(code ?? 1));
});

if (exitCode !== 0) process.exit(exitCode);

const executableName = process.platform === "win32" ? "codex-pulse.exe" : "codex-pulse";
const executableSource = path.join(targetDir, "release", executableName);
const executableDestination = path.join(
  projectRoot,
  "src-tauri",
  "target",
  "release",
  executableName,
);
await mkdir(path.dirname(executableDestination), { recursive: true });
await cp(executableSource, executableDestination, { force: true });
console.log(`Executable copied to ${executableDestination}`);

if (!noBundle) {
  const source = path.join(targetDir, "release", "bundle");
  const destination = path.join(projectRoot, "src-tauri", "target", "release", "bundle");
  await mkdir(destination, { recursive: true });
  await cp(source, destination, { recursive: true, force: true });
  console.log(`Installer copied to ${destination}`);
}
