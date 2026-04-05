---
title: CLI
parent: Tooling
grand_parent: English Docs
nav_order: 1
---

# CLI

The current `prism` CLI is split across build, analysis, navigation, and utility commands.

## Build and Validation

- `compile <path>` compiles a file or directory of `.prsm` sources
- `check <path>` runs diagnostics without generating C# output
- `build` discovers `.prsmproject` from the current working directory and builds the configured project

Useful flags:

- `--output` for `compile`
- `--json` for machine-readable output
- `--watch` for `build`
- `--no-warnings` for quieter `compile` output

## Analysis and Navigation

- `hir [path] --json`
- `definition [path] --file ... --line ... --col ... --json`
- `references [path] --file ... --line ... --col ... --json`
- `index [path] --json`

`index` also supports:

- `--symbol <name>`
- `--qualified-name <name>`
- `--file --line --col` for position queries

## Utility Commands

- `init`
- `where`
- `version`

## Typical Usage

```powershell
cargo run -p refraction --bin prism -- compile samples\PlayerController.prsm --output build-output
cargo run -p refraction --bin prism -- check samples\PlayerController.prsm --json
cargo run -p refraction --bin prism -- build --json
cargo run -p refraction --bin prism -- hir . --json
cargo run -p refraction --bin prism -- definition . --json --file samples\PlayerController.prsm --line 10 --col 5
cargo run -p refraction --bin prism -- references . --json --file samples\PlayerController.prsm --line 10 --col 5
cargo run -p refraction --bin prism -- index . --json --symbol PlayerController
cargo run -p refraction --bin prism -- index . --json --file samples\PlayerController.prsm --line 10 --col 5
```

In practice, that means the CLI is not just a transpiler. It is also the analysis surface that powers the current editor experience.
