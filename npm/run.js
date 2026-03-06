#!/usr/bin/env node
"use strict";

const path = require("path");
const { spawnSync } = require("child_process");

const BINARY = "lint-ifchange";
const binName = process.platform === "win32" ? `${BINARY}.exe` : BINARY;
const binPath = path.join(__dirname, binName);

const result = spawnSync(binPath, process.argv.slice(2), {
  stdio: "inherit",
  windowsHide: true,
});

if (result.error) {
  if (result.error.code === "ENOENT") {
    console.error(
      `${BINARY} binary not found at ${binPath}\n` +
      "Try reinstalling: npm install lint-ifchange"
    );
    process.exit(1);
  }
  throw result.error;
}

process.exit(result.status ?? 1);
