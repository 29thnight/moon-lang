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

### Pattern bindings (since PrSM 2)

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

### When guards (since PrSM 2)

Guards add a condition after a pattern:

```prsm
when state {
    EnemyState.Stunned(duration) if duration > 0.0 => wait(duration)
    EnemyState.Stunned(duration) => recover()
}
```

The guard expression is checked after the pattern matches. It generates an `&&` condition in the C# output.

### Destructuring in `val` (since PrSM 2)

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

### Destructuring in `for` (since PrSM 2)

The same destructuring syntax works in `for` loops:

```prsm
for Spawn(pos, delay) in wave.spawns {
    spawnAt(pos, delay)
}
```

### OR patterns (since PrSM 4)

Multiple patterns separated by commas in a `when` arm match if any individual pattern matches. All arms in an OR group must bind the same variables (or none).

```prsm
when direction {
    Direction.Up, Direction.Down    => handleVertical()
    Direction.Left, Direction.Right => handleHorizontal()
}
```

Generated C#:

```csharp
switch (direction) {
    case Direction.Up:
    case Direction.Down:
        handleVertical();
        break;
    case Direction.Left:
    case Direction.Right:
        handleHorizontal();
        break;
}
```

OR pattern arms that bind different variables produce E130.

### Range patterns (since PrSM 4)

`in low..high` inside a `when` arm matches values in the inclusive range `[low, high]`. Only integral and floating-point types are supported.

```prsm
when score {
    in 90..100 => "A"
    in 80..89  => "B"
    in 70..79  => "C"
    else       => "F"
}
```

A range with `low > high` produces E131. Overlapping range patterns emit W023.

### Smart casts in `when` (since PrSM 4)

After an `is` arm matches, the subject is narrowed to the checked type within the arm body:

```prsm
when target {
    is Enemy => target.takeDamage(10)
    is Ally  => target.heal(5)
}
```

## `try` / `catch` / `finally` (since PrSM 4)

Exceptions are first-class. The `new` keyword is omitted on `throw`. `try` may also be used as an expression when it has exactly one `catch` clause.

```prsm
try {
    val data = File.readAllText(path)
} catch (e: FileNotFoundException) {
    warn(e.message)
} catch (e: Exception) {
    error(e.message)
} finally {
    cleanup()
}

throw ArgumentException("Invalid value")

val result = try { parseInt(str) } catch (e: Exception) { -1 }
```

Generated C#:

```csharp
try
{
    var data = File.ReadAllText(path);
}
catch (FileNotFoundException e) { Debug.LogWarning(e.Message); }
catch (Exception e) { Debug.LogError(e.Message); }
finally { Cleanup(); }

throw new ArgumentException("Invalid value");
```

A `catch` clause whose type is already covered by a higher clause produces E100. `throw` of a non-Exception expression produces E101. Empty `catch` blocks emit W020.

## `use` (IDisposable) (since PrSM 4)

`use` ensures automatic disposal of `IDisposable` resources. The block form disposes at block exit; the declaration form disposes at the enclosing scope exit.

```prsm
use stream = FileStream(path, FileMode.Open) {
    val data = stream.readToEnd()
}

use val conn = DbConnection(connString)
// conn auto-disposed at scope end
```

Lowers to a C# `using` statement (block form) or `using` declaration (`use val`). Using `use` on a type that does not implement IDisposable produces E119.

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
