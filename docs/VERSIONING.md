# Versioning

`ifchange` follows [semver](https://semver.org/).

## Stable API surface

Breaking change = major bump post-1.0:

- CLI flags and their documented behavior
- Exit codes: `0` ok, `1` lint errors, `2` fatal error
- Error output format: `error: <file>:<line>: <message>` prefix and location
- Summary line format: `found N error(s) (...)`
- Directive syntax: `IfChange`, `ThenChange`, `Label`, `EndLabel`

## Not stable

May change in minor or patch releases:

- Exact error message wording after the location prefix
- Debug/verbose output format and content
- Color codes and terminal formatting
- Help text wording
- New lint rules or error types (adding rules is not a breaking change)

During `0.x`, minor versions may include breaking changes.
