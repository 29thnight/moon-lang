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
- compiler-backed VS Code navigation is implemented, including definition, hover, references, rename, and symbols
- generated C# back-mapping through `.prsmmap.json` is implemented in both the VS Code extension and Unity package tooling
- in-repo lowering regression coverage includes event-listener sugar and intrinsic escape-hatch paths
- BlazeTest smoke coverage and package-level editor tests already exist

What is still incomplete:

- broader negative test coverage
- a full dedicated LSP server
- deeper debugging and source-map workflows beyond the current foundation
