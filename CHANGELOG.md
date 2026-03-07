# Changelog

## 0.1.0 (2026-03-07)


### Features

* initial release of ifchange ([860af68](https://github.com/slnc/ifchange/commit/860af68fbdd48baed7866002adce016a95304303))

## 0.1.0 (2026-03-07)


### Features

* add GitHub Action for lint-ifchange ([4ce430a](https://github.com/slnc/ifchange/commit/4ce430a6fbcc114e5f510034b21680665439d568))
* add HTML comment support for md/html/xml ([b9db6b0](https://github.com/slnc/ifchange/commit/b9db6b0ee14e54543b40e0f0fb56837733280bc1))
* add lint-ifchange to pre-commit hook ([d8f71cf](https://github.com/slnc/ifchange/commit/d8f71cfb2c074fa8155807340deecaa13a170151))
* add multi-channel distribution ([c1a6048](https://github.com/slnc/ifchange/commit/c1a6048c053984e1ab3a79b8aec5d35aa77ba724))
* case-insensitive LINT directive matching ([6a810e0](https://github.com/slnc/ifchange/commit/6a810e0dd1d2acbe54d36ba047fde98fb33f5377))
* implement ifttt-lint-rs ([4da8921](https://github.com/slnc/ifchange/commit/4da8921bde98dd82158a160e5a8825df10b00eee))
* improve DX with verbose, debug, colors ([a77f704](https://github.com/slnc/ifchange/commit/a77f7041cbcf46cfbf53427abc97e2009d4079f1))
* optimize verbose/debug output ([5660897](https://github.com/slnc/ifchange/commit/5660897f3e7c17274557d0c8e089f81c97941552))
* run check and lint together by default ([b2c8ecd](https://github.com/slnc/ifchange/commit/b2c8ecd1f9310e965422e872205c5ff88527ecec))
* support Dockerfile.* variants ([e412b31](https://github.com/slnc/ifchange/commit/e412b311257c9b054a7f39d8817d5433989e8c2c))
* support unquoted targets and labels ([64632b2](https://github.com/slnc/ifchange/commit/64632b21e3280b42f0d62782742944ad2653ba9e))


### Bug Fixes

* add clippy to pre-commit and fix lint errors ([a12a9b1](https://github.com/slnc/ifchange/commit/a12a9b1b0659751891a5852fe40d1286b84b13b8))
* benchmark CI output format for parser ([f53080c](https://github.com/slnc/ifchange/commit/f53080ce4381fbb0728ad1693c15789174ccbdac))
* broken benchmark CI and stale ThenChange path ([ea25aff](https://github.com/slnc/ifchange/commit/ea25affd607eb4b1f250686a041c820e2cf74d20))
* correct comment style classifications ([6d05990](https://github.com/slnc/ifchange/commit/6d0599063433210c1b09d8c0c21c847d31eeb796))
* correct release-please-action pinned SHA ([83a8147](https://github.com/slnc/ifchange/commit/83a8147f113b0f2eccabc1042a014b192e2f1f40))
* match directives only at start of comment ([5c4d37a](https://github.com/slnc/ifchange/commit/5c4d37a123daf4e9b0dbb4948e8d188647bc5cce))
* normalize remaining Windows path in integration test ([94e0585](https://github.com/slnc/ifchange/commit/94e058596296dc1a29ab8c33d1f0cb1de578e2b0))
* normalize Windows path separators in target resolution ([25c49eb](https://github.com/slnc/ifchange/commit/25c49eb50e377d767792cb61d02a6d27fa2887aa))
* pass repository to Renovate self-hosted action ([9de5682](https://github.com/slnc/ifchange/commit/9de56827de3d20582389e5b54c4a657fd3da0402))
* pin cosign-installer to v4.0.0 (no rolling major tag) ([da72d43](https://github.com/slnc/ifchange/commit/da72d430f9bbc96c1c22cce5c14d2b84f32124ab))
* resolve action repo for local (uses: ./) and remote usage ([29b88ac](https://github.com/slnc/ifchange/commit/29b88acd736f972543233ee6b96505111152a119))
* resolve failing CI workflows ([ae6fe4b](https://github.com/slnc/ifchange/commit/ae6fe4befa4476e5257d2ff8a8b130577a90acf5))
* run native binary directly in npm/pypi ([e3886ab](https://github.com/slnc/ifchange/commit/e3886ab8f0af7e5d26249d7882aca545408cb1b8))
* run native binary directly in npm/pypi ([c287acf](https://github.com/slnc/ifchange/commit/c287acf80e39d2e3dc6b012054791d8a67b3231c))
* scan skips files with lowercase LINT directives ([db52503](https://github.com/slnc/ifchange/commit/db52503623b865bb6007aed73f598a529da6eba7))
* skip binary download when lint-ifchange is already on PATH ([888794d](https://github.com/slnc/ifchange/commit/888794dcae02fdcc1d8d9bc96f4de9bbb58b9a97))
* support multi-line ThenChange across line comments ([62c5c3d](https://github.com/slnc/ifchange/commit/62c5c3dcd783f4f82578221841fc3f355ca162b4))
* update cosign to use --bundle flag ([e527cf8](https://github.com/slnc/ifchange/commit/e527cf81a0da36e369fe7c2d7606d81fa2700d12))
* use cargo install --path . for dogfood workflow ([923d4bf](https://github.com/slnc/ifchange/commit/923d4bf291f31ff1c02b72d3bde480a242e13425))
* use correct renovatebot/github-action version (v46) ([990e3ee](https://github.com/slnc/ifchange/commit/990e3ee8633e94aa22ff037e980122517c92e962))
* use full version tag for renovatebot/github-action ([197c305](https://github.com/slnc/ifchange/commit/197c305d93705acfaa6194c99f589b5a12ca85e3))
* use valid Renovate config to suppress issue creation ([a8ae9e5](https://github.com/slnc/ifchange/commit/a8ae9e54d946872070a03658238fedb238375c99))
* use valid Renovate schedule syntax for vulnerability alerts ([d19d679](https://github.com/slnc/ifchange/commit/d19d679b3891e06b52574bdba8d1fdb1e1ac365e))
* Windows path separator handling in tests and path resolution ([1ed3537](https://github.com/slnc/ifchange/commit/1ed353782c471b8185bba335174167a16fee6305))


### Performance

* eliminate unnecessary String clones in lint engine ([1f9de04](https://github.com/slnc/ifchange/commit/1f9de041438d29af279395c917448b49d922c964))


### Refactoring

* clean up justfile recipes ([1e6a543](https://github.com/slnc/ifchange/commit/1e6a543dc06ef344c7e0996a0df43128c35e8648))
* improve module naming and structure ([f2038b3](https://github.com/slnc/ifchange/commit/f2038b3861b42f9910732209c4600beceb8712f0))
* remove 64MB input size limit ([5f07e19](https://github.com/slnc/ifchange/commit/5f07e192c99ff791404f2490a4dc54eded1ab1dc))
* remove dead code and fix minor inefficiencies ([75718ba](https://github.com/slnc/ifchange/commit/75718ba6cbcfed82e27b9e08ef509460464d50aa))
* rename CLI flags for clarity ([8af99dd](https://github.com/slnc/ifchange/commit/8af99dd6320a5e7d743888d9fc8abc0e421368e3))
* rename perf comparison script ([c02277f](https://github.com/slnc/ifchange/commit/c02277ff9e80c4c6aa533a0d3910814b229fb1f2))
* simplify examples and rename to ifchange ([5c91e6c](https://github.com/slnc/ifchange/commit/5c91e6c1bf5b700da33b24032abfaa524c3f197b))
* split integration tests by module ([c2db4f0](https://github.com/slnc/ifchange/commit/c2db4f0bd5c255d52485e0e4a928e567cf61ef01))
* unify comment extractors into single parameterized parser ([d4144cf](https://github.com/slnc/ifchange/commit/d4144cf6bcbe70227f86f4aa409c721c46e6264c))
