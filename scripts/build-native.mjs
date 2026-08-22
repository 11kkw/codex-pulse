import { spawn } from "node:child_process";
import { copyFile, mkdir } from "node:fs/promises";
import os from "node:os";
import path from "node:path";

const projectRoot = path.resolve(import.meta.dirname, "..");
const targetDir = path.join(os.tmpdir(), "codex-pulse-cargo-target-v2");
const release = process.argv.includes("--release");
const profileDir = release ? "release" : "debug";
const executableName = process.platform === "win32" ? "codex-pulse.exe" : "codex-pulse";
const cargoArgs = [
  "build",
  "--manifest-path",
  path.join(projectRoot, "src-tauri", "Cargo.toml"),
  "--features",
  "tauri/custom-protocol",
  "-j",
  "1",
];

if (release) cargoArgs.push("--release");

const exitCode = await new Promise((resolve, reject) => {
  const child = spawn("cargo", cargoArgs, {
    cwd: projectRoot,
    stdio: "inherit",
    env: {
      ...process.env,
      CARGO_TARGET_DIR: targetDir,
    },
  });
  child.on("error", reject);
  child.on("exit", (code) => resolve(code ?? 1));
});

if (exitCode !== 0) process.exit(exitCode);

const outputDir = path.join(projectRoot, "src-tauri", "target", profileDir);
await mkdir(outputDir, { recursive: true });
await copyFile(
  path.join(targetDir, profileDir, executableName),
  path.join(outputDir, executableName),
);
