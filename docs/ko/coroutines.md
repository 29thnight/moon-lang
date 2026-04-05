---
title: Coroutines
parent: 언어 가이드
grand_parent: 한국어 문서
nav_order: 10
---

# Coroutines

코루틴은 PrSM의 핵심 기능 중 하나입니다.

```prsm
coroutine hitInvincible() {
    invincible = true
    wait invincibleTime.s
    invincible = false
}
```

지원되는 wait 형식:

- `wait 1.0s`
- `wait nextFrame`
- `wait fixedFrame`
- `wait until condition`
- `wait while condition`

코루틴 제어 형식:

- `start coroutineName()`
- `stop coroutineName()`
- `stopAll()`
