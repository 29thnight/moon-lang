---
title: Overview
parent: Introduction
nav_order: 1
---

# Overview

PrSM (**P**ragmatic **R**educed **S**yntax for **M**etascript) is a statically typed, Unity-first scripting language that compiles `.prsm` source files into plain C#. There is no runtime library to ship — your Unity project just builds and runs the generated code normally.

The language targets the day-to-day gameplay scripting loop: components that react to lifecycle events, coroutines that sequence async logic, events that wire UI actions, and data that flows between systems. PrSM makes these patterns shorter to write without hiding what is happening underneath.

## Design goals

- **Familiar to Unity developers** — the generated C# looks like code a Unity developer would write by hand, so nothing is hidden
- **Concise for the common case** — lifecycle blocks, `require`/`optional`/`child` field qualifiers, `listen` event wiring, and coroutine syntax all eliminate boilerplate that is otherwise written the same way every time
- **Null-safety by default** — field qualifiers carry meaning at compile time so the compiler can reject missing or misused references before Unity processes the asset
- **Traceable** — `.prsmmap.json` sidecars record how each PrSM declaration maps to the generated C# output, enabling the VS Code extension and Unity editor to navigate, remap diagnostics, and unwind stack traces back to `.prsm` source

## Language versions

| Version | Tool release | Highlights |
|---------|--------------|------------|
| [PrSM 1](spec/lang-1.md) | Prism v0.x | Core component model, lifecycle blocks, `serialize`/`require`, `listen`, coroutines |
| [PrSM 2](spec/lang-2.md) | Prism v0.x | Pattern bindings, listen lifetimes, destructuring, Input System sugar, generic inference |
| [PrSM 3](spec/lang-3.md) | Prism v1.0.0 | Interfaces, generics, `singleton`, `pool`, SOLID warnings, optimizer |
| [PrSM 4](spec/lang-4.md) | Prism v2.0.0 | 30 features: try/catch, lambdas, properties, struct, async/await, state machine, command, bind, optimizer v4, Burst analysis, Rust/Elm-style errors, refactoring tools, debugger source maps |

Each version is fully backward-compatible — programs from earlier versions compile without changes. See [Version Migration](migration-v1-to-v2.md) for upgrade steps.

## Repository layout

| Path | Role |
|---|---|
| `crates/refraction` | Rust compiler and `prism` CLI (lexer, parser, semantic, lowering, codegen, LSP) |
| `unity-package` | Unity Editor integration: import hooks, diagnostic helpers, source-map consumer, stack-trace remapper |
| `vscode-prsm` | VS Code extension: syntax, snippets, diagnostics, navigation, LSP client |
| `samples` | Checked-in `.prsm` files used for smoke tests and regression validation |
| `roslyn-sidecar` | Optional .NET sidecar that enriches hover with Roslyn/Unity C# symbol data |

## Compiler pipeline

```text
.prsm source
  -> Lexer           token stream with source positions
  -> Parser          typed AST with identifier-level spans
  -> Semantic        HIR: symbol resolution, type checks, null safety
  -> Lowering        C# IR construction
  -> Codegen         formatted .cs output
  -> Source map      .prsmmap.json sidecar
```

Each stage feeds into the next. The Typed HIR produced by semantic analysis also backs the editor navigation commands (`definition`, `references`, `index`) and the LSP server.
