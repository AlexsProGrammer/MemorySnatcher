#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import process from "node:process";
import { existsSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

const isWindows = process.platform === "win32";

function hasCommand(command) {
  const probe = isWindows ? "where" : "which";
  const result = spawnSync(probe, [command], { stdio: "ignore" });
  return result.status === 0;
}

function isRustupInstalled() {
  const rustupHome = join(homedir(), ".rustup");
  return existsSync(rustupHome);
}

function getRustupBinPath() {
  return join(homedir(), ".cargo", "bin");
}

function fail(message) {
  console.error(`\n[env-check] ${message}`);
  process.exit(1);
}

if (!hasCommand("cargo") || !hasCommand("rustc")) {
  const hasRustup = isRustupInstalled();
  const rustupBin = getRustupBinPath();

  if (isWindows) {
    if (hasRustup) {
      console.error(
        `\n[env-check] Rust is installed at ${rustupBin}, but not in your PATH.`
      );
      console.error(
        `\n[env-check] SOLUTION 1: Close PowerShell completely and open a NEW terminal window.`
      );
      console.error(
        `[env-check] SOLUTION 2: Or set PATH in this session by running (copy-paste exactly):`
      );
      console.error(
        `[env-check]\n  \$env:PATH = "${rustupBin}" + ";" + \$env:PATH\n`
      );
      console.error(`[env-check] Then retry: npm run tauri dev\n`);
      process.exit(1);
    } else {
      fail(
        "Rust toolchain not found. Install rustup from https://rustup.rs/, then restart your terminal."
      );
    }
  }

  if (hasRustup) {
    fail(
      `Rust is installed, but cargo/rustc are not in PATH. Try: source "$HOME/.cargo/env" and retry.`
    );
  } else {
    fail(
      "Rust toolchain not found (cargo/rustc). Install rustup from https://rustup.rs/ and retry."
    );
  }
}

if (isWindows && !hasCommand("cl")) {
  console.warn(
    "[env-check] MSVC compiler (cl.exe) not found in PATH. If build fails at linking, install Visual Studio Build Tools (Desktop development with C++)."
  );
}

if (!hasCommand("ffmpeg")) {
  console.warn(
    "[env-check] ffmpeg not found in PATH. The app can start, but media processing features may fail until ffmpeg is installed."
  );
}

console.log("[env-check] Environment checks passed.");
