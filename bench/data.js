window.BENCHMARK_DATA = {
  "lastUpdate": 1772867880355,
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
      }
    ]
  }
}