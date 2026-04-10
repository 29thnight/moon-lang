---
title: Input System
parent: Language Guide
grand_parent: English Docs
nav_order: 9
---

# Input System

(since PrSM 2)

PrSM v2 introduces syntactic sugar for Unity's **New Input System** package,
giving you a concise, declarative way to query action states without writing
boilerplate `PlayerInput` or `InputAction` code by hand.

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

## Asset Wiring

To drive the sugar from an external `.inputactions` asset, annotate a serialized
`InputActionAsset` field:

```prsm
@inputActions(defaultMap: "Gameplay")
serialize controls: InputActionAsset
```

The annotation is compiler-only. It does not lower to a C# attribute. Instead,
the compiler treats the field as the source for `PlayerInput.actions` and, when
`defaultMap` is present, also assigns `PlayerInput.defaultActionMap`.

Generated wiring:

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

This keeps the asset reference Inspector-friendly while ensuring the generated
component always has a `PlayerInput` dependency.

## Basic Form

The basic form queries a named action from the default action map:

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

## Player Form (Multiplayer)

For multiplayer scenarios where each player has their own action map, use the
player form. This specifies both the map name and the action name:

```prsm
input.player("MapName").action("ActionName").state
```

The player form generates a lookup key in the format `"Map/Action"`, so the
compiler translates `input.player("Gameplay").action("Fire").pressed` into a
lookup against `_prsmInput.actions["Gameplay/Fire"]`.

### Example

```prsm
func update() {
    if input.player("Gameplay").action("Fire").pressed {
        fireWeapon()
    }

    val look = input.player("Gameplay").action("Look").vector2
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
| `input.action("Jump").pressed` | `_prsmInput.actions["Jump"].WasPressedThisFrame()` |

### `.released`

Returns `true` on the frame the action was released.

| PrSM | Generated C# |
|---|---|
| `input.action("Jump").released` | `_prsmInput.actions["Jump"].WasReleasedThisFrame()` |

### `.held`

Returns `true` every frame the action is continuously held down.

| PrSM | Generated C# |
|---|---|
| `input.action("Crouch").held` | `_prsmInput.actions["Crouch"].IsPressed()` |

### `.vector2`

Reads the current value as a `Vector2`. Typical for movement or look sticks.

| PrSM | Generated C# |
|---|---|
| `input.action("Move").vector2` | `_prsmInput.actions["Move"].ReadValue<UnityEngine.Vector2>()` |

### `.scalar`

Reads the current value as a `float`. Useful for triggers or 1D axes.

| PrSM | Generated C# |
|---|---|
| `input.action("Throttle").scalar` | `_prsmInput.actions["Throttle"].ReadValue<float>()` |

## Generated Infrastructure

When the compiler detects input sugar in a class, it automatically generates
the backing infrastructure so you never need to declare it yourself:

1. A private field on the class:

   ```csharp
   private UnityEngine.InputSystem.PlayerInput _prsmInput;
   ```

2. A class-level dependency marker:

   ```csharp
   [UnityEngine.RequireComponent(typeof(UnityEngine.InputSystem.PlayerInput))]
   ```

3. Initialization inside `Awake` (or a merged `Awake` if one already exists):

   ```csharp
   void Awake()
   {
       _prsmInput = GetComponent<UnityEngine.InputSystem.PlayerInput>();
   }
   ```

If the class already has an `awake()` function in PrSM, the compiler merges the
`GetComponent` call into the generated `Awake` body before any user statements.

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
