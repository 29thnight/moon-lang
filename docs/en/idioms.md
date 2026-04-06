---
title: Idioms & Patterns
parent: Language Guide
grand_parent: English Docs
nav_order: 14
---

# Idioms & Patterns

This page collects recommended patterns and common anti-patterns for PrSM development. Each section includes a short code example.

## Component design

Keep components focused on a single responsibility. Use `require` for hard dependencies that must exist at runtime and `optional` for soft dependencies that may or may not be present.

```prsm
component DamageReceiver : MonoBehaviour {
    require collider: Collider
    optional animator: Animator

    serialize maxHp: Int = 100
    var hp: Int = maxHp

    func takeDamage(amount: Int) {
        hp = Math.Max(0, hp - amount)
        animator?.SetTrigger("Hit")
        if hp <= 0 {
            die()
        }
    }
}
```

Avoid "god components" that handle movement, input, health, UI, and audio in one declaration. Split them into separate components and communicate through events or shared `asset` data.

## Event subscription patterns

### Preferred — auto-cleanup with `until disable`

The safest pattern in v2 components. The listener is automatically removed when the component is disabled, preventing stale callbacks on destroyed objects.

```prsm
component ShopUI : MonoBehaviour {
    require buyButton: Button
    require sellButton: Button

    listen buyButton.onClick until disable {
        purchaseSelectedItem()
    }

    listen sellButton.onClick until disable {
        sellSelectedItem()
    }
}
```

### Temporary listeners — manual lifetime

For listeners that should only be active for a limited time, capture the token and unlisten when done.

```prsm
component Tutorial : MonoBehaviour {
    require skipButton: Button

    var skipToken: ListenToken? = null

    func startTutorial() {
        skipToken = listen skipButton.onClick manual {
            endTutorial()
        }
    }

    func endTutorial() {
        if skipToken != null {
            unlisten skipToken!!
            skipToken = null
        }
    }
}
```

### Anti-pattern — register-only in v2 components

In v2 with `auto-unlisten` enabled, a bare `listen` without a lifetime modifier defaults to `until disable`. If you intentionally want register-only semantics, be explicit with `manual` and document why.

## Coroutine patterns

### Timed actions

Use `wait` with a duration instead of tracking timers manually.

```prsm
coroutine flashDamage() {
    spriteRenderer.color = Color.red
    wait 0.15s
    spriteRenderer.color = Color.white
}
```

### Polling with `wait until`

Replace manual `Update`-based checks with `wait until` for cleaner intent.

```prsm
coroutine waitForDoorOpen() {
    wait until door.isOpen
    playOpenAnimation()
}
```

### Multi-step sequences

Chain `wait` statements for scripted sequences. Each step reads top-to-bottom.

```prsm
coroutine introSequence() {
    fadeIn(title)
    wait 2.0s
    fadeOut(title)
    wait 0.5s
    fadeIn(subtitle)
    wait 3.0s
    fadeOut(subtitle)
    loadGameplay()
}
```

### Anti-pattern — infinite loop without wait

A coroutine that loops without yielding will freeze Unity. Always include a `wait` inside the loop body.

```prsm
// BAD — freezes Unity
coroutine badLoop() {
    while true {
        checkSomething()
    }
}

// GOOD — yields each frame
coroutine goodLoop() {
    while true {
        checkSomething()
        wait nextFrame
    }
}
```

## Null safety patterns

### Use `optional` and `?.` for components that may not exist

```prsm
component Interactable : MonoBehaviour {
    optional outline: OutlineEffect

    func highlight() {
        outline?.Enable()
    }

    func unhighlight() {
        outline?.Disable()
    }
}
```

### Use `?:` for fallback values

The elvis operator provides a concise default when a value may be null.

```prsm
func getDisplayName(): String {
    return player.customName ?: "Unknown Player"
}
```

### Avoid `!!` — prefer `require`

The not-null assertion `!!` throws at runtime if the value is null. In components, prefer `require` to catch missing references at initialization time rather than during gameplay.

```prsm
// Risky — fails at runtime if missing
optional rb: Rigidbody
func move() {
    rb!!.MovePosition(target)   // NullReferenceException if missing
}

// Safe — fails immediately at Awake with a clear message
require rb: Rigidbody
func move() {
    rb.MovePosition(target)
}
```

## Data modeling

### `data class` for value-like types

Use `data class` for structures that carry data. The compiler generates equality, hashing, and string representation.

```prsm
data class DamageInfo {
    amount: Int
    source: GameObject
    damageType: DamageType
}

data class SpawnConfig {
    prefab: GameObject
    position: Vector3
    rotation: Quaternion = Quaternion.identity
}
```

### `enum` for finite states

Plain enums work well for simple state machines.

```prsm
enum EnemyState {
    Idle,
    Chase,
    Attack,
    Flee
}
```

### Parameterized `enum` for states with data

When states carry associated data, use parameterized variants.

```prsm
enum AICommand {
    MoveTo(target: Vector3),
    Attack(enemy: GameObject),
    Wait(duration: Float),
    Patrol(waypoints: List<Vector3>)
}
```

## Intrinsic escape hatch

`intrinsic` lets you drop into raw C# when PrSM does not yet cover a Unity API. Use it sparingly.

### Prefer `intrinsic func` for reusability

Wrap the escape hatch in a named function so call sites stay clean.

```prsm
intrinsic func setLayerRecursive(obj: GameObject, layer: Int) {
    obj.layer = layer;
    foreach (Transform child in obj.transform) {
        SetLayerRecursive(child.gameObject, layer);
    }
}
```

### Keep inline `intrinsic {}` blocks small

If you must use an inline block, limit it to a few lines. Large intrinsic blocks defeat the purpose of writing PrSM.

```prsm
func captureScreenshot() {
    val path = "Screenshots/" + System.DateTime.Now.ToString("yyyyMMdd_HHmmss") + ".png"
    intrinsic {
        ScreenCapture.CaptureScreenshot(path);
    }
}
```

### When to use intrinsic

- Unity APIs that PrSM does not wrap (e.g., low-level rendering, native plugins)
- Performance-critical inner loops where you need exact C# control
- Third-party library interop that requires specific C# patterns

### When not to use intrinsic

- Standard Unity lifecycle — use `awake`, `start`, `update`, `onDestroy` blocks
- Event wiring — use `listen`
- Coroutines — use `coroutine` and `wait`
- Input — use `on input` (PrSM 2 부터)

If you find yourself writing large `intrinsic` blocks frequently, consider filing a feature request so the pattern can be supported natively.
