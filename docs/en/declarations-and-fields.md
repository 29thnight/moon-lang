---
title: Declarations & Fields
parent: Language Guide
grand_parent: English Docs
nav_order: 5
---

# Declarations & Fields

## Top-level declarations

Each `.prsm` file contains exactly one top-level declaration.

| Keyword | C# equivalent | Purpose |
|---|---|---|
| `component` | `MonoBehaviour` subclass | Gameplay logic attached to a GameObject |
| `asset` | `ScriptableObject` subclass | Data containers, config, shared state |
| `class` | Regular C# `class` | Utilities, services, plain data |
| `data class` | Serializable value class | Lightweight data with generated equality |
| `enum` | `enum` | Named constant sets |
| `attribute` | `Attribute` subclass | Custom C# annotations |

## `component`

```prsm
using UnityEngine

component PlayerController : MonoBehaviour {
    @header("Movement")
    serialize speed: Float = 5.0

    require rb: Rigidbody

    update {
        move()
    }

    func move() {
        rb.MovePosition(rb.position + transform.forward * speed * Time.fixedDeltaTime)
    }
}
```

## `asset`

```prsm
using UnityEngine

asset WeaponConfig : ScriptableObject {
    serialize damage: Int = 10
    serialize fireRate: Float = 0.2
    serialize projectilePrefab: GameObject = null
}
```

 Assets created via `ScriptableObject.CreateInstance<T>()` in the Unity Editor store their values persistently in `.asset` files.

## `class`

```prsm
class DamageCalculator {
    func calculate(base: Int, multiplier: Float): Float {
        return base * multiplier
    }
}
```

`class` maps to a regular C# class with no Unity dependency.

## Serialized fields

Fields marked `serialize` are exposed in the Unity Inspector. Several decorator annotations control how they appear:

```prsm
@header("Stats")
serialize maxHp: Int = 100

@tooltip("Units per second")
serialize speed: Float = 5.0

@range(0.0, 1.0)
serialize damageMultiplier: Float = 0.5

@space
serialize weaponSlot: GameObject = null
```

Supported decorators: `@header(label)`, `@tooltip(text)`, `@range(min, max)`, `@space`, `@hideInInspector`.

## `val` and `var`

- `val` — immutable; cannot be reassigned after initialization
- `var` — mutable field or local

```prsm
val gravity: Float = 9.81      // constant
var hp: Int = 100               // mutable
```

## Visibility modifiers

`public`, `private`, and `protected` map directly to C#. In most contexts members default to `public`.

```prsm
private var invincible: Bool = false
protected var baseSpeed: Float = 5.0
```

## Component lookup fields

Four qualifiers are only valid inside `component` declarations. They generate lookup code in the synthesized `Awake()` **before** the user `awake` body runs:

| Qualifier | Generated C# | Null contract |
|---|---|---|
| `require name: Type` | `GetComponent<Type>()` | Logs an error and asserts non-null if missing |
| `optional name: Type?` | `GetComponent<Type>()` | May be null, stored as nullable |
| `child name: Type` | `GetComponentInChildren<Type>()` | Asserts non-null |
| `parent name: Type` | `GetComponentInParent<Type>()` | Asserts non-null |

```prsm
require animator: Animator
optional shield: Shield?
child muzzle: Transform
parent vehicle: Vehicle
```

These qualifiers are only valid in components (error E013 in class/asset).

## `data class`

A data class generates a plain C# class with constructor, `Equals`, `GetHashCode`, and `ToString`:

```prsm
data class DamageInfo(amount: Int, crit: Bool)
```

Generated C#:

```csharp
[System.Serializable]
public class DamageInfo {
    public int amount;
    public bool crit;

    public DamageInfo(int amount, bool crit) { ... }
    public override bool Equals(object obj) { ... }
    public override int GetHashCode() { ... }
    public override string ToString() {
        return $"DamageInfo(amount={amount}, crit={crit})";
    }
}
```

Data classes support v2 destructuring: `val DamageInfo(amount, crit) = info`.

## `enum` (parameterized)

Simple enums map directly to C# enums:

