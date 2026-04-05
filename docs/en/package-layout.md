---
title: Package Layout
parent: Internals
grand_parent: English Docs
nav_order: 7
---

# Package Layout

Current high-level repository layout:

- `crates/refraction`: compiler crate and `prism` binary
- `unity-package`: Unity package source, editor integration, templates, and tests
- `vscode-prsm`: extension source, grammar, snippets, themes, and tests
- `samples`: example `.prsm` scripts and generated comparisons
- `tests`: invalid and negative source fixtures
- `build-output`: generated sample outputs and smoke artifacts
- `plan_docs`: roadmap, spec, architecture, and design notes
- `docs`: GitHub Pages oriented documentation tree

This split reflects the real product shape: language compiler, Unity integration, and editor tooling developed together in one repository.
