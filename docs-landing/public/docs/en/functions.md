---
title: Functions
parent: Language Guide
grand_parent: English Docs
nav_order: 8
---

# Functions

PrSM currently supports:

- block-bodied functions
- expression-bodied functions
- explicit return types
- `private`, `protected`, `public`
- `override`

Examples:

```prsm
func jump() {
    print("jump")
}

func isDead(): Bool = hp <= 0
```

Closures are not yet presented as a broad first-class language chapter. Event lambdas are produced by lowering for `listen`, but user-facing closure syntax is not a major documented feature today.
