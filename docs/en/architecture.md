---
title: Architecture
parent: Internals
grand_parent: English Docs
nav_order: 1
---

# Architecture

The current compiler pipeline is:

```text
.prsm source
  -> Lexer
  -> Parser
  -> Semantic Analysis
  -> Lowering to C# IR
  -> C# emission
  -> .cs + .prsmmap.json output
```

Primary module responsibilities today:

- `lexer`: tokenization, interpolation, duration literals, source positions
- `parser`: recursive descent parsing and error recovery
- `semantic`: symbol resolution, type checks, null safety, validation
- `lowering`: AST to C# IR
- `codegen`: formatted C# emission
- `driver` and project graph/index helpers: CLI-facing orchestration
