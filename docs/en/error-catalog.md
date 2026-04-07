---
title: Error Catalog
parent: Language Guide
grand_parent: English Docs
nav_order: 12
---

# Error Catalog

Every diagnostic the PrSM compiler emits carries a stable code. This page lists all codes, their severity, the message text, and how to fix the underlying issue.

---

## Errors

### E000 -- I/O error during compilation

**Severity:** Error
**Message:** `Cannot read source file: {path}`
**Explanation:** The compiler could not open or read a `.prsm` source file. This typically means the file was deleted, moved, or locked by another process after the file list was resolved.
**Fix:** Verify the file exists and is not locked. Check `.prsmproject` include/exclude patterns for stale entries.

---

### E012 -- Lifecycle block in wrong context

**Severity:** Error
**Message:** `Lifecycle block '{name}' is only valid inside a component declaration`
**Explanation:** Lifecycle blocks such as `update` or `awake` can only appear inside `component` bodies. They are not valid in `asset`, `class`, or other declarations.

```prsm
// triggers E012
asset GameConfig : ScriptableObject {
    update {
        tick()
    }
}
```

**Fix:** Move the lifecycle block into a `component`, or convert the declaration to a `component` if it needs frame callbacks.

---

### E013 -- Component-only field qualifier in wrong context

**Severity:** Error
**Message:** `'{qualifier}' fields are only valid inside a component declaration`
**Explanation:** The field qualifiers `require`, `optional`, `child`, and `parent` rely on `GetComponent` lookups generated in `Awake()`. They are only meaningful in a `component`.

```prsm
// triggers E013
class Utility {
    require rb: Rigidbody
}
```

**Fix:** Use a regular `val` or `var` field instead, or change the declaration to a `component`.

---

### E014 -- Duplicate lifecycle block

**Severity:** Error
**Message:** `Duplicate lifecycle block '{name}'; only one per component is allowed`
**Explanation:** Each lifecycle block may appear at most once per component. The compiler merges the block into a single generated Unity method and cannot handle duplicates.

```prsm
component Player : MonoBehaviour {
    update { movePlayer() }
    update { rotatePlayer() }  // E014
}
```

**Fix:** Combine the logic into a single lifecycle block, or extract one part into a helper function.

---

### E020 -- Type mismatch

**Severity:** Error
**Message:** `Type mismatch: expected '{expected}', found '{found}'`
**Explanation:** An expression produced a type that does not match what the surrounding context requires.

```prsm
component Demo : MonoBehaviour {
    serialize speed: Float = "fast"  // E020: expected Float, found String
}
```

**Fix:** Change the expression to produce the expected type, or update the type annotation.

---

### E022 -- Variable without type and without initializer

**Severity:** Error
**Message:** `Variable '{name}' must have a type annotation or an initializer`
**Explanation:** PrSM requires enough information to infer every variable's type. A bare declaration with neither a type nor an initial value is ambiguous.

```prsm
func demo() {
    val x  // E022: no type, no initializer
}
```

**Fix:** Add a type annotation (`val x: Int`) or an initializer (`val x = 0`), or both.

---

### E031 -- break/continue outside loop

**Severity:** Error
**Message:** `'{keyword}' can only be used inside a loop`
**Explanation:** `break` and `continue` must appear within a `for` or `while` body.

```prsm
func demo() {
    break  // E031
}
```

**Fix:** Move the statement inside a loop, or use `return` to exit the function instead.

---

### E032 -- wait outside coroutine

**Severity:** Error
**Message:** `'wait' can only be used inside a coroutine`
**Explanation:** `wait` lowers to `yield return` and is only valid inside a `coroutine` declaration.

```prsm
func fire() {
    wait 1.0s  // E032
}
```

**Fix:** Change `func` to `coroutine`, or remove the `wait` and use a different timing strategy.

---

### E040 -- Assignment to immutable val

**Severity:** Error
**Message:** `Cannot assign to immutable value '{name}'`
**Explanation:** A `val` binding is immutable after initialization. Attempting to reassign it is an error.

```prsm
func demo() {
    val hp = 100
    hp = 50  // E040
}
```

**Fix:** Change the declaration from `val` to `var` if the value needs to change.

---

### E041 -- Assignment to require field

**Severity:** Error
**Message:** `Cannot assign to 'require' field '{name}'`
**Explanation:** `require` fields are resolved once in `Awake()` and are treated as immutable for the lifetime of the component.

```prsm
component Demo : MonoBehaviour {
    require rb: Rigidbody

    func reset() {
        rb = null  // E041
    }
}
```

**Fix:** Use `optional` instead of `require` if the reference needs to change at runtime.

---

### E050 -- Empty enum

