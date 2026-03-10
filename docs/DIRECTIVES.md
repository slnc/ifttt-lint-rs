# Directive Syntax Reference

Full reference for `ifchange` comment directives. For a quick-start overview, see the [README](../README.md#directive-syntax).

## Placement

Directives go at the start of a comment line (after optional whitespace). Mentions of `LINT.*` mid-comment are ignored. Supported in [128 file extensions](../README.md#supported-languages) with comment styles: `//`, `/* */`, `#`, `<!-- -->`, `--`, `%`, `;`, `'`, `!`, and more.

## Case Sensitivity

- **Directive keywords** are case-insensitive. `LINT.IfChange`, `lint.ifchange`, `LINT.THENCHANGE` all work.
- **File extensions** are case-insensitive. `FOO.CSS`, `foo.css`, `Foo.Css` are all recognized.
- **File paths and label names** are case-sensitive, matching git and Unix filesystem semantics.

## LINT.IfChange

All accepted formats:

```text
LINT.IfChange                    # bare (unlabeled)
LINT.IfChange("my-label")        # labeled, double quotes
LINT.IfChange('my-label')        # labeled, single quotes
LINT.IfChange(my-label)          # labeled, unquoted
```

## LINT.ThenChange

All accepted formats:

```text
LINT.ThenChange(other.py)                           # single target (relative to source file)
LINT.ThenChange("other.py#label")                   # with label reference
LINT.ThenChange(/src/config.py)                     # absolute: repo-root-relative
LINT.ThenChange("/src/config.py#label")             # absolute with label
LINT.ThenChange(#label)                             # self-reference (same file)
LINT.ThenChange("a.py", "b.py")                     # comma-separated
LINT.ThenChange(["a.ts", "config.py#db", "c.sql"])  # array syntax
```

Multi-line array (each line in its own comment):

```js
// LINT.ThenChange([
//   "constants.ts",
//   "config.py#db",
//   "schema.sql",
// ])
```

## Path Resolution

| Syntax | Resolves from | Example (source: `deploy/app.yml`) |
|--------|--------------|-------------------------------------|
| `other.py` | Source file's directory | `deploy/other.py` |
| `../src/config.py` | Source file's directory | `src/config.py` |
| `/src/config.py` | Repo root | `src/config.py` |
| `#label` | Same file | `deploy/app.yml#label` |

A leading `/` means **repo-root-relative**, not filesystem-absolute. `ifchange` detects the repo root by walking up from CWD looking for `.git`, `.hg`, `.jj`, `.svn`, `.pijul`, `.fslckout`, or `_FOSSIL_`. `.` and `..` components are normalized.

## LINT.Label / LINT.EndLabel

All accepted formats:

```text
LINT.Label("name")     # double quotes
LINT.Label('name')     # single quotes
LINT.Label(name)       # unquoted
LINT.EndLabel          # closes the labeled section
```

Label names can contain letters, numbers, hyphens, underscores, and dots.
