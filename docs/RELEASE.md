# Release Workflow

This project uses [release-please](https://github.com/googleapis/release-please) for automated versioning and changelog generation, and [commitlint](https://commitlint.js.org/) for enforcing conventional commit format on PR titles.

## Conventional Commits

All PR titles must follow the [Conventional Commits](https://www.conventionalcommits.org/) format. Since we squash-merge, the PR title becomes the commit message on `main`.

### Format

```
<type>[optional scope][!]: <description>
```

### Types and Semver Impact

| Type | Description | Semver bump |
|------|-------------|-------------|
| `feat` | New feature | minor |
| `feat!` | Breaking feature | major (minor while 0.x) |
| `fix` | Bug fix | patch |
| `perf` | Performance improvement | patch |
| `refactor` | Code refactoring | patch |
| `docs` | Documentation only | (hidden) |
| `build` | Build system changes | (hidden) |
| `ci` | CI configuration | (hidden) |
| `test` | Tests only | (hidden) |
| `chore` | Maintenance | (hidden) |

Hidden types still trigger a release if combined with visible types in the same release cycle, but won't appear in the changelog on their own.

Adding `BREAKING CHANGE` in the commit body or `!` after the type triggers a major bump (minor while on 0.x).

### Examples

```
feat: add --json output flag
fix: handle empty diff input without panic
perf: reduce memory allocation in directive parser
feat!: rename --check to --lint
docs: update CLI usage in README
```

## Release Workflow

```
1. Open PR with conventional commit title
   -> commitlint CI validates the title

2. Merge PR (squash) to main
   -> release-please analyzes commits since last release
   -> Creates/updates a Release PR with:
      - Version bump in Cargo.toml, npm/package.json, pypi/pyproject.toml
      - Updated CHANGELOG.md

3. Review and merge the Release PR
   -> release-please creates git tag (e.g. v0.2.0) + GitHub Release
   -> release-binaries.yml triggers on v* tag
   -> Builds binaries, signs, publishes to crates.io/npm/PyPI
```

## Release Checklist

Before merging a Release PR:

- [ ] Version bump looks correct for the changes included
- [ ] CHANGELOG entries are accurate
- [ ] CI is green on the Release PR
- [ ] No pending critical fixes that should be included

## Manual Overrides

### Force a specific bump level

Add a commit (or PR) with:
- `fix:` for patch
- `feat:` for minor
- `feat!:` or `BREAKING CHANGE` in body for major

### Skip a release

Simply don't merge the Release PR. It will continue accumulating changes.

### Emergency hotfix

1. Create a `fix:` PR targeting `main`
2. Merge it — release-please updates the Release PR
3. Merge the Release PR immediately
