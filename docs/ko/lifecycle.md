---
title: Lifecycle
parent: 언어 가이드
grand_parent: 한국어 문서
nav_order: 9
---

# Lifecycle

Lifecycle 블록은 `component` 내부의 1급 문법입니다.

구현된 형태:

- `awake`
- `start`
- `update`
- `fixedUpdate`
- `lateUpdate`
- `onEnable`
- `onDisable`
- `onDestroy`
- `onTriggerEnter`, `onTriggerExit`, `onTriggerStay`
- `onCollisionEnter`, `onCollisionExit`, `onCollisionStay`

예시:

```prsm
update {
    tick()
}
```

`require`, `optional`, `child`, `parent` 룩업은 사용자 `awake` 본문보다 먼저 처리됩니다.
