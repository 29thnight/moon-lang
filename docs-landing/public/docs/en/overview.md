---
title: Overview
parent: Introduction
nav_order: 1
---

# Overview

PrSM uses `.prsm` source files and compiles them into generated C# for Unity projects. The compiler also emits `.prsmmap.json` sidecars so diagnostics and navigation can be mapped back to source.

The repository is centered on four working areas:

- `crates/refraction`: compiler core and the `prism` CLI
- `unity-package`: Unity Editor integration and generated-code workflow support
- `vscode-prsm`: syntax, diagnostics, navigation, snippets, and generated source-map helpers
- `samples`: local validation and regression fixtures

PrSM is not positioned as a general-purpose language. Its design center is concise Unity gameplay code with strong null-safety, first-class lifecycle syntax, explicit component lookup, coroutine support, and readable generated C#.
