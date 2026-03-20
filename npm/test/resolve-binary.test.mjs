/**
 * Tests for the npm package binary resolution.
 *
 * Run: node --test npm/test/resolve-binary.test.mjs
 */
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, existsSync, statSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { execFileSync } from "node:child_process";

const __dirname = dirname(fileURLToPath(import.meta.url));
const NPM_DIR = join(__dirname, "..");
const PLATFORMS_DIR = join(NPM_DIR, "platforms");

const EXPECTED_PLATFORMS = [
  { dir: "ifchange-linux-x64", os: "linux", cpu: "x64", name: "@slnc/ifchange-linux-x64", files: ["ifchange"] },
  { dir: "ifchange-linux-arm64", os: "linux", cpu: "arm64", name: "@slnc/ifchange-linux-arm64", files: ["ifchange"] },
  { dir: "ifchange-darwin-x64", os: "darwin", cpu: "x64", name: "@slnc/ifchange-darwin-x64", files: ["ifchange"] },
  { dir: "ifchange-darwin-arm64", os: "darwin", cpu: "arm64", name: "@slnc/ifchange-darwin-arm64", files: ["ifchange"] },
  { dir: "ifchange-win32-x64", os: "win32", cpu: "x64", name: "@slnc/ifchange-win32-x64", files: ["ifchange.exe"] },
];

function readJson(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

describe("package.json consistency", () => {
  const mainPkg = readJson(join(NPM_DIR, "package.json"));

  it("main package has all platform optional dependencies", () => {
    const optDeps = mainPkg.optionalDependencies;
    assert.ok(optDeps, "optionalDependencies should exist");
    for (const p of EXPECTED_PLATFORMS) {
      assert.ok(optDeps[p.name], `missing optionalDependency: ${p.name}`);
    }
    assert.equal(
      Object.keys(optDeps).length,
      EXPECTED_PLATFORMS.length,
      "should have exactly the expected number of optional dependencies"
    );
  });

  it("main package has no postinstall script", () => {
    assert.equal(mainPkg.scripts?.postinstall, undefined, "postinstall should not exist");
  });

  it("main package has bin entry", () => {
    assert.equal(mainPkg.bin?.ifchange, "bin/ifchange");
  });

  it("all versions are consistent", () => {
    const version = mainPkg.version;
    assert.ok(version, "main package should have a version");

    // Optional dependency versions match main version
    for (const [name, ver] of Object.entries(mainPkg.optionalDependencies)) {
      assert.equal(ver, version, `${name} optionalDependency version should match main`);
    }

    // Platform package.json versions match
    for (const p of EXPECTED_PLATFORMS) {
      const platPkg = readJson(join(PLATFORMS_DIR, p.dir, "package.json"));
      assert.equal(platPkg.version, version, `${p.name} version should match main`);
    }
  });

  for (const p of EXPECTED_PLATFORMS) {
    describe(`platform package: ${p.name}`, () => {
      const platPkg = readJson(join(PLATFORMS_DIR, p.dir, "package.json"));

      it("has correct name", () => {
        assert.equal(platPkg.name, p.name);
      });

      it("has correct os constraint", () => {
        assert.deepEqual(platPkg.os, [p.os]);
      });

      it("has correct cpu constraint", () => {
        assert.deepEqual(platPkg.cpu, [p.cpu]);
      });

      it("linux packages have glibc libc constraint", () => {
        if (p.os === "linux") {
          assert.deepEqual(platPkg.libc, ["glibc"]);
        }
      });

      it("has MIT license", () => {
        assert.equal(platPkg.license, "MIT");
      });

      it("has repository", () => {
        assert.equal(platPkg.repository?.url, "https://github.com/slnc/ifchange");
      });

      it("has explicit files field with binary", () => {
        assert.deepEqual(platPkg.files, p.files, `${p.name} should declare files: ${JSON.stringify(p.files)}`);
      });

      it("has preferUnplugged for Yarn PnP compatibility", () => {
        assert.equal(platPkg.preferUnplugged, true);
      });
    });
  }
});

describe("bin/ifchange resolver script", () => {
  const binPath = join(NPM_DIR, "bin", "ifchange");

  it("exists and is executable", () => {
    assert.ok(existsSync(binPath), "bin/ifchange should exist");
    const mode = statSync(binPath).mode;
    assert.ok(mode & 0o111, "bin/ifchange should be executable");
  });

  it("has correct shebang", () => {
    const content = readFileSync(binPath, "utf8");
    assert.ok(content.startsWith("#!/usr/bin/env node"), "should start with node shebang");
  });

  it("maps all expected platforms", () => {
    const content = readFileSync(binPath, "utf8");
    for (const p of EXPECTED_PLATFORMS) {
      assert.ok(content.includes(p.name), `should reference ${p.name}`);
    }
  });

  it("supports IFCHANGE_BINARY env override", () => {
    const content = readFileSync(binPath, "utf8");
    assert.ok(content.includes("IFCHANGE_BINARY"), "should support IFCHANGE_BINARY env");
  });

  it("forwards signals from child process", () => {
    const content = readFileSync(binPath, "utf8");
    assert.ok(content.includes("result.signal"), "should check for child signal");
    assert.ok(content.includes("process.kill(process.pid"), "should re-raise signal");
  });

  it("mentions musl/Alpine in error message", () => {
    const content = readFileSync(binPath, "utf8");
    assert.ok(content.includes("musl"), "error message should mention musl for Alpine users");
  });
});

describe("old install.js removal", () => {
  it("install.js no longer exists", () => {
    assert.ok(!existsSync(join(NPM_DIR, "install.js")), "install.js should be removed");
  });
});

describe("current platform resolution (integration)", () => {
  it("bin script runs without error on missing platform package", () => {
    // When no platform package is installed, the script should exit with code 1
    // and print a helpful error, NOT crash with an unhandled exception
    try {
      const stdout = execFileSync("node", [join(NPM_DIR, "bin", "ifchange"), "--version"], {
        stdio: "pipe",
        timeout: 5000,
      });
      // Platform package is installed locally (dev scenario) — assert a version was printed
      assert.match(stdout.toString().trim(), /^\d+\.\d+\.\d+/, "version output should match semver");
    } catch (err) {
      // Should get a clean exit with code 1, not a crash with an unhandled exception
      assert.strictEqual(err.status, 1, `expected exit code 1, got: ${err.status}`);
      assert.strictEqual(err.signal, null, `expected no signal, got: ${err.signal}`);
      const stderr = err.stderr?.toString() || "";
      const stdout = err.stdout?.toString() || "";
      const output = stderr + stdout;
      assert.ok(
        output.includes("not installed") || output.includes("not ship"),
        `should print helpful error message, got: ${output}`
      );
    }
  });
});
