# Vendor audit

Date: 2026-09-28

This repository's vendored crates were checked for dependency sources that escape
the repository.

## What was checked

- All `vendor/**/Cargo.toml` manifests were scanned for `git = ...` dependency
  sources.
- All vendored dependency `path = ...` entries were checked for paths that point
  outside `vendor/`.
- The working tree was checked for any untracked vendored crates.

## Verification commands

```sh
git ls-files --others --exclude-standard vendor
rg '^\s*git\s*=' vendor --glob '**/Cargo.toml'
rg '^\s*path\s*=\s*"\.\./' vendor --glob '**/Cargo.toml'
rg '^\s*path\s*=\s*"/' vendor --glob '**/Cargo.toml'
rg '^\s*path\s*=\s*"\.\./\.\./' vendor --glob '**/Cargo.toml'
```

## Result

- No vendored crate declares a `git` dependency source.
- No vendored crate declares a dependency path outside `vendor/`.
- No untracked vendored crates remain.

## Conclusion

The vendored dependency graph is tracked in this repository.
