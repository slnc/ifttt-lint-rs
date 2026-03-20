/**
 * End-to-end test: builds the real binary, sets up a simulated node_modules
 * layout, and verifies the resolver script can find and execute it.
 *
 * Run: node --test npm/test/e2e-resolution.test.mjs
 *
 * Requires: cargo (Rust toolchain) to be available
 */
import { describe, it, before, after } from "node:test";
import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import { existsSync, copyFileSync, mkdirSync, rmSync, symlinkSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import os from "node:os";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dirname, "..", "..");
const NPM_DIR = join(__dirname, "..");
const PLATFORMS_DIR = join(NPM_DIR, "platforms");

// Map current platform to package dir and npm scope
const PLATFORM_MAP = {
  "linux-x64": { dir: "ifchange-linux-x64", scope: "@slnc/ifchange-linux-x64" },
  "linux-arm64": { dir: "ifchange-linux-arm64", scope: "@slnc/ifchange-linux-arm64" },
  "darwin-x64": { dir: "ifchange-darwin-x64", scope: "@slnc/ifchange-darwin-x64" },
  "darwin-arm64": { dir: "ifchange-darwin-arm64", scope: "@slnc/ifchange-darwin-arm64" },
  "win32-x64": { dir: "ifchange-win32-x64", scope: "@slnc/ifchange-win32-x64" },
};

const platformKey = `${process.platform}-${os.arch()}`;
const platformInfo = PLATFORM_MAP[platformKey];

// Skip if unsupported platform
if (!platformInfo) {
  console.log(`Skipping e2e test: unsupported platform ${platformKey}`);
  process.exit(0);
}

const binName = process.platform === "win32" ? "ifchange.exe" : "ifchange";
const builtBinPath = join(ROOT, "target", "release", binName);

// We'll create a temporary node_modules structure so require.resolve works.
// Use os.tmpdir() to avoid mutating the real npm/ project directory.
const tmpRoot = join(os.tmpdir(), `ifchange-e2e-${process.pid}`);
const tmpNodeModules = join(tmpRoot, "node_modules");
const tmpScopedDir = join(tmpNodeModules, "@slnc", platformInfo.dir);

describe("e2e binary resolution", () => {
  before(() => {
    // Build the binary if not already built
    if (!existsSync(builtBinPath)) {
      console.log("Building ifchange binary (cargo build --release)...");
      execFileSync("cargo", ["build", "--release"], {
        cwd: ROOT,
        stdio: "inherit",
        timeout: 300_000,
      });
    }
    assert.ok(existsSync(builtBinPath), `built binary should exist at ${builtBinPath}`);

    // Set up fake node_modules/@slnc/ifchange-{platform}/ with the binary
    mkdirSync(tmpScopedDir, { recursive: true });
    copyFileSync(builtBinPath, join(tmpScopedDir, binName));
  });

  after(() => {
    // Clean up the temporary directory
    if (existsSync(tmpRoot)) {
      rmSync(tmpRoot, { recursive: true, force: true });
    }
  });

  it("resolver finds and executes the platform binary via require.resolve", () => {
    const result = spawnSync("node", [join(NPM_DIR, "bin", "ifchange"), "--version"], {
      stdio: "pipe",
      timeout: 10_000,
      env: { ...process.env, NODE_PATH: tmpNodeModules },
    });

    const stdout = result.stdout?.toString().trim() || "";
    const stderr = result.stderr?.toString().trim() || "";

    assert.equal(result.status, 0, `should exit 0, got ${result.status}. stderr: ${stderr}`);
    assert.match(stdout, /\d+\.\d+\.\d+/, `should print version, got: ${stdout}`);
  });

  it("IFCHANGE_BINARY env override with absolute path works", () => {
    const result = spawnSync("node", [join(NPM_DIR, "bin", "ifchange"), "--version"], {
      stdio: "pipe",
      timeout: 10_000,
      env: {
        ...process.env,
        NODE_PATH: tmpNodeModules,
        IFCHANGE_BINARY: builtBinPath,
      },
    });

    const stdout = result.stdout?.toString().trim() || "";
    const stderr = result.stderr?.toString().trim() || "";

    assert.equal(result.status, 0, `should exit 0, got ${result.status}. stderr: ${stderr}`);
    assert.match(stdout, /\d+\.\d+\.\d+/, `should print version, got: ${stdout}`);
  });

  it("passes arguments through to the binary", () => {
    const result = spawnSync("node", [join(NPM_DIR, "bin", "ifchange"), "--help"], {
      stdio: "pipe",
      timeout: 10_000,
      env: { ...process.env, NODE_PATH: tmpNodeModules },
    });

    const stdout = result.stdout?.toString() || "";

    assert.equal(result.status, 0, "should exit 0");
    assert.ok(stdout.includes("ifchange") || stdout.includes("Usage"), `should show help, got: ${stdout}`);
  });
});
