---
title: Generated C# & Source Maps
parent: Tooling
grand_parent: English Docs
nav_order: 4
---

# Generated C# & Source Maps

PrSM compiles source into readable generated C# and emits source-map sidecars for back-mapping.

Artifacts:

- source: `.prsm`
- generated code: `.cs`
- sidecar source map: `.prsmmap.json`

These source maps are already used by the compiler workflow, VS Code extension, and Unity integration to map diagnostics and navigation back to the original source.