```prsm
enum Direction { Up, Down, Left, Right }
```

Parameterized enums generate an enum + extension methods for payload access:

```prsm
enum Weapon(val damage: Int, val range: Float) {
    Sword(10, 1.5),
    Bow(7, 8.0)
}
```

Generated C# creates `Weapon.Damage()` and `Weapon.Range()` extension methods that use a switch to return the correct value per entry.

**Rules:**
- Every entry must provide the same number of arguments as the enum parameters (error E051)
- At least one entry is required (error E050)
- No duplicate entry names (error E052)

## `attribute`

Custom attributes for serialized fields:

```prsm
attribute Cooldown(val duration: Float, val label: String)
```

Used as decorators on fields: `@cooldown(2.0, "Fire Rate")`.

## `interface` (since PrSM 3)

Interfaces define method signatures and properties without implementation:

```prsm
interface IDamageable {
    func takeDamage(amount: Int)
    val isAlive: Bool
}

interface IHealable : IDamageable {
    func heal(amount: Int)
}
```

Generated C#:

```csharp
public interface IDamageable
{
    void takeDamage(int amount);
    bool isAlive { get; }
}
```

Components and classes implement interfaces after the base class:

```prsm
component Enemy : MonoBehaviour, IDamageable {
    var hp: Int = 100
    val isAlive: Bool = hp > 0
    func takeDamage(amount: Int) { hp -= amount }
}
```

Interfaces can be used with `require`: `require target: IDamageable`.

Unimplemented members produce error E090. Implementation bodies in interface produce E091.

## `singleton` component (since PrSM 3)

The `singleton` modifier on a component auto-generates the singleton pattern:

```prsm
singleton component AudioManager : MonoBehaviour {
    serialize volume: Float = 1.0
    func playSound(clip: AudioClip) { /* ... */ }
}
```

This generates: static `Instance` property with lazy initialization, `Awake` duplicate check with `DontDestroyOnLoad`. Access via `AudioManager.instance.playSound(clip)`.

`singleton` is only valid on `component` declarations (E097).

## `struct` declaration (since PrSM 4)

`struct` declares a value type with copy semantics and stack allocation. Similar to `data class` but stored as a C# `struct`.

```prsm
struct DamageInfo(amount: Int, type: DamageType)

struct Color32(r: Byte, g: Byte, b: Byte, a: Byte) {
    static val white = Color32(255, 255, 255, 255)
    static val black = Color32(0, 0, 0, 255)
}
```

Generated C#:

```csharp
public struct DamageInfo {
    public int amount;
    public DamageType type;
    public DamageInfo(int amount, DamageType type) { this.amount = amount; this.type = type; }
}
```

Structs may not inherit from a class (E120). Structs larger than 16 bytes emit W022 to flag potential copy cost.

## Property `get` / `set` (since PrSM 4)

Properties may have custom accessors. `val` with only `get` defines a computed read-only property. Inside `set`, the implicit `field` keyword refers to the auto-generated backing field (Kotlin-style).

```prsm
component Player : MonoBehaviour {
    var hp: Int
        get = _hp
        set(value) {
            _hp = Mathf.clamp(value, 0, maxHp)
            onHpChanged?.invoke(_hp)
        }

    val isAlive: Bool
        get = hp > 0
}
```

Generated C#:

```csharp
private int __hp;
public int hp {
    get => __hp;
    set {
        __hp = Mathf.Clamp(value, 0, maxHp);
        onHpChanged?.Invoke(__hp);
    }
}
public bool isAlive => hp > 0;
```

E110 fires if `set` is declared on a `val`. E111 fires if `field` appears outside a property accessor.

## `event` members (since PrSM 4)

`event` declares a multicast delegate that supports `+=` / `-=` subscription and `.invoke()` dispatch:

```prsm
component Damageable : MonoBehaviour {
    event onHealthChanged: (Int) => Unit

    func takeDamage(amount: Int) {
        hp -= amount
        onHealthChanged.invoke(hp)
    }
}

// elsewhere:
target.onHealthChanged += { hp => log("HP: $hp") }
```

Generated C#:

