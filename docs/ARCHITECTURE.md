# Architecture

## System Overview

```
              ┌──────────────────────────────┐
              │           app.rs             │
              │  CLI (clap) + input routing  │
              └────────────────┬─────────────┘
                               │
              ┌────────────────┴───────────────┐
              ▼                                ▼
  ┌─── lint mode ────────────┐   ┌─── scan mode ───────────┐
  │                          │   │                         │
  │  diff/parser             │   │  Walk directory (rayon) │
  │  → engine/lint (3-phase) │   │  → parse each file      │
  │                          │   │  → validate directives  │
  └──────────────────────────┘   └─────────────────────────┘
              │                                │
              └────────────────┬───────────────┘
                               ▼
                 ┌──────────────────────────┐
                 │    directive/parse       │
                 │                          │
                 │  comment extraction      │
                 │  → regex matching        │
                 │  → directive pairing     │
                 └──────────────────────────┘
```

The CLI has two modes:

- **Lint mode** (default): takes a unified diff, finds which files changed and where, then validates that all `ThenChange` targets were also modified.
- **Scan mode** (`-s <dir>`): walks a directory tree and validates directive *syntax* (mismatched pairs, duplicate labels, malformed directives) without needing a diff. Useful as a static check in CI or editors.

Both modes share the directive parsing layer but differ in what they validate.

## Lint Mode: 3-Phase Algorithm

The phasing exists because target files may not appear in the diff, so we can't parse everything in one pass.

**Phase 1: Parse changed files** (parallel)
For each file in the diff:

```
file ──▶ read ──▶ extract comments ──▶ match directives ──▶ pair IfChange↔ThenChange ──▶ resolve target paths
```

**Phase 2: Parse target files** (parallel)
For each target file not already parsed in Phase 1: read → extract directives → build label→line-range index. A directive cache avoids re-parsing files seen in Phase 1.

**Phase 3: Validate pairs** (sequential)
For each IfChange→ThenChange pair, check whether the target file (or labeled region) was also modified in the diff.

The key subtlety is cross-reference detection: when two files reference each other via `IfChange` blocks, only changes within an `IfChange` region trigger validation. Without this, mutual references would always fire. A change to file `A` triggers a check on file `B`, whose own `IfChange` pointing back at A would trigger again. The cross-reference logic breaks this cycle by narrowing the trigger condition.

## Scan Mode

Single pass: walk directory tree (via `ignore` crate, respects `.gitignore`) → parse each file in parallel → report structural errors (orphan `IfChange` without `ThenChange`, duplicate labels, malformed directives). No diff needed, no cross-file validation.

## Key Design Decisions

- **Custom diff parser.** No external diff library. The parser only needs file paths and changed line numbers from unified diffs, so ~200 lines of purpose-built code keeps dependencies minimal.
- **Parallel phases, sequential validation.** Phases 1 and 2 are I/O-bound (file reads) and embarrassingly parallel via rayon. Phase 3 is CPU-light (set lookups) and sequential to avoid synchronization overhead on the shared results vector.
- **Comment extraction with string-literal tracking.** The extractor skips directive-like patterns inside string literals to avoid false positives. This matters for languages where `"// LINT.IfChange"` in a string shouldn't trigger linting.
- **Extension-based comment style dispatch.** 50+ extensions mapped to 7 comment styles. Unknown extensions try C-style first, then hash. Covers virtually all codebases without configuration.

## Best Alternative: Single-Pass Streaming Architecture

Instead of batching work into phases, you process diff hunks as they arrive from stdin, extract directives, and register them into a concurrent dependency graph. The moment both sides of an IfChange/ThenChange pair are resolved (because the target was already seen in the diff or loaded on-demand into a concurrent cache), validation fires and emits the result. No phase boundaries exist.

This maps to the problem's actual dependency structure: validation of a pair depends only on that pair's source and target being parsed, not on every file in the diff being parsed first. The phased model imposes a total order where the true dependency graph is sparse.

- **Latency:** validation results for early files emerge before the last file has been parsed. The critical path follows data dependencies, not phase barriers.
- **Memory:** parsed directive data for resolved pairs can be released immediately rather than held until Phase 3 completes.
- **Composability:** a streaming architecture composes with piped workflows. Hunks from `git diff` feed directly into the validator without materializing the entire diff, and diagnostics emit incrementally to downstream consumers.

A concurrent file cache with lock-free reads for target files avoids redundant I/O without requiring an explicit "collect all targets, then batch-load" step. Parsing, I/O, and validation overlap across the full execution timeline rather than serializing into three discrete stages.

The phased architecture was chosen because the tool runs in 15 to 28ms on 5,000 files, which means the streaming model's advantages are unmeasurable in practice while its implementation complexity (concurrent dependency tracking, cache invalidation, incremental constraint resolution) is very real.
