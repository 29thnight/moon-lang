---
title: VS Code Extension
parent: Tooling
grand_parent: English Docs
nav_order: 3
---

# VS Code Extension

The `vscode-prsm` package already provides a compiler-backed editing workflow without a dedicated LSP server.

## Current Capabilities

- TextMate syntax highlighting and snippets
- Unity-aware completion data plus project-aware completion helpers
- diagnostics through `prism check --json` on open, save, and debounced edits in trusted workspaces
- go-to-definition, hover, references, and rename through compiler-backed CLI queries
- document symbols and workspace symbols through cached `prism index` data
- project discovery through `.prsmproject`
- generated C# navigation through `.prsmmap.json`
- jump from PrSM source to generated C# and back again
- explorer, graph view, lifecycle insertion, and compile/check commands

## Trusted Workspace Behavior

The extension keeps syntax assets and basic editing help available everywhere, but the compiler-driven features activate only in trusted workspaces.

That affects diagnostics, navigation, rename, symbol queries, and compile commands.

## Navigation Model

The extension is intentionally compiler-backed:

- definition prefers `prism definition`
- hover and fallback navigation use `prism index`
- references and rename use Typed HIR-backed `prism references`
- generated C# lookups consult `.prsmmap.json` sidecars before falling back to looser heuristics

The roadmap still treats a full LSP server as future work, but the current extension already covers the main authoring loop for real projects.
