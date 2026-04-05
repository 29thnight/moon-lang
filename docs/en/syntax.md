---
title: Syntax
parent: Language Guide
grand_parent: English Docs
nav_order: 1
---

# Syntax

PrSM keeps the surface syntax intentionally small.

- one file contains one top-level declaration
- files usually begin with `using` imports
- statements end by newline, not semicolon
- control flow is brace-based but parenthesis-free
- generated C# stays structurally close to source

Minimal file shape:

```prsm
using UnityEngine

component PlayerController : MonoBehaviour {
    update {
    }
}
```