**Severity:** Error
**Message:** `Enum '{name}' must have at least one entry`
**Explanation:** An enum with zero entries is not valid. The compiler needs at least one variant to generate the backing C# enum.

```prsm
enum Status {}  // E050
```

**Fix:** Add at least one entry to the enum body.

---

### E051 -- Enum entry argument count mismatch

**Severity:** Error
**Message:** `Enum entry '{entry}' expects {expected} argument(s), but {found} given`
**Explanation:** When constructing an enum value that carries a payload, the number of arguments must match the entry definition.

```prsm
enum Result {
    Ok(Int),
    Err(String)
}

func demo() {
    val r = Result.Ok(1, 2)  // E051: Ok expects 1, got 2
}
```

**Fix:** Pass exactly the number of arguments declared in the enum entry.

---

### E052 -- Duplicate enum entry name

**Severity:** Error
**Message:** `Duplicate enum entry '{name}'`
**Explanation:** Each entry within a single enum must have a unique name.

```prsm
enum Direction {
    Up,
    Down,
    Up  // E052
}
```

**Fix:** Rename or remove the duplicate entry.

---

### E060 -- Coroutine in non-component declaration

**Severity:** Error
**Message:** `Coroutines are only valid inside a component declaration`
**Explanation:** Coroutines lower to `StartCoroutine` calls which require a `MonoBehaviour` context. They cannot appear in `asset` or `class` bodies.

```prsm
class Utility {
    coroutine delay() {  // E060
        wait 1.0s
    }
}
```

**Fix:** Move the coroutine into a `component`, or use a regular function with a callback pattern.

---

### E070 -- Input System sugar without feature flag

**Severity:** Error
**Message:** `Input System sugar requires the 'input-system' feature flag`
**Explanation:** The shorthand input binding syntax is gated behind a feature flag that must be enabled in `.prsmproject`.

**Fix:** Add `"input-system"` to the `language.features` array in your `.prsmproject` file.

---

### E081 -- Unknown enum variant in pattern

**Severity:** Error
**Message:** `Unknown variant '{variant}' for enum '{enum}'`
**Explanation:** A `when` branch references an enum variant that does not exist in the enum definition.

```prsm
enum State { Idle, Running }

func demo(s: State) {
    when s {
        State.Idle    => idle()
        State.Flying  => fly()  // E081: Flying not in State
    }
}
```

**Fix:** Check for typos and verify the variant name matches the enum definition.

---

### E082 -- Pattern binding arity mismatch

**Severity:** Error
**Message:** `Pattern for '{variant}' expects {expected} binding(s), found {found}`
**Explanation:** Destructuring a payload enum entry must bind the same number of values as the entry declares.

```prsm
enum Result { Ok(Int), Err(String) }

func demo(r: Result) {
    when r {
        Result.Ok(val a, val b) => log(a)  // E082: Ok has 1 field, 2 bound
        Result.Err(val msg)     => log(msg)
    }
}
```

**Fix:** Match the number of bindings to the enum entry's payload count.

---

### E083 -- Listen lifetime in wrong context

**Severity:** Error
**Message:** `Listen lifetime modifier is only valid inside a component`
**Explanation:** The `.once` and `.whileEnabled` listen lifetime modifiers depend on component lifecycle hooks to manage cleanup. They cannot be used in `asset` or `class` bodies.

**Fix:** Move the `listen` statement into a `component`, or wire the event manually.

---

### E100 -- Parser / syntax error

**Severity:** Error
**Message:** `Syntax error: {details}`
**Explanation:** The parser encountered a token it did not expect. This is the catch-all for malformed source text.

```prsm
component Demo : MonoBehaviour {
    func () { }  // E100: expected identifier after 'func'
}
```

**Fix:** Check the line indicated in the diagnostic for missing identifiers, unmatched braces, or misplaced keywords.

---

### E090 -- Interface member not implemented (since PrSM 3)

**Severity:** Error
**Message:** `Interface member '{name}' is not implemented`
**Explanation:** A component or class declares that it implements an interface but does not provide an implementation for a required member.

```prsm
interface IDamageable {
    func takeDamage(amount: Int)
    val isAlive: Bool
}

component Enemy : MonoBehaviour, IDamageable {
    var hp: Int = 100
    // E090: missing takeDamage and isAlive
}
```

**Fix:** Add the missing method or property to satisfy the interface contract.

---

### E091 -- Interface member has implementation body (since PrSM 3)

**Severity:** Error
**Message:** `Interface members shall not have implementation bodies`
**Explanation:** Interface members are signatures only. Providing a body inside an interface declaration is not allowed.

```prsm
interface IDamageable {
    func takeDamage(amount: Int) { }  // E091
}
```

