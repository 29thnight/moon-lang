---
title: Coroutines
parent: Language Guide
grand_parent: English Docs
nav_order: 10
---

# Coroutines

Coroutines are one of the core PrSM language features.

```prsm
coroutine hitInvincible() {
    invincible = true
    wait invincibleTime.s
    invincible = false
}
```

Supported wait forms:

- `wait 1.0s`
- `wait nextFrame`
- `wait fixedFrame`
- `wait until condition`
- `wait while condition`

Coroutine control forms:

- `start coroutineName()`
- `stop coroutineName()`
- `stopAll()`
