---
title: Version Migration
parent: Language Guide
grand_parent: English Docs
nav_order: 13
---

# Version Migration

This page covers migrating between PrSM language versions. Each version is fully backward-compatible — all programs from the previous version compile without changes.

---

## PrSM 1 → PrSM 2

### Activation

```toml
[language]
version = "2"
features = ["pattern-bindings", "input-system", "auto-unlisten"]
```

### What's new

| Feature | Description |
|---------|-------------|
| Pattern matching with bindings | `EnemyState.Chase(target) =>` in `when` branches |
| Listen lifetime model | `until disable`, `until destroy`, `manual` + `unlisten` |
| Destructuring | `val Stats(hp, speed) = getStats()` |
| New Input System sugar | `input.action("Jump").pressed` |
| Generic type inference | `val rb: Rigidbody = get()` |

### Breaking changes

None. `listen` without a modifier is unchanged (register-only). Pattern bindings are now validated — previously unchecked patterns may produce E081/E082.

### Migration steps

1. Set `version = "2"` in `.prsmproject`
2. Add desired features to `features` array
3. Run `prism build` — fix any E081/E082/E083 diagnostics
4. Adopt new features incrementally:
   - Add `until disable` to long-lived listen statements
   - Replace manual cleanup intrinsic blocks with `unlisten`
   - Use `input.action()` instead of legacy `input.getKey()`

### Rollback

Change `version = "1"` and remove v2-only syntax (`until disable`, `unlisten`, pattern bindings).

---

## PrSM 2 → PrSM 3

### Activation

```toml
[language]
version = "3"
features = ["pattern-bindings", "input-system", "auto-unlisten", "interface", "generics", "singleton", "pool", "solid-analysis", "optimizer"]
```

Or use the Unity Editor GUI: **Window > PrSM > Project Settings**.

### What's new

| Feature | Description |
|---------|-------------|
| Interface declaration | `interface IDamageable { func takeDamage(amount: Int) }` |
| Generic declaration | `class Registry<T> where T : Component { }` |
| `singleton` keyword | `singleton component GameManager : MonoBehaviour { }` |
| `pool` modifier | `pool bullets: Bullet(capacity = 20, max = 100)` |
| SOLID analysis | W010/W011/W012 warnings for design issues |
| Code optimizer | Single-binding destructure inlining |
| Reserved names | `get`, `find` are reserved built-in method names (E101) |

### Breaking changes

**E101 — Reserved method names:** If you have user-defined functions named `get` or `find`, rename them. These names are reserved for built-in sugar methods (`GetComponent`, `FindFirstObjectByType`).

```prsm
// Before (v2) — compiles but may conflict with sugar
func get(): Item { return items[0] }

// After (v3) — rename to avoid E101
func getItem(): Item { return items[0] }
```

### Migration steps

1. Set `version = "3"` in `.prsmproject` (or use **Window > PrSM > Project Settings**)
2. Run `prism build` — fix any E101 diagnostics (rename `get`/`find` functions)
3. Review SOLID warnings (W010/W011/W012) and refactor if desired
4. Adopt new features incrementally:
   - Define interfaces for component contracts
   - Use `singleton` instead of manual singleton boilerplate
   - Use `pool` instead of manual object pool management
   - Add generics to utility classes

### Rollback

Change `version = "2"` and remove v3-only syntax (`interface`, generic params, `singleton`, `pool`).

---

## Feature flag reference

| Flag | Since | Description |
|------|-------|-------------|
| `pattern-bindings` | PrSM 2 | Enum payload binding, destructuring, when guards |
| `input-system` | PrSM 2 | Input System sugar (requires Unity Input System package) |
| `auto-unlisten` | PrSM 2 | Listen lifetime modifiers and unlisten |
| `interface` | PrSM 3 | Interface declaration |
| `generics` | PrSM 3 | Generic class/func with where clauses |
| `singleton` | PrSM 3 | Singleton component keyword |
| `pool` | PrSM 3 | Object pool modifier |
| `solid-analysis` | PrSM 3 | SOLID analysis warnings |
| `optimizer` | PrSM 3 | Code optimizer |

Setting `version = "N"` implicitly enables all features for that version and below.