```csharp
public event Action<int> onHealthChanged;
// ...
onHealthChanged?.Invoke(hp);
```

Events may only be invoked from inside the declaring type (E121) and must have a `Unit` return type (E122). Function-typed callbacks (`var onDamaged: ((Int) => Unit)? = null`) provide single-cast equivalents without the `event` keyword.

## Extension methods — `extend` (since PrSM 4)

`extend Type { }` adds methods and computed properties to an existing type. Inside the block, `this` refers to the receiver instance:

```prsm
extend Transform {
    func resetLocal() {
        this.localPosition = Vector3.zero
        this.localRotation = Quaternion.identity
        this.localScale = Vector3.one
    }
}

extend Vector3 {
    val flat: Vector3
        get = vec3(this.x, 0, this.z)
}

transform.resetLocal()
val groundPos = transform.position.flat
```

Lowers to a static class with C# extension methods. Extension blocks shall not contain mutable state (E113).

## `bind` reactive properties (since PrSM 4)

`bind` declares a reactive property that automatically notifies subscribers when its value changes. The compiler emits `INotifyPropertyChanged` plumbing:

```prsm
component PlayerHUD : MonoBehaviour {
    bind hp: Int = 100
    bind playerName: String = "Hero"

    serialize hpLabel: TextMeshProUGUI

    awake {
        bind hp to hpLabel.text
    }
}
```

Generated C#:

```csharp
private int _hp = 100;
public int hp {
    get => _hp;
    set {
        if (_hp != value) {
            _hp = value;
            OnPropertyChanged(nameof(hp));
        }
    }
}

public event PropertyChangedEventHandler PropertyChanged;
```

E143 fires when a `bind to` target is not writable. E144 fires on a type mismatch between the source and target. W031 warns about bind properties that are never read.

## `command` declarations (since PrSM 4)

`command` declares an executable action with optional undo support and a guard condition. The compiler generates a nested `ICommand` implementation:

```prsm
component UnitController : MonoBehaviour {
    command moveUnit(target: Vector3) {
        val prevPos = transform.position
        transform.position = target
    } undo {
        transform.position = prevPos
    } canExecute = isAlive && !isStunned
}
```

The compiler emits a `MoveUnitCommand : ICommand` class with `Execute()`, `Undo()`, and `CanExecute()` plus a helper method on the owner. Commands without a `canExecute` guard emit W030.

## `state machine` declarations (since PrSM 4)

`state machine` declares a finite state machine with named states, transitions, and optional `enter`/`exit` hooks:

```prsm
component EnemyAI : MonoBehaviour {
    state machine aiState {
        state Idle {
            enter { playAnimation("idle") }
            on playerDetected => Chase
        }
        state Chase {
            enter { playAnimation("run") }
            on playerLost => Idle
            on inAttackRange => Attack
        }
        state Attack {
            enter { playAnimation("attack") }
            exit  { resetCooldown() }
            on attackDone => Chase
        }
    }
}
```

The compiler generates a private state enum, a current-state field, a public `TransitionAiState(string event)` dispatcher, and `_enterAiState`/`_exitAiState` helpers. Transitions to undeclared states fire E140; duplicate state names fire E141; states with no outgoing transitions emit W029.

## `pool` modifier (since PrSM 3)

The `pool` modifier creates an object pool backed by `UnityEngine.Pool.ObjectPool<T>`:

```prsm
component BulletSpawner : MonoBehaviour {
    serialize bulletPrefab: Bullet
    pool bullets: Bullet(capacity = 20, max = 100)

    func fire(direction: Vector3) {
        val bullet = bullets.get()
        bullet.launch(direction)
    }
}
```

The pool auto-matches a `serialize` field by type for the prefab. Missing prefab produces E098. `pool` is only valid inside components (E099).

## Initialization order

For components, the initialization sequence is:

1. Unity calls `Awake()`
2. Compiler-generated: `require`/`optional`/`child`/`parent` lookups execute
3. Compiler-generated: serialized field defaults applied
4. User `awake { }` body runs
5. Unity calls `Start()` → user `start { }` body runs
