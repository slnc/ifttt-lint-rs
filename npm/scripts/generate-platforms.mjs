#!/usr/bin/env node
/**
 * Generates platform package.json files from the main npm/package.json.
 * Mirrors the approach used by biomejs/biome.
 *
 * Run: node npm/scripts/generate-platforms.mjs
 */
import { readFileSync, writeFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const NPM_DIR = resolve(__dirname, "..");
const PLATFORMS_DIR = resolve(NPM_DIR, "platforms");

const rootManifest = JSON.parse(readFileSync(resolve(NPM_DIR, "package.json"), "utf8"));

const PLATFORMS = [
  { os: "linux", cpu: "x64", libc: "glibc" },
  { os: "linux", cpu: "arm64", libc: "glibc" },
  { os: "darwin", cpu: "x64" },
  { os: "darwin", cpu: "arm64" },
  { os: "win32", cpu: "x64" },
];

for (const p of PLATFORMS) {
  const dirName = `ifchange-${p.os}-${p.cpu}`;
  const pkgName = `@slnc/${dirName}`;
  const bin = p.os === "win32" ? "ifchange.exe" : "ifchange";

  const manifest = {
    name: pkgName,
    version: rootManifest.version,
    description: `ifchange binary for ${p.os}-${p.cpu}`,
    license: rootManifest.license,
    repository: rootManifest.repository,
    engines: rootManifest.engines,
    os: [p.os],
    cpu: [p.cpu],
    ...(p.libc ? { libc: [p.libc] } : {}),
    preferUnplugged: true,
    files: [bin],
  };

  const path = resolve(PLATFORMS_DIR, dirName, "package.json");
  writeFileSync(path, JSON.stringify(manifest, null, 2) + "\n");
  console.log(`Generated ${path}`);
}
