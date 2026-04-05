---
title: Pattern Matching & Control Flow
parent: Language Guide
grand_parent: English Docs
nav_order: 7
---

# Pattern Matching & Control Flow

## `when`

PrSM pattern matching currently centers on `when`.

```prsm
when state {
    EnemyState.Idle => idle()
    EnemyState.Chase => chase()
    else => attack()
}
```

Condition-style `when` is also implemented:

```prsm
when {
    hp <= 0 => die()
    else => run()
}
```

The semantic layer performs exhaustiveness checks for supported cases.

## `if`, `for`, `while`

PrSM uses brace-based control flow without parentheses.

```prsm
if hp <= 0 {
    die()
} else {
    run()
}

for i in 0 until count {
    tick(i)
}

while alive {
    updateState()
}
```

`if` and `when` expressions are implemented, and `break` / `continue` are supported.
