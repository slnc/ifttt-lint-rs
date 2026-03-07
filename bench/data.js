window.BENCHMARK_DATA = {
  "lastUpdate": 1772883955070,
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
          "id": "83a8147f113b0f2eccabc1042a014b192e2f1f40",
          "message": "fix: correct release-please-action pinned SHA",
          "timestamp": "2026-03-07T09:26:32+01:00",
          "tree_id": "c9384fa5dc1cba3e2b8e5e58398136c563a08216",
          "url": "https://github.com/slnc/ifchange/commit/83a8147f113b0f2eccabc1042a014b192e2f1f40"
        },
        "date": 1772872117540,
        "tool": "cargo",
        "benches": [
          {
            "name": "lint_latency_16kloc_diff",
            "value": 2109496,
            "range": "± 4188",
            "unit": "ns/iter"
          },
          {
            "name": "lint_1000_files",
            "value": 7044945,
            "range": "± 53304",
            "unit": "ns/iter"
          },
          {
            "name": "lint_5000_files",
            "value": 36189737,
            "range": "± 1356649",
            "unit": "ns/iter"
          },
          {
            "name": "scan_5000_files",
            "value": 58684161,
            "range": "± 866085",
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
          "id": "5efb026be02fc3976af9379fb0d4a2c632957a8a",
          "message": "ci: re-trigger release-please",
          "timestamp": "2026-03-07T09:28:36+01:00",
          "tree_id": "c9384fa5dc1cba3e2b8e5e58398136c563a08216",
          "url": "https://github.com/slnc/ifchange/commit/5efb026be02fc3976af9379fb0d4a2c632957a8a"
        },
        "date": 1772872231691,
        "tool": "cargo",
        "benches": [
          {
            "name": "lint_latency_16kloc_diff",
            "value": 2023887,
            "range": "± 45603",
            "unit": "ns/iter"
          },
          {
            "name": "lint_1000_files",
            "value": 5791646,
            "range": "± 218333",
            "unit": "ns/iter"
          },
          {
            "name": "lint_5000_files",
            "value": 32629076,
            "range": "± 507668",
            "unit": "ns/iter"
          },
          {
            "name": "scan_5000_files",
            "value": 37577162,
            "range": "± 101210",
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
          "id": "7b48f487100afba04b7e35f62c70dcbff5bc0239",
          "message": "ci: set release-please manifest to 0.0.0 for initial release",
          "timestamp": "2026-03-07T09:32:44+01:00",
          "tree_id": "f92d6cd2e852c0b381b7afb15db045e5db1111ed",
          "url": "https://github.com/slnc/ifchange/commit/7b48f487100afba04b7e35f62c70dcbff5bc0239"
        },
        "date": 1772872471861,
        "tool": "cargo",
        "benches": [
          {
            "name": "lint_latency_16kloc_diff",
            "value": 2108655,
            "range": "± 5808",
            "unit": "ns/iter"
          },
          {
            "name": "lint_1000_files",
            "value": 7039118,
            "range": "± 56998",
            "unit": "ns/iter"
          },
          {
            "name": "lint_5000_files",
            "value": 37026536,
            "range": "± 1499495",
            "unit": "ns/iter"
          },
          {
            "name": "scan_5000_files",
            "value": 60815664,
            "range": "± 1001046",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "41898282+github-actions[bot]@users.noreply.github.com",
            "name": "github-actions[bot]",
            "username": "github-actions[bot]"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "3bc8ef7138e1fa02ea76ecb5054385ea37c0597b",
          "message": "chore: release ifchange 0.1.0 (#7)",
          "timestamp": "2026-03-07T09:35:56+01:00",
          "tree_id": "eac93fcb5f16a34e66e47c9bbea8a511aa332b5f",
          "url": "https://github.com/slnc/ifchange/commit/3bc8ef7138e1fa02ea76ecb5054385ea37c0597b"
        },
        "date": 1772872663950,
        "tool": "cargo",
        "benches": [
          {
            "name": "lint_latency_16kloc_diff",
            "value": 2110725,
            "range": "± 10609",
            "unit": "ns/iter"
          },
          {
            "name": "lint_1000_files",
            "value": 7018923,
            "range": "± 65054",
            "unit": "ns/iter"
          },
          {
            "name": "lint_5000_files",
            "value": 36858331,
            "range": "± 1294051",
            "unit": "ns/iter"
          },
          {
            "name": "scan_5000_files",
            "value": 60247679,
            "range": "± 832062",
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
          "id": "20e695f0d0cc511ce8500499fc4cad08bd84f655",
          "message": "ci: reset manifest to 0.0.0 for clean initial release",
          "timestamp": "2026-03-07T09:42:09+01:00",
          "tree_id": "d909bbb08b437559606bc17694ce5e006ccf44d5",
          "url": "https://github.com/slnc/ifchange/commit/20e695f0d0cc511ce8500499fc4cad08bd84f655"
        },
        "date": 1772873051672,
        "tool": "cargo",
        "benches": [
          {
            "name": "lint_latency_16kloc_diff",
            "value": 2114214,
            "range": "± 85287",
            "unit": "ns/iter"
          },
          {
            "name": "lint_1000_files",
            "value": 7032927,
            "range": "± 292778",
            "unit": "ns/iter"
          },
          {
            "name": "lint_5000_files",
            "value": 36384262,
            "range": "± 1309885",
            "unit": "ns/iter"
          },
          {
            "name": "scan_5000_files",
            "value": 59654602,
            "range": "± 468901",
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
          "id": "860af68fbdd48baed7866002adce016a95304303",
          "message": "feat: initial release of ifchange",
          "timestamp": "2026-03-07T09:43:46+01:00",
          "tree_id": "4872f3460f1cae3bb90653611c6ec4c8269027fb",
          "url": "https://github.com/slnc/ifchange/commit/860af68fbdd48baed7866002adce016a95304303"
        },
        "date": 1772873318992,
        "tool": "cargo",
        "benches": [
          {
            "name": "lint_latency_16kloc_diff",
            "value": 2111394,
            "range": "± 62544",
            "unit": "ns/iter"
          },
          {
            "name": "lint_1000_files",
            "value": 7158866,
            "range": "± 87419",
            "unit": "ns/iter"
          },
          {
            "name": "lint_5000_files",
            "value": 37877000,
            "range": "± 1142752",
            "unit": "ns/iter"
          },
          {
            "name": "scan_5000_files",
            "value": 60786185,
            "range": "± 717259",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "41898282+github-actions[bot]@users.noreply.github.com",
            "name": "github-actions[bot]",
            "username": "github-actions[bot]"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "f518f768e88f1932c2762337fdab39d0f332ebe3",
          "message": "chore(main): release 0.1.0 (#10)\n\nCo-authored-by: github-actions[bot] <41898282+github-actions[bot]@users.noreply.github.com>",
          "timestamp": "2026-03-07T09:51:07+01:00",
          "tree_id": "10ea106dff07decd29852a32ef233387db1bd540",
          "url": "https://github.com/slnc/ifchange/commit/f518f768e88f1932c2762337fdab39d0f332ebe3"
        },
        "date": 1772873575928,
        "tool": "cargo",
        "benches": [
          {
            "name": "lint_latency_16kloc_diff",
            "value": 2110850,
            "range": "± 11238",
            "unit": "ns/iter"
          },
          {
            "name": "lint_1000_files",
            "value": 7051532,
            "range": "± 336487",
            "unit": "ns/iter"
          },
          {
            "name": "lint_5000_files",
            "value": 38576291,
            "range": "± 1707462",
            "unit": "ns/iter"
          },
          {
            "name": "scan_5000_files",
            "value": 62027414,
            "range": "± 404876",
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
          "id": "2b3bb38ed02feeb9daf4bb6dd9eec0b3953ed988",
          "message": "ci: chain release-binaries from release-please\n\nGITHUB_TOKEN actions don't trigger other workflows, so the v* tag\ncreated by release-please never triggered release-binaries. Fix by\nhaving release-please call release-binaries via workflow_call when\na release is created.",
          "timestamp": "2026-03-07T09:56:36+01:00",
          "tree_id": "bcda513dc236c123635084c3d7ac5e46d0be518c",
          "url": "https://github.com/slnc/ifchange/commit/2b3bb38ed02feeb9daf4bb6dd9eec0b3953ed988"
        },
        "date": 1772873904032,
        "tool": "cargo",
        "benches": [
          {
            "name": "lint_latency_16kloc_diff",
            "value": 2202626,
            "range": "± 28845",
            "unit": "ns/iter"
          },
          {
            "name": "lint_1000_files",
            "value": 7397559,
            "range": "± 57302",
            "unit": "ns/iter"
          },
          {
            "name": "lint_5000_files",
            "value": 38332446,
            "range": "± 1567639",
            "unit": "ns/iter"
          },
          {
            "name": "scan_5000_files",
            "value": 70335476,
            "range": "± 655046",
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
          "id": "4a17adb7ac03418ed74debb10d76f0f6db3f9657",
          "message": "fix: configure git identity for tag operations in release workflow",
          "timestamp": "2026-03-07T10:01:22+01:00",
          "tree_id": "a751c795b5430570a68ab6291f764e2c6f2cbb94",
          "url": "https://github.com/slnc/ifchange/commit/4a17adb7ac03418ed74debb10d76f0f6db3f9657"
        },
        "date": 1772874187837,
        "tool": "cargo",
        "benches": [
          {
            "name": "lint_latency_16kloc_diff",
            "value": 2111485,
            "range": "± 20940",
            "unit": "ns/iter"
          },
          {
            "name": "lint_1000_files",
            "value": 7067859,
            "range": "± 39546",
            "unit": "ns/iter"
          },
          {
            "name": "lint_5000_files",
            "value": 36750686,
            "range": "± 1278693",
            "unit": "ns/iter"
          },
          {
            "name": "scan_5000_files",
            "value": 58986375,
            "range": "± 406853",
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
          "id": "dc0e1f1a4e98ddfd68019a117ecccae42d89c6a7",
          "message": "fix: default installer version channel to latest",
          "timestamp": "2026-03-07T11:43:24+01:00",
          "tree_id": "9485fbe879d1259b4070fe9422e715df4a307fd6",
          "url": "https://github.com/slnc/ifchange/commit/dc0e1f1a4e98ddfd68019a117ecccae42d89c6a7"
        },
        "date": 1772880741512,
        "tool": "cargo",
        "benches": [
          {
            "name": "lint_latency_16kloc_diff",
            "value": 2115715,
            "range": "± 8423",
            "unit": "ns/iter"
          },
          {
            "name": "lint_1000_files",
            "value": 7156878,
            "range": "± 67282",
            "unit": "ns/iter"
          },
          {
            "name": "lint_5000_files",
            "value": 38390362,
            "range": "± 1784006",
            "unit": "ns/iter"
          },
          {
            "name": "scan_5000_files",
            "value": 60625551,
            "range": "± 786227",
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
          "id": "6792b48493cc156330c822efedb6a11720e1dbcf",
          "message": "ci: update action.yml",
          "timestamp": "2026-03-07T11:55:04+01:00",
          "tree_id": "a9ace8a85da939cc9bdeb17c4190bdd32bfcab57",
          "url": "https://github.com/slnc/ifchange/commit/6792b48493cc156330c822efedb6a11720e1dbcf"
        },
        "date": 1772881010241,
        "tool": "cargo",
        "benches": [
          {
            "name": "lint_latency_16kloc_diff",
            "value": 2119763,
            "range": "± 78418",
            "unit": "ns/iter"
          },
          {
            "name": "lint_1000_files",
            "value": 7054446,
            "range": "± 279322",
            "unit": "ns/iter"
          },
          {
            "name": "lint_5000_files",
            "value": 36554125,
            "range": "± 1575393",
            "unit": "ns/iter"
          },
          {
            "name": "scan_5000_files",
            "value": 59447464,
            "range": "± 642670",
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
          "id": "97cb88ecd3b7c8110e39d162cbe6f0f02ed991e4",
          "message": "ci: fix release-please",
          "timestamp": "2026-03-07T12:04:12+01:00",
          "tree_id": "06b5d1b7d8a57f5beafae1ae039f2efaf37d2a2a",
          "url": "https://github.com/slnc/ifchange/commit/97cb88ecd3b7c8110e39d162cbe6f0f02ed991e4"
        },
        "date": 1772881562412,
        "tool": "cargo",
        "benches": [
          {
            "name": "lint_latency_16kloc_diff",
            "value": 2120774,
            "range": "± 46003",
            "unit": "ns/iter"
          },
          {
            "name": "lint_1000_files",
            "value": 7206996,
            "range": "± 305807",
            "unit": "ns/iter"
          },
          {
            "name": "lint_5000_files",
            "value": 38571923,
            "range": "± 1211293",
            "unit": "ns/iter"
          },
          {
            "name": "scan_5000_files",
            "value": 62936909,
            "range": "± 1783074",
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
          "id": "7fea561b4e3c2bfe2a58ddcfd57225be3eb16b77",
          "message": "ci: move benchmark job into test workflow",
          "timestamp": "2026-03-07T12:08:33+01:00",
          "tree_id": "6c0e2250dfd3c8229660199da924a7795f249f42",
          "url": "https://github.com/slnc/ifchange/commit/7fea561b4e3c2bfe2a58ddcfd57225be3eb16b77"
        },
        "date": 1772881914016,
        "tool": "cargo",
        "benches": [
          {
            "name": "lint_latency_16kloc_diff",
            "value": 2112701,
            "range": "± 5809",
            "unit": "ns/iter"
          },
          {
            "name": "lint_1000_files",
            "value": 7022009,
            "range": "± 50171",
            "unit": "ns/iter"
          },
          {
            "name": "lint_5000_files",
            "value": 35931026,
            "range": "± 450920",
            "unit": "ns/iter"
          },
          {
            "name": "scan_5000_files",
            "value": 58882526,
            "range": "± 472420",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "69429+slnc@users.noreply.github.com",
            "name": "slnc",
            "username": "slnc"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "584112a4819e79fb0118f40a41ec333f8caa3229",
          "message": "fix: avoid direct interpolation of user args in action runner (#12)",
          "timestamp": "2026-03-07T12:43:10+01:00",
          "tree_id": "a87a70204db5a4d106181ed6050f45c46f41b246",
          "url": "https://github.com/slnc/ifchange/commit/584112a4819e79fb0118f40a41ec333f8caa3229"
        },
        "date": 1772883954407,
        "tool": "cargo",
        "benches": [
          {
            "name": "lint_latency_16kloc_diff",
            "value": 2110159,
            "range": "± 66883",
            "unit": "ns/iter"
          },
          {
            "name": "lint_1000_files",
            "value": 7107671,
            "range": "± 434535",
            "unit": "ns/iter"
          },
          {
            "name": "lint_5000_files",
            "value": 37528956,
            "range": "± 1399038",
            "unit": "ns/iter"
          },
          {
            "name": "scan_5000_files",
            "value": 60426113,
            "range": "± 666725",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}