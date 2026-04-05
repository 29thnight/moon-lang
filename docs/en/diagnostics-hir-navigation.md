---
title: Diagnostics, HIR, and Navigation
parent: Advanced
grand_parent: English Docs
nav_order: 2
---

# Diagnostics, HIR, and Navigation

PrSM already exposes a layered analysis pipeline beyond plain code generation.

Current building blocks:

- semantic diagnostics from parsing and analysis
- JSON diagnostic output with precise start and end ranges
- Typed HIR emission for definitions and references
- definition lookup and project-wide reference lookup
- syntax-level project symbol and type-reference index queries

## Diagnostics

Use `prism check --json` or `prism build --json` to get machine-readable diagnostics.

Each diagnostic includes:

- a stable code and severity
- message text
- file path
- `line` / `col`
- `end_line` / `end_col`

Those end positions are what the VS Code extension and Unity-side tooling use for precise highlighting and remapping.

## Typed HIR

`prism hir . --json` emits per-file Typed HIR data.

That output includes:

- definitions with kind, qualified name, type, mutability, and exact span
- references with kind, exact span, and resolved definition ids when known
- project-level stats when HIR is produced through `build`

Typed HIR is the semantic backbone for navigation commands such as definition and references.

## Definition and References

The navigation-oriented CLI commands are position-based:

```powershell
cargo run -p refraction --bin prism -- definition . --json --file samples\PlayerController.prsm --line 10 --col 5
cargo run -p refraction --bin prism -- references . --json --file samples\PlayerController.prsm --line 10 --col 5
```

`definition` resolves the declaration behind the symbol at that position.

`references` resolves the owning definition first, then returns the project-wide reference set for the same qualified symbol.

## Project Index

`prism index` is the cheaper syntax-level companion to Typed HIR.

It supports:

- full project symbol listing
- exact `--symbol` filtering
- exact `--qualified-name` filtering
- position lookup through `--file`, `--line`, and `--col`

Position lookups can return both:

- `symbol_at` for the enclosing declaration/member
- `reference_at` for type-reference positions such as field annotations, parameter types, and declaration headers

This makes the index useful even when semantic lookup is unavailable or intentionally skipped.

## Editor Integration

The VS Code extension uses these layers together:

- diagnostics come from `prism check --json`
- go-to-definition prefers `prism definition`
- hover and navigation fall back to `prism index`
- references and rename are compiler-backed through Typed HIR
- document and workspace symbols are built from cached index results

So while a dedicated LSP server is still future work, the existing CLI and extension already provide a real analysis-driven editing workflow.
