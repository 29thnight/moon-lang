---
title: Input System
parent: Language Guide
grand_parent: English Docs
nav_order: 9
---

# Input System

(since PrSM 2)

PrSM v2 introduces syntactic sugar for Unity's **New Input System** package,
giving you a concise, declarative way to query action states. The generated code
uses Unity's project-wide **Actions workflow** (`InputSystem.actions`), so you
do not need a `PlayerInput` component for basic usage.

## Feature Gate

The input sugar is behind an explicit feature flag. Add the following to your
`.prsmproject` file:

```toml
features = ["input-system"]
```

You must also have the **Input System** package installed in your Unity project
(`com.unity.inputsystem`). If the feature flag is missing, using any `input.*`
expression will produce **error E070**.

```
error[E070]: input-system sugar requires `features = ["input-system"]` in .prsmproject
  --> src/player_controller.prsm:12:5
   |
12 |     val move = input.action("Move").vector2
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

## Basic Form

The basic form queries a named action from the project-wide action asset:

```prsm
input.action("ActionName").state
```

Replace `state` with one of the five supported state accessors described below.

### Example

```prsm
func update() {
    if input.action("Jump").pressed {
        jump()
    }

    val move = input.action("Move").vector2
    transform.translate(move.x, 0, move.y)
}
```

Generated C#:

```csharp
if (InputSystem.actions.FindAction("Jump").WasPressedThisFrame())
{
    Jump();
}

var move = InputSystem.actions.FindAction("Move").ReadValue<UnityEngine.Vector2>();
transform.Translate(move.x, 0, move.y);
```

## Map Form (Explicit Action Map)

When you need to target a specific action map, use the map form. This is useful
when multiple maps contain actions with the same name:

```prsm
input.map("MapName").action("ActionName").state
```

The map form generates a `"Map/Action"` lookup key passed to `FindAction`:

```prsm
input.map("Gameplay").action("Fire").pressed
```

Generated C#:

```csharp
InputSystem.actions.FindAction("Gameplay/Fire").WasPressedThisFrame()
```

### Example

```prsm
func update() {
    if input.map("Gameplay").action("Fire").pressed {
        fireWeapon()
    }

    val look = input.map("Gameplay").action("Look").vector2
    rotateCameraBy(look)
}
```

## State Accessors

PrSM provides five state accessors. Each one maps to a specific C# call on the
resolved `InputAction`.

### `.pressed`

Returns `true` on the frame the action was first pressed.

| PrSM | Generated C# |
|---|---|
| `input.action("Jump").pressed` | `InputSystem.actions.FindAction("Jump").WasPressedThisFrame()` |

### `.released`

Returns `true` on the frame the action was released.

| PrSM | Generated C# |
|---|---|
| `input.action("Jump").released` | `InputSystem.actions.FindAction("Jump").WasReleasedThisFrame()` |

### `.held`

Returns `true` every frame the action is continuously held down.

| PrSM | Generated C# |
|---|---|
| `input.action("Crouch").held` | `InputSystem.actions.FindAction("Crouch").IsPressed()` |

### `.vector2`

Reads the current value as a `Vector2`. Typical for movement or look sticks.

| PrSM | Generated C# |
|---|---|
| `input.action("Move").vector2` | `InputSystem.actions.FindAction("Move").ReadValue<UnityEngine.Vector2>()` |

### `.scalar`

Reads the current value as a `float`. Useful for triggers or 1D axes.

| PrSM | Generated C# |
|---|---|
| `input.action("Throttle").scalar` | `InputSystem.actions.FindAction("Throttle").ReadValue<float>()` |

## Generated Infrastructure

When the compiler detects input sugar in a class, it automatically adds:

```csharp
using UnityEngine.InputSystem;
```

No `PlayerInput` component or backing field is injected. The sugar uses
`InputSystem.actions` which accesses the project-wide action asset configured in
**Edit > Project Settings > Input System Package**.

## Asset Wiring (Advanced)

If you need to drive the sugar from a specific `.inputactions` asset through a
`PlayerInput` component, annotate a serialized `InputActionAsset` field:

```prsm
@inputActions(defaultMap: "Gameplay")
serialize controls: InputActionAsset
```

The annotation is compiler-only. It does not lower to a C# attribute. When
present, the compiler injects `PlayerInput` infrastructure:

```csharp
[UnityEngine.RequireComponent(typeof(UnityEngine.InputSystem.PlayerInput))]
private UnityEngine.InputSystem.PlayerInput _prsmInput;

void Awake()
{
    _prsmInput = GetComponent<UnityEngine.InputSystem.PlayerInput>();
    _prsmInput.actions = controls;
    _prsmInput.defaultActionMap = "Gameplay";
}
```

Note that even with `@inputActions`, the `input.action()` sugar still generates
`InputSystem.actions.FindAction(...)` calls. The annotation is for wiring the
asset to the `PlayerInput` component for callback-based workflows.

## Error E070

If you use input sugar without enabling the feature gate, the compiler emits
**E070** with a clear remediation message:

```
error[E070]: input-system sugar requires `features = ["input-system"]` in .prsmproject
```

To fix it, add `features = ["input-system"]` to your project file and ensure
the Unity Input System package is installed.

## Comparison with Legacy Input Sugar

PrSM v1 shipped sugar for Unity's legacy `UnityEngine.Input` API:

| Legacy sugar (PrSM 1) | New Input System sugar (since PrSM 2) |
|---|---|
| `input.getKey(KeyCode.Space)` | `input.action("Jump").pressed` |
| `input.axis("Horizontal")` | `input.action("Move").vector2.x` |
| `input.getMouseButton(0)` | `input.action("Fire").pressed` |

The legacy forms still compile and generate calls to `Input.GetKey`,
`Input.GetAxis`, etc. However, if you enable the `input-system` feature, the
compiler will emit a **warning** when legacy sugar is used alongside new input
sugar in the same file, encouraging migration to the action-based API.

Both styles may coexist in the same project during a migration period, but they
cannot be mixed within a single class.
