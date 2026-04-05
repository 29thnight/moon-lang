---
title: Events & Intrinsic
parent: Language Guide
grand_parent: English Docs
nav_order: 11
---

# Events & Intrinsic

## Event listener sugar

PrSM includes `listen` syntax for Unity event wiring.

```prsm
listen startButton.onClick {
    SceneManager.loadScene("Game")
}
```

This lowers to `AddListener(...)` calls in generated C#.

## Intrinsic escape hatch

PrSM also includes `intrinsic` forms for raw C# insertion.

- statement block intrinsic
- typed intrinsic expression
- intrinsic function
- intrinsic coroutine

These forms are intentionally outside normal PrSM semantic checking.
