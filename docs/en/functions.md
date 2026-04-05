---
title: Functions
parent: Language Guide
grand_parent: English Docs
nav_order: 8
---

# Functions

Functions in PrSM are declared with `func` and live as members of a top-level declaration.

## Block-bodied functions

```prsm
func jump() {
    rb.AddForce(Vector3.up * jumpForce)
}

func takeDamage(amount: Int) {
    hp -= amount
    if hp <= 0 {
        die()
    }
}
```

## Expression-bodied functions

Functions whose body is a single expression use `=`:

```prsm
func isDead(): Bool = hp <= 0
func label(): String = "HP: $hp"
```

## Return types

Return types are explicit and follow `:`:

```prsm
func getCurrentSpeed(): Float {
    return rb.velocity.magnitude
}
```

Functions with `Unit` return type can omit the annotation.

## Visibility modifiers

- `public` — accessible from other C# code (default for most members)
- `private` — scoped to the component class
- `protected` — accessible from subclasses

```prsm
private func handleInput() {
    // ...
}

public func TakeDamage(amount: Int) {
    hp -= amount
}
```

## Override

Use `override` on any function that overrides a base class method:

```prsm
override func ToString(): String = "Player[$name]"
```

## Parameters

All parameters are positional with explicit types:

```prsm
func move(direction: Vector3, speed: Float) {
    transform.Translate(direction * speed * Time.deltaTime)
}
```