**Fix:** Remove the body -- interface members are signatures only.

---

### E095 -- Generic type constraint violation (since PrSM 3)

**Severity:** Error
**Message:** `Type argument '{T}' does not satisfy constraint '{constraint}'`
**Explanation:** A generic type parameter was instantiated with a type that does not meet the declared constraint.

```prsm
class Registry<T> where T : Component {
    var items: List<T> = []
}

val r = Registry<String>()  // E095: String does not satisfy Component
```

**Fix:** Use a type that satisfies the constraint.

---

### E096 -- Generic params on unsupported declaration (since PrSM 3)

**Severity:** Error
**Message:** `Generic type parameters cannot be declared on component/asset/enum/data class`
**Explanation:** Generic type parameters are only supported on `class` and `func` declarations. Unity-serialized types (`component`, `asset`) and value-like types (`enum`, `data class`) cannot be generic.

```prsm
component Foo<T> : MonoBehaviour { }  // E096
```

**Fix:** Use generics only on `class` and `func`.

---

### E097 -- singleton on non-component (since PrSM 3)

**Severity:** Error
**Message:** `'singleton' can only be used before 'component'`
**Explanation:** The `singleton` modifier generates Unity-specific `Awake` and `DontDestroyOnLoad` code that is only meaningful on a `component`.

```prsm
singleton class Foo { }  // E097
```

**Fix:** Use `singleton` only with `component`.

---

### E098 -- Pool missing prefab (since PrSM 3)

**Severity:** Error
**Message:** `Pool type '{T}' has no matching serialize prefab field`
**Explanation:** A `pool` declaration requires a corresponding `serialize` field of the same type to serve as the prefab source.

```prsm
component Spawner : MonoBehaviour {
    pool bullets: Bullet(capacity = 20, max = 100)  // E098: no serialize Bullet field
}
```

**Fix:** Add a `serialize` field of the pool's type (e.g. `serialize bulletPrefab: Bullet`).

---

### E099 -- Pool outside component (since PrSM 3)

**Severity:** Error
**Message:** `'pool' is only valid inside a component declaration`
**Explanation:** Object pools depend on Unity lifecycle hooks and are only valid inside `component` bodies.

```prsm
class Utility {
    pool items: Bullet(capacity = 10, max = 50)  // E099
}
```

**Fix:** Move the `pool` declaration into a `component`.

---

### E101 -- Reserved built-in method name (since PrSM 3)

**Severity:** Error
**Message:** `'{name}' is a reserved built-in method name`
**Explanation:** Certain method names are reserved by the compiler for generated code and cannot be used as user-defined function names.

```prsm
component Demo : MonoBehaviour {
    func get() { }  // E101
}
```

**Fix:** Rename the function (e.g. `getData`, `findTarget`).

---

## Warnings

### W001 -- Unnecessary non-null assertion

**Severity:** Warning
**Message:** `Unnecessary '!!' on non-nullable type '{type}'`
**Explanation:** Applying `!!` to a value whose type is already non-nullable has no effect.

```prsm
val x: Int = 10
val y = x!!  // W001: Int is already non-nullable
```

**Fix:** Remove the `!!` operator.

---

### W003 -- Incomplete when pattern

**Severity:** Warning
**Message:** `'when' does not cover all variants of '{enum}'; missing: {variants}`
**Explanation:** A `when` expression over an enum does not list every variant and has no `else` branch. At runtime, unmatched values will fall through silently.

```prsm
enum Dir { Up, Down, Left, Right }

func demo(d: Dir) {
    when d {
        Dir.Up   => moveUp()
        Dir.Down => moveDown()
        // W003: missing Left, Right
    }
}
```

**Fix:** Add branches for the missing variants, or add an `else` branch.

---

### W005 -- Data class with no fields

**Severity:** Warning
**Message:** `Data class '{name}' has no fields`
**Explanation:** A `data class` with an empty parameter list is technically valid but almost certainly unintentional.

```prsm
data class Empty()  // W005
```

**Fix:** Add fields to the parameter list, or remove the data class if it is unused.

---

### W010 -- Too many public methods (since PrSM 3)

**Severity:** Warning
**Message:** `Component '{name}' has {n} public methods. Consider splitting responsibilities.`
**Explanation:** A component with 8 or more public methods may be taking on too many responsibilities. This warning encourages adherence to the SOLID Single Responsibility Principle.

**Fix:** Split the component into smaller, focused components with distinct responsibilities.

---

### W011 -- Too many dependencies (since PrSM 3)

**Severity:** Warning
**Message:** `Component '{name}' has {n} dependency fields. Consider reducing dependencies.`
**Explanation:** A component with 6 or more `require`/`optional`/`child`/`parent` fields may have too many dependencies. This warning encourages adherence to the SOLID Dependency Inversion Principle.

