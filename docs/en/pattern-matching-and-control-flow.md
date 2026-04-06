---
title: Pattern Matching & Control Flow
parent: Language Guide
grand_parent: English Docs
nav_order: 7
---

# Pattern Matching & Control Flow

## `if` / `else`

Conditions are written without parentheses:

```prsm
if hp <= 0 {
    die()
} else if hp < 20 {
    playLowHealthFX()
} else {
    run()
}
```

`if` is also an expression — it produces a value:

```prsm
val label = if hp <= 0 { "Dead" } else { "Alive" }
```

## `when`

`when` is PrSM's pattern matching construct. It replaces `switch` in the common case.

### Subject form

Matches branches against a value:

```prsm
when state {
    EnemyState.Idle   => idle()
    EnemyState.Chase  => chase()
    EnemyState.Attack => attack()
    else              => wait()
}
```

### Condition form

Matches the first true branch:

```prsm
when {
    hp <= 0        => die()
    hp < lowHpThreshold => playWarning()
    else           => run()
}
```

`when` is also an expression and can return values. The semantic layer checks for exhaustiveness where branch coverage can be determined (warning W003 when missing variants).

### Pattern bindings (PrSM 2 부터)

Enum payload bindings extract data from parameterized enum entries:

```prsm
enum EnemyState(val target: String) {
    Idle(""),
    Chase("player"),
    Stunned("player")
}

when state {
    EnemyState.Idle => idle()
    EnemyState.Chase(target) => moveTo(target)
    EnemyState.Stunned(duration) if duration > 0.0 => wait(duration)
}
```

Generated C# uses tuple-style access:

```csharp
case EnemyState.Chase _prsm_m8_5:
    var target = _prsm_m8_5.Item1;
    moveTo(target);
    break;
```

**Rules:**
- Binding count must match enum parameter count (error E082 if mismatched)
- Variant name must exist in the enum (error E081 if unknown)
- Empty bindings `EnemyState.Idle` match without extraction

### When guards (PrSM 2 부터)

Guards add a condition after a pattern:

```prsm
when state {
    EnemyState.Stunned(duration) if duration > 0.0 => wait(duration)
    EnemyState.Stunned(duration) => recover()
}
```

The guard expression is checked after the pattern matches. It generates an `&&` condition in the C# output.

### Destructuring in `val` (PrSM 2 부터)

Data class instances can be destructured into individual variables:

```prsm
data class PlayerStats(hp: Int, speed: Float)

val PlayerStats(hp, speed) = getStats()
```

Generated C#:

```csharp
var _prsm_d = getStats();
var hp = _prsm_d.hp;
var speed = _prsm_d.speed;
```

**Rules:**
- Binding count must match the data class field count (error E082)
- Binding names are used as local variable names

### Destructuring in `for` (PrSM 2 부터)

The same destructuring syntax works in `for` loops:

```prsm
for Spawn(pos, delay) in wave.spawns {
    spawnAt(pos, delay)
}
```

## `for`

Range-based iteration:

```prsm
for i in 0 until count {
    process(i)
}

for i in count downTo 0 {
    countdown(i)
}

for i in 0 until 10 step 2 {
    evens(i)
}
```

## `while`

```prsm
while alive {
    updateState()
}
```

## `break` and `continue`

Both are supported inside loops:

```prsm
for i in 0 until items.Count {
    if items[i] == null { continue }
    if i > maxItems { break }
    process(items[i])
}
```

## `is` type check

Branch on runtime type:

```prsm
if collider is BoxCollider {
    handleBox()
}

when shape {
    is Circle => drawCircle()
    is Rect   => drawRect()
    else      => drawDefault()
}
```
