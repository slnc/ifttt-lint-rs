#!/usr/bin/env node
"use strict";

const https = require("https");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");
const os = require("os");
const crypto = require("crypto");

const REPO = "slnc/ifchange";
const BINARY = "ifchange";
const VERSION = `v${require("./package.json").version}`;

const BIN_DIR = path.join(__dirname, "bin");
const BIN_NAME = process.platform === "win32" ? `${BINARY}.exe` : BINARY;
const BIN_PATH = path.join(BIN_DIR, BIN_NAME);

const PLATFORM_MAP = {
  linux: { x64: "x86_64-unknown-linux-gnu", arm64: "aarch64-unknown-linux-gnu" },
  darwin: { x64: "x86_64-apple-darwin", arm64: "aarch64-apple-darwin" },
  win32: { x64: "x86_64-pc-windows-msvc" },
};

function getTarget() {
  const platform = PLATFORM_MAP[process.platform];
  if (!platform) {
    throw new Error(`Unsupported platform: ${process.platform}`);
  }
  const target = platform[os.arch()];
  if (!target) {
    throw new Error(`Unsupported architecture: ${os.arch()} on ${process.platform}`);
  }
  return target;
}

function fetch(url) {
  return new Promise((resolve, reject) => {
    https.get(url, { headers: { "User-Agent": "ifchange-npm" } }, (res) => {
      if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
        return fetch(res.headers.location).then(resolve, reject);
      }
      if (res.statusCode !== 200) {
        return reject(new Error(`HTTP ${res.statusCode} for ${url}`));
      }
      const chunks = [];
      res.on("data", (chunk) => chunks.push(chunk));
      res.on("end", () => resolve(Buffer.concat(chunks)));
      res.on("error", reject);
    }).on("error", reject);
  });
}

async function install() {
  if (fs.existsSync(BIN_PATH)) {
    console.log(`${BINARY} already installed.`);
    return;
  }

  const target = getTarget();
  const ext = process.platform === "win32" ? "zip" : "tar.gz";
  const archiveName = `${BINARY}-${VERSION}-${target}.${ext}`;

  const archiveURL = `https://github.com/${REPO}/releases/download/${VERSION}/${archiveName}`;
  const checksumsURL = `https://github.com/${REPO}/releases/download/${VERSION}/SHA256SUMS`;

  console.log(`Downloading ${archiveName}...`);
  const [archive, checksums] = await Promise.all([
    fetch(archiveURL),
    fetch(checksumsURL),
  ]);

  // Verify checksum
  const expectedLine = checksums.toString().split("\n").find((l) => l.includes(archiveName));
  if (!expectedLine) {
    throw new Error(`Checksum not found for ${archiveName}`);
  }
  const expectedSum = expectedLine.trim().split(/\s+/)[0];
  const actualSum = crypto.createHash("sha256").update(archive).digest("hex");
  if (expectedSum !== actualSum) {
    throw new Error(`Checksum mismatch: expected ${expectedSum}, got ${actualSum}`);
  }
  console.log("Checksum verified.");

  // Extract to temp dir
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "ifchange-"));
  const archivePath = path.join(tmpDir, archiveName);
  fs.writeFileSync(archivePath, archive);

  try {
    if (ext === "zip") {
      execFileSync("powershell", ["-Command", `Expand-Archive -Path '${archivePath}' -DestinationPath '${tmpDir}'`], { stdio: "ignore" });
    } else {
      execFileSync("tar", ["-xzf", archivePath, "-C", tmpDir], { stdio: "ignore" });
    }

    // Find binary
    let extracted = findFile(tmpDir, BIN_NAME);
    if (!extracted) {
      throw new Error(`Could not find ${BIN_NAME} in archive`);
    }

    // Place native binary directly into bin/
    fs.mkdirSync(BIN_DIR, { recursive: true });
    fs.copyFileSync(extracted, BIN_PATH);
    fs.chmodSync(BIN_PATH, 0o555);
    console.log(`Installed ${BINARY} to ${BIN_PATH}`);
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
}

function findFile(dir, name) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      const found = findFile(full, name);
      if (found) return found;
    } else if (entry.name === name) {
      return full;
    }
  }
  return null;
}

install().catch((err) => {
  console.error(`Failed to install ${BINARY}: ${err.message}`);
  process.exit(1);
});