**Fix:** Reduce the number of dependency fields by extracting intermediary components or using events.

---

### W012 -- Method too long (since PrSM 3)

**Severity:** Warning
**Message:** `Method '{name}' has {n} statements. Consider extracting helper methods.`
**Explanation:** A method or lifecycle block with 50 or more statements is difficult to read and maintain. This warning encourages adherence to the SOLID Single Responsibility Principle.

**Fix:** Extract logic into smaller helper methods.

---

## Language 4 diagnostics (since PrSM 4)

PrSM 4 introduces 47 new diagnostic codes (E100–E146, W020–W032) covering exceptions, lambdas, modifiers, casts, properties, structs, async, Burst, and the v4 pattern sugars. The condition for each is summarized below; full prose entries follow the same format as the codes above.

### Errors

| Code | Feature | Condition |
|------|---------|-----------|
| E100 | try/catch | Unreachable catch clause (supertype caught above) |
| E101 | throw | Throw expression is not an Exception subtype |
| E102 | Lambda | Lambda parameter count mismatch with expected function type |
| E103 | Lambda | Cannot infer lambda parameter types without context |
| E104 | Lambda | `it` used in multi-parameter lambda |
| E105 | const | Non-literal initializer for `const` |
| E106 | static | `static` on a lifecycle block |
| E107 | Collection | Empty collection literal without type annotation |
| E108 | Collection | Mixed element types in collection literal |
| E109 | Cast | `as!` to provably unrelated type |
| E110 | Property | `set` on a `val` property |
| E111 | Property | `field` outside a property accessor |
| E112 | extend | Extension member conflicts with existing member on target type |
| E113 | extend | Extension block contains state (`var` field) |
| E114 | Modifier | `override` without matching `open`/`abstract` parent |
| E115 | sealed | Non-exhaustive `when` on sealed class |
| E116 | abstract | Instantiation of an `abstract` class |
| E117 | Tuple | Tuple destructuring count mismatch |
| E118 | Tuple | Named tuple field access with wrong label |
| E119 | use | Type does not implement `IDisposable` |
| E120 | struct | Struct inheriting from a class |
| E121 | event | Event invoked from outside the declaring type |
| E122 | event | Event with non-Unit return type |
| E123 | Operator | Unknown operator name in `operator` declaration |
| E124 | Operator | `operator equals` without matching `GetHashCode` override |
| E125 | Indexer | `operator get` without `operator set` on a mutable indexer |
| E126 | typealias | typealias creating a circular reference |
| E127 | typealias | typealias shadows a built-in type name |
| E128 | Raw string | Unterminated raw string literal |
| E129 | in | `in` used with a type that has no `Contains`/`ContainsKey` |
| E130 | OR pattern | OR pattern arms bind different variables |
| E131 | Range pattern | Range pattern with low > high |
| E132 | ?:= | Null coalescing assign on non-nullable variable |
| E133 | ?:= | Null coalescing assign on a `val` (immutable) |
| E134 | Interface | Default interface method uses feature unavailable in target C# version |
| E135 | async | `await` outside of `async func` |
| E136 | async | `async func` with non-awaitable return type |
| E137 | Burst | Managed type reference in `@burst` method |
| E138 | Burst | `try`/`catch` in `@burst` method |
| E139 | Burst | Virtual or interface call in `@burst` context |
| E140 | State machine | Transition to undeclared state |
| E141 | State machine | Duplicate state name |
| E142 | Command | `undo` references variable not captured during `execute` |
| E143 | bind | `bind to` target is not a writable property |
| E144 | bind | `bind` type mismatch between source and target |
| E145 | DX | Assignment used as condition expression |
| E146 | DX | Common API misuse detected |

### Warnings

| Code | Feature | Condition |
|------|---------|-----------|
| W020 | try/catch | Empty `catch` block |
| W021 | Cast | `as?` result never null-checked |
| W022 | struct | Large struct (over 16 bytes) |
| W023 | Range pattern | Overlapping range patterns in `when` |
| W024 | Interface | Default interface method references instance state |
| W025 | async | `async func` that never uses `await` |
| W026 | Optimizer | String allocation in hot path (cached by `opt.string`) |
| W027 | Optimizer | LINQ chain in `Update`/`FixedUpdate` (rewritten by `opt.linq`) |
| W028 | Burst | Boxing detected in `@burst` method |
| W029 | State machine | State with no outgoing transitions (dead state) |
| W030 | Command | `command` declared without a `canExecute` guard |
| W031 | bind | `bind` property never read |
| W032 | Debugger | Source map generation failed for a file (non-fatal) |
