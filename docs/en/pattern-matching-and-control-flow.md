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

`when` is also an expression and can return values. The semantic layer checks for exhaustiveness where branch coverage can be determined.

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
