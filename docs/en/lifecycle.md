---
title: Lifecycle
parent: Language Guide
grand_parent: English Docs
nav_order: 9
---

# Lifecycle

Lifecycle blocks are first-class syntax inside `component` declarations.

Implemented lifecycle forms include:

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

Example:

```prsm
update {
    tick()
}
```

`require`, `optional`, `child`, and `parent` lookups are resolved before the user `awake` body runs.
