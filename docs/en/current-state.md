---
title: Current State
parent: Internals
grand_parent: English Docs
nav_order: 2
---

# Current State

As of the current repository snapshot:

- lexer, parser, semantic analysis, lowering, and code generation are implemented
- the `prism` CLI is implemented and verified in-repo
- Unity package integration is implemented
- trusted-workspace `prism lsp` support is implemented for completion, definition, hover, references, rename, and document/workspace symbols
- VS Code hover now stays on the LSP path while the extension layers generated C# enrichment on top when available
- generated C# back-mapping through `.prsmmap.json` is implemented in both the VS Code extension and Unity package tooling
- in-repo lowering regression coverage includes event-listener sugar and intrinsic escape-hatch paths
- BlazeTest smoke coverage and package-level editor tests already exist
- VS Code extension distribution now has automated VSIX packaging, bundled artifact verification, and isolated install smoke coverage

What is still incomplete:

- broader negative test coverage
- deeper debugging and source-map workflows beyond the current foundation
- broader end-to-end validation across real Unity projects
