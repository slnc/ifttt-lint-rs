window.BENCHMARK_DATA = {
  "lastUpdate": 1772871768124,
  "repoUrl": "https://github.com/slnc/ifchange",
  "entries": {
    "Benchmark": [
      {
        "commit": {
          "author": {
            "email": "juan@juanalonso.com",
            "name": "slnc",
            "username": "slnc"
          },
          "committer": {
            "email": "juan@juanalonso.com",
            "name": "slnc",
            "username": "slnc"
          },
          "distinct": true,
          "id": "f53080ce4381fbb0728ad1693c15789174ccbdac",
          "message": "fix: benchmark CI output format for parser\n\n## Why\n`benchmark-action` with `tool: cargo` expects libtest format, not\nCriterion's default output. The benchmark workflow has never passed.\n\n## What\n- Add `--output-format bencher` to emit libtest-compatible output",
          "timestamp": "2026-03-07T08:15:35+01:00",
          "tree_id": "e4cf9badb1ba030c0a226a1bb90e31affb3f8941",
          "url": "https://github.com/slnc/ifchange/commit/f53080ce4381fbb0728ad1693c15789174ccbdac"
        },
        "date": 1772867879510,
        "tool": "cargo",
        "benches": [
          {
            "name": "lint_latency_16kloc_diff",
            "value": 1952263,
            "range": "± 44767",
            "unit": "ns/iter"
          },
          {
            "name": "lint_1000_files",
            "value": 6042646,
            "range": "± 32801",
            "unit": "ns/iter"
          },
          {
            "name": "lint_5000_files",
            "value": 33097780,
            "range": "± 1051678",
            "unit": "ns/iter"
          },
          {
            "name": "scan_5000_files",
            "value": 37141299,
            "range": "± 403167",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "juan@juanalonso.com",
            "name": "slnc",
            "username": "slnc"
          },
          "committer": {
            "email": "juan@juanalonso.com",
            "name": "slnc",
            "username": "slnc"
          },
          "distinct": true,
          "id": "86dd078bca9a4ef253952a4663c1bc77de16f8fb",
          "message": "ci: add release-please automation and commitlint enforcement\n\n- Add release-please workflow and config for automated versioning,\n  changelog generation, and Release PR creation\n- Add commitlint workflow to validate PR titles against conventional\n  commit format\n- Add x-release-please-version marker to pypi/pyproject.toml\n- Disable generate_release_notes in release-binaries (release-please\n  handles release notes)\n- Add docs/RELEASE.md documenting the release workflow\n- Narrow IfChange block in README to only guard the language table\n- Include pending README, test refactoring changes",
          "timestamp": "2026-03-07T09:20:43+01:00",
          "tree_id": "5f66db125cff77c457110539bf9b27a7dc4f5d6c",
          "url": "https://github.com/slnc/ifchange/commit/86dd078bca9a4ef253952a4663c1bc77de16f8fb"
        },
        "date": 1772871767302,
        "tool": "cargo",
        "benches": [
          {
            "name": "lint_latency_16kloc_diff",
            "value": 2114935,
            "range": "± 9314",
            "unit": "ns/iter"
          },
          {
            "name": "lint_1000_files",
            "value": 7038470,
            "range": "± 275267",
            "unit": "ns/iter"
          },
          {
            "name": "lint_5000_files",
            "value": 35971602,
            "range": "± 1452411",
            "unit": "ns/iter"
          },
          {
            "name": "scan_5000_files",
            "value": 58163055,
            "range": "± 416613",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}