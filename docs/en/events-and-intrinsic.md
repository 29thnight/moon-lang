---
title: Events & Intrinsic
parent: Language Guide
grand_parent: English Docs
nav_order: 11
---

# Events & Intrinsic

## `listen` — event wiring

The `listen` keyword wires a code block to a Unity `UnityEvent` or `UnityAction` field. It lowers to an `AddListener(...)` call that is appended to the generated `Awake()` body after all component lookups are resolved.

### Basic usage

```prsm
listen startButton.onClick {
    SceneManager.LoadScene("Game")
}

listen slider.onValueChanged {
    updateVolume(slider.value)
}
```

### With parameters

`UnityEvent<T>` callbacks automatically receive the event payload:

```prsm
listen healthBar.onValueChanged { val newValue ->
    if newValue <= 0.0 {
        triggerDeath()
    }
}
```

### Listen lifetimes (PrSM 2 부터)

v2 introduces explicit lifetime policies that automatically manage listener cleanup. These are only valid inside `component` declarations (error E083 outside).

#### `until disable` — auto-cleanup in OnDisable

```prsm
listen button.onClick until disable {
    fire()
}
```

Generated C#:

```csharp
private System.Action _prsm_h0;

void Start() {
    _prsm_h0 = () => { fire(); };
    button.onClick.AddListener(_prsm_h0);
}

private void __prsm_cleanup_disable() {
    button.onClick.RemoveListener(_prsm_h0);
    _prsm_h0 = null;
}

void OnDisable() {
    __prsm_cleanup_disable();
}
```

#### `until destroy` — auto-cleanup in OnDestroy

```prsm
listen spawner.onSpawn until destroy {
    count += 1
}
```

Same pattern as `until disable`, but cleanup runs in `OnDestroy`.

#### `manual` — explicit control with tokens

```prsm
val token = listen timer.onFinished manual {
    reset()
}

// Later:
unlisten token
```

`unlisten` resolves the token to the backing field and emits `RemoveListener` + null assignment. Works in any component method (lifecycle or user-defined functions).

#### Default behavior

| Context | Default |
|---------|---------|
| No modifier (`listen event { }`) | Register only — no auto-cleanup (same in v1 and v2) |
| `until disable` | Auto-cleanup in OnDisable |
| `until destroy` | Auto-cleanup in OnDestroy |
| `manual` | Explicit control via `unlisten` |

Lifetime modifiers must be explicitly written. There is no implicit default change between v1 and v2 — the compiler always defaults to register-only unless a modifier is specified.

#### Multiple listeners

Multiple listen statements generate separate handler fields (`_prsm_h0`, `_prsm_h1`, etc.) with independent cleanup.

---

## `intrinsic` — raw C# escape hatch

When PrSM's syntax does not cover a Unity API or pattern, `intrinsic` lets you embed raw C# without leaving the component structure. `intrinsic` code is passed through verbatim and is only validated by the C# compiler — PrSM's semantic checker does not inspect it.

### Statement block intrinsic

Inserts one or more raw C# statements inline in a method body:

```prsm
update {
    intrinsic {
        var ray = Camera.main.ScreenPointToRay(Input.mousePosition);
        if (Physics.Raycast(ray, out var hit, 100f)) {
            Debug.DrawLine(ray.origin, hit.point, Color.red);
        }
    }
}
```

### Intrinsic function

Declares a function whose entire body is raw C#:

```prsm
intrinsic func getMouseWorldPos(): Vector3 {
    """
    var ray = Camera.main.ScreenPointToRay(Input.mousePosition);
    Physics.Raycast(ray, out RaycastHit hit, 100f);
    return hit.point;
    """
}
```

### Intrinsic coroutine

Declares a coroutine whose body contains raw C# `yield` statements:

```prsm
intrinsic coroutine fadeOut(): IEnumerator {
    """
    float t = 1f;
    while (t > 0f) {
        t -= Time.deltaTime;
        canvasGroup.alpha = t;
        yield return null;
    }
    """
}
```

Use `intrinsic` sparingly. It is an escape valve for uncommon patterns, not a replacement for PrSM syntax.
