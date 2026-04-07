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

## Lambda expressions (since PrSM 4)

Lambdas are anonymous functions enclosed in `{ }`. A single-parameter lambda may use the implicit `it` identifier. When the last argument of a call is a lambda, it may be placed outside the parentheses (trailing lambda).

```prsm
val callback: (Int) => Unit = { x => log(x) }
val add: (Int, Int) => Int = { a, b => a + b }

list.filter { it > 10 }
list.where({ x => x > 10 }).select({ x => x * 2 })
```

`(A, B) => R` lowers to `Func<A, B, R>`. `() => Unit` lowers to `Action`. Closure capture follows the same reference semantics as C# lambdas.

## `static` and `const` members (since PrSM 4)

`static` declares a member accessible without an instance. `const` declares a compile-time constant whose initializer must be a literal.

```prsm
class MathHelper {
    static val PI: Float = 3.14159
    static func lerp(a: Float, b: Float, t: Float): Float = a + (b - a) * t
}

const MAX_HEALTH: Int = 100
const VERSION: String = "1.0.0"
```

`static` is not permitted on lifecycle blocks (E106). `const` initializers must be literals (E105).

## `abstract` / `open` / `override` (since PrSM 4)

By default, classes and methods are final. The modifiers control inheritance:

| Modifier | Effect |
|---|---|
| `open` | Permits inheritance / override |
| `abstract` | Requires a subclass implementation; cannot be instantiated |
| `sealed` | Restricts subclasses to the same file (enables exhaustive `when`) |
| `override` | Marks a method that replaces a parent `open`/`abstract` method |

```prsm
abstract class Weapon {
    abstract func attack()
    open func reload() { }
}

class Sword : Weapon {
    override func attack() { swing() }
}
```

`override` without a matching parent method produces E114. Instantiating an `abstract` class produces E116.

## `async` / `await` (since PrSM 4)

`async func` declares an asynchronous function. `await` suspends until the awaited task completes. The compiler prefers UniTask in Unity contexts and falls back to `Task`.

```prsm
async func loadData(url: String): String {
    val response = await Http.get(url)
    return response.body
}
```

```csharp
public async UniTask<string> loadData(string url)
{
    var response = await Http.Get(url);
    return response.body;
}
```

`Unit`-returning `async` functions lower to `UniTask`. `await` outside an `async func` produces E135.

## Operator overloading (since PrSM 4)

Operator functions map symbolic operators to named methods. PrSM follows Kotlin conventions: `plus`, `minus`, `times`, `div`, `mod`, `compareTo`, `equals`, `unaryMinus`, `not`.

```prsm
data class Vec2i(x: Int, y: Int) {
    operator plus(other: Vec2i): Vec2i = Vec2i(x + other.x, y + other.y)
    operator minus(other: Vec2i): Vec2i = Vec2i(x - other.x, y - other.y)
}

val c = Vec2i(1, 2) + Vec2i(3, 4)  // Vec2i(4, 6)
```

`operator get` and `operator set` define indexer access for `[]` syntax.
