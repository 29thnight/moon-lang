---
title: PrSM Language Standard
parent: Specification
nav_order: 1
---

# PrSM Language Standard

::: warning
Language 2 --- Working Draft
:::

---

## 1 Scope [scope]

### 1.1 Purpose

This document specifies the PrSM programming language. PrSM is a Unity-first
scripting language that compiles (transpiles) to C# source files consumed by
Unity's compilation pipeline. A conforming implementation shall accept every
valid PrSM program as defined herein and shall produce the C# output specified
in S12 for that program.

### 1.2 Language version

This document defines **Language 2** (current). Features that were not present in Language 1 are marked with (PrSM 2 부터) at the point of introduction.

### 1.3 Target platform

A conforming implementation shall target **Unity 2022.3 LTS** or later,
running under either the **Mono** or **IL2CPP** scripting backend.

### 1.4 Conformance

A conforming implementation:

1. Shall accept all programs that satisfy the syntactic and semantic rules of
   this document.
2. Shall reject all programs for which a diagnostic is required (error codes
   **E0xx**).
3. Shall produce C# output that, when compiled by Unity's C# compiler, yields
   runtime behavior identical to the semantics described in each section.
4. May issue additional warnings beyond those required by this document.

---

## 2 Normative references [norm.ref]

The following documents are referred to in the text in such a way that some or
all of their content constitutes requirements of this document.

| Reference | Description |
|---|---|
| **Unity Engine API** | Unity Technologies, Unity Scripting Reference, version 2022.3 LTS or later. |
| **C# Language Specification** | ECMA-334, 6th edition or later. The generated C# output shall be valid under this specification. |
| **`.prsmproject` configuration format** | The project-level configuration file that controls language version and feature gates. See S5.8. |

---

## 3 Lexical structure [lex]

### 3.1 Source files [lex.source]

A PrSM source file shall be encoded in **UTF-8**. Each file contains exactly
one top-level declaration (S5). Statements are terminated by newlines; there
are no semicolons.

### 3.2 Comments [lex.comment]

Two forms of comment are defined:

```
// single-line comment (extends to end of line)

/* multi-line comment
   may span multiple lines */
```

Comments are stripped during lexing and have no semantic effect.

### 3.3 Keywords [lex.keyword]

The following tokens are reserved keywords. A keyword shall not be used as an
identifier.

**Declaration keywords:**
`component` `asset` `class` `data` `enum` `attribute`

**Field keywords:**
`serialize` `require` `optional` `child` `parent` `val` `var`

**Function keywords:**
`func` `coroutine` `intrinsic` `override`

**Control flow keywords:**
`if` `else` `when` `for` `while` `return` `break` `continue`

**Event keywords:**
`listen` `unlisten`

**Coroutine keywords:**
`start` `stop` `stopAll` `wait` `nextFrame` `fixedFrame`

**Operator keywords:**
`is` `in` `until` `downTo` `step`

**Lifecycle keywords:**
`awake` `start` `update` `fixedUpdate` `lateUpdate`
`onEnable` `onDisable` `onDestroy`
`onTriggerEnter` `onTriggerExit` `onTriggerStay`
`onCollisionEnter` `onCollisionExit` `onCollisionStay`

**Modifier keywords:**
`public` `private` `protected` `manual` `disable` `destroy`

**Import keywords:**
`using`

**Literal keywords:**
`true` `false` `null` `this`

### 3.4 Identifiers [lex.ident]

```ebnf
IDENT = LETTER { LETTER | DIGIT | "_" } ;
LETTER = "a".."z" | "A".."Z" | "_" ;
DIGIT  = "0".."9" ;
```

Identifiers beginning with an uppercase letter are, by convention, type names
(PascalCase). Member and field names use camelCase. These conventions are not
enforced by the grammar but are assumed by the standard library and lowering
rules.

If a PrSM identifier collides with a C# keyword, the generated code shall
prefix it with `@` (e.g., parameter `base` becomes `@base` in C#).

### 3.5 Literals [lex.literal]

#### 3.5.1 Integer literals

```ebnf
INT_LIT = DIGIT { DIGIT } ;
```

An integer literal denotes a value of type `Int`. Example: `42`, `0`, `1000`.

#### 3.5.2 Floating-point literals

```ebnf
FLOAT_LIT = DIGIT { DIGIT } "." DIGIT { DIGIT } [ "f" ] ;
```

A floating-point literal denotes a value of type `Float`. The trailing `f`
suffix is optional. Example: `3.14`, `1.0f`.

#### 3.5.3 Duration literals

A numeric literal immediately followed by a time suffix denotes a duration:

| Syntax | Meaning | C# lowering |
|---|---|---|
| `1.5s` | 1.5 seconds | `new WaitForSeconds(1.5f)` |
| `500ms` | 500 milliseconds | `new WaitForSeconds(0.5f)` |

Duration literals are valid only in `wait` statements (S10).

#### 3.5.4 String literals

```ebnf
STRING_LIT = '"' { CHAR | ESCAPE | "$" IDENT | "${" Expr "}" } '"' ;
```

String literals support interpolation (S3.6) and escape sequences (S3.7).

#### 3.5.5 Boolean literals

`true` and `false` denote values of type `Bool`.

#### 3.5.6 Null literal

`null` denotes the null reference. It is assignable to any nullable type
`T?`.

### 3.6 String interpolation [lex.interp]

Two forms of interpolation are defined within string literals:

| Form | Example | Description |
|---|---|---|
| Short form | `"hello $name"` | Inserts the value of identifier `name`. |
| Expression form | `"score: ${player.score + 1}"` | Inserts the result of an arbitrary expression. |

**Lowering.** Interpolated strings shall be lowered to C# interpolated strings
(`$"..."`).

```prsm
val msg = "Player $name has ${hp} HP"
```

```csharp
// Generated C#
var msg = $"Player {name} has {hp} HP";
```

### 3.7 Escape sequences [lex.escape]

The following escape sequences are recognized inside string literals:

| Escape | Character |
|---|---|
| `\n` | Newline (U+000A) |
| `\t` | Horizontal tab (U+0009) |
| `\r` | Carriage return (U+000D) |
| `\\` | Backslash |
| `\"` | Double quote |
| `\$` | Dollar sign (suppresses interpolation) |

Any other escape sequence is ill-formed; a conforming implementation shall
issue a diagnostic.

### 3.8 Operators and punctuation [lex.op]

#### 3.8.1 Operators

The following operator tokens are defined, listed from lowest to highest
precedence:

| Precedence | Operator(s) | Associativity | Description |
|:---:|---|---|---|
| 1 | `?:` | Right | Elvis (null coalescing) |
| 2 | `\|\|` | Left | Logical OR |
| 3 | `&&` | Left | Logical AND |
| 4 | `==` `!=` | Left | Equality |
| 5 | `<` `>` `<=` `>=` `is` | Left | Comparison, type check |
| 6 | `..` `until` `downTo` | --- | Range |
| 7 | `+` `-` | Left | Additive |
| 8 | `*` `/` `%` | Left | Multiplicative |
| 9 | `!` `-` (unary) | Right | Unary prefix |
| 10 | `.` `?.` `!!` `[]` `()` | Left | Postfix |

#### 3.8.2 Assignment operators

Assignment is a statement, not an expression. The following assignment
operators are defined:

| Operator | Description |
|---|---|
| `=` | Simple assignment |
| `+=` `-=` `*=` `/=` `%=` | Compound assignment |

Assignment to a `val` binding is ill-formed (E040).

---

## 4 Types [type]

### 4.1 Primitive types [type.prim]

PrSM defines the following primitive types. Each shall be lowered to its C#
equivalent as shown:

| PrSM type | C# type | Category |
|---|---|---|
| `Int` | `int` | Value |
| `Float` | `float` | Value |
| `Double` | `double` | Value |
| `Bool` | `bool` | Value |
| `String` | `string` | Reference |
| `Char` | `char` | Value |
| `Long` | `long` | Value |
| `Byte` | `byte` | Value |
| `Unit` | `void` | Return type only |

`Unit` shall appear only in return-type position. A function with no explicit
return type and a block body has return type `Unit`.

### 4.2 Nullable types [type.nullable]

A type followed by `?` denotes the nullable variant of that type.

- For reference types, `T?` permits the value `null`.
- For value types, `T?` lowers to `System.Nullable<T>` (e.g., `Int?` becomes
  `int?`).

A type without `?` is non-nullable. Assigning `null` to a non-nullable
position is ill-formed.

```prsm
var name: String = "Alice"     // non-nullable
var title: String? = null      // nullable
```

**Null-safe operators.** The following operators are provided for nullable
types:

| Operator | Name | Semantics |
|---|---|---|
| `?.` | Safe call | Short-circuits to `null` if receiver is `null`. |
| `?:` | Elvis | Returns left operand if non-null, otherwise right operand. |
| `!!` | Non-null assert | Asserts non-null at runtime; throws `NullReferenceException` if `null`. Emits warning **W001** if the operand is already non-nullable. |

**Unity null semantics.** For types derived from `UnityEngine.Object`, the
compiler shall use Unity's overloaded equality operators for null checks. A
`require` field is guaranteed non-null after `Awake()` but may become
"Unity-null" (destroyed) at runtime; this is not tracked statically.

### 4.3 Generic types [type.generic]

PrSM provides the following generic type aliases. Each shall be lowered to the
corresponding .NET type:

| PrSM type | C# type |
|---|---|
| `Array<T>` | `T[]` |
| `List<T>` | `System.Collections.Generic.List<T>` |
| `Map<K, V>` | `System.Collections.Generic.Dictionary<K, V>` |
| `Set<T>` | `System.Collections.Generic.HashSet<T>` |
| `Queue<T>` | `System.Collections.Generic.Queue<T>` |
| `Stack<T>` | `System.Collections.Generic.Stack<T>` |
| `Seq<T>` | `System.Collections.Generic.IEnumerable<T>` |

### 4.4 Unity and external types [type.unity]

Any type name that is not a PrSM primitive or generic alias shall be passed
through to C# unchanged. This includes all Unity types (`Transform`,
`Rigidbody`, `Vector3`, `Quaternion`, etc.) and user-defined C# types visible
to the Unity compilation pipeline.

```prsm
serialize target: Transform = null
val offset: Vector3 = Vector3.zero
```

### 4.5 Type inference [type.infer]

Local variable declarations with an initializer may omit the type annotation
when the type of the initializer is unambiguous:

```prsm
val name = "Player"       // inferred as String
val hp = 100              // inferred as Int
val speed = 5.0           // inferred as Float
var alive = true          // inferred as Bool
```

Type inference is **local only**. The following positions shall always require
explicit type annotations:

- Function parameters.
- Function return types on block-bodied functions.
- `require`, `optional`, `child`, and `parent` fields.
- Fields without initializers.

### 4.6 Generic type inference [type.infer.generic] (PrSM 2 부터)

When a variable declaration has an explicit type annotation, generic sugar
methods (`get`, `find`, `child`, `parent`, `require`) may omit their type
argument. The compiler shall infer the type argument from the declaration's
type annotation.

```prsm
val rb: Rigidbody = get()         // inferred: GetComponent<Rigidbody>()
val health: Health? = child()     // inferred: GetComponentInChildren<Health>()
```

If the target type cannot be determined, the compiler shall issue **E060**.

### 4.7 Type conversions [type.conv]

PrSM does not define implicit type conversions. All type relationships are
delegated to the C# type system. Explicit conversions may be performed via
`intrinsic` blocks or by relying on C# implicit conversions at the generated
code level.

---

## 5 Declarations [decl]

### 5.1 File structure [decl.file]

A PrSM source file has the following structure:

```ebnf
File = { UsingDecl } Declaration ;
```

Each file shall contain exactly one top-level declaration. The file name shall
match the declared type name (e.g., `Player.prsm` for `component Player`).

### 5.2 Using declarations [decl.using]

```ebnf
UsingDecl = "using" QualifiedName NEWLINE ;
```

A `using` declaration brings a .NET or Unity namespace into scope. It shall
appear before the top-level declaration.

```prsm
using UnityEngine
using System.Collections.Generic
```

**Lowering.** Each `using` declaration lowers to a C# `using` directive.

### 5.3 Component declarations [decl.component]

```ebnf
ComponentDecl = "component" IDENT [ ":" TypeRef { "," TypeRef } ] "{" { ComponentMember } "}" ;
```

A `component` declaration defines a Unity MonoBehaviour. The optional type
references after `:` specify implemented interfaces. A component implicitly
extends `MonoBehaviour`; specifying a base class is ill-formed.

**Lowering:**

```prsm
component Player : IDamageable {
    var health: Int = 100

    update {
        move()
    }

    func move() {
        transform.Translate(Vector3.forward * Time.deltaTime)
    }
}
```

```csharp
// Generated C#
public class Player : MonoBehaviour, IDamageable {
    [SerializeField] public int health = 100;

    private void Update() {
        move();
    }

    public void move() {
        transform.Translate(Vector3.forward * Time.deltaTime);
    }
}
```

**Constraints:**

1. A component shall not extend another component (E001).
2. A component shall not declare a constructor (E002).
3. Exactly one component per file (E003).
4. Components shall not be nested inside other declarations (E004).

### 5.4 Asset declarations [decl.asset]

```ebnf
AssetDecl = "asset" IDENT [ ":" TypeRef ] "{" { AssetMember } "}" ;
```

An `asset` declaration defines a Unity ScriptableObject. Assets may contain
fields and functions but shall not contain lifecycle blocks (E012) or
component-lookup fields (E013).

**Lowering:**

```prsm
asset WeaponData {
    val damage: Int = 10
    val range: Float = 5.0

    func dps(attackSpeed: Float): Float = damage / attackSpeed
}
```

```csharp
// Generated C#
[CreateAssetMenu(fileName = "New WeaponData", menuName = "PrSM/WeaponData")]
public class WeaponData : ScriptableObject {
    [SerializeField] private int _damage = 10;
    public int damage => _damage;
    [SerializeField] private float _range = 5.0f;
    public float range => _range;

    public float dps(float attackSpeed) => _damage / attackSpeed;
}
```

The compiler shall automatically generate the `[CreateAssetMenu]` attribute.

### 5.5 Class declarations [decl.class]

```ebnf
ClassDecl = "class" IDENT [ ":" TypeRef { "," TypeRef } ] "{" { ClassMember } "}" ;
```

A `class` declaration defines a plain C# class. Classes support single
inheritance and multiple interface implementation. Classes shall not contain
lifecycle blocks (E012) or component-lookup fields (E013). Fields in classes
are **not** serialized by default.

**Lowering:**

```prsm
class DamageCalculator {
    func compute(base: Int, multiplier: Float): Int {
        return (base * multiplier).toInt()
    }
}
```

```csharp
// Generated C#
public class DamageCalculator {
    public int compute(int @base, float multiplier) {
        return (int)(@base * multiplier);
    }
}
```

### 5.6 Data class declarations [decl.data]

```ebnf
DataClassDecl = "data" "class" IDENT "(" ParamList ")" ;
```

A `data class` declares a class with value semantics. The compiler shall
generate:

1. A constructor accepting all declared fields.
2. `Equals(object)` and `GetHashCode()` based on all fields.
3. `ToString()` returning a human-readable representation.

```prsm
data class DamageResult(val amount: Int, val wasCritical: Bool)
```

```csharp
// Generated C#
public class DamageResult {
    public int amount { get; }
    public bool wasCritical { get; }

    public DamageResult(int amount, bool wasCritical) {
        this.amount = amount;
        this.wasCritical = wasCritical;
    }

    public override bool Equals(object obj) { /* field-wise equality */ }
    public override int GetHashCode() { /* field-wise hash */ }
    public override string ToString() => $"DamageResult(amount={amount}, wasCritical={wasCritical})";
}
```

### 5.7 Enum declarations [decl.enum]

#### 5.7.1 Simple enums

```ebnf
EnumDecl = "enum" IDENT "{" EnumEntry { "," EnumEntry } [ "," ] "}" ;
```

A simple enum lowers to a C# `enum`. Entries are comma-separated; a trailing
comma is permitted.

```prsm
enum Direction {
    Up, Down, Left, Right
}
```

```csharp
// Generated C#
public enum Direction { Up, Down, Left, Right }
```

An enum shall have at least one entry (E050). Duplicate entry names are
ill-formed (E051).

#### 5.7.2 Parameterized enums

```ebnf
EnumDecl = "enum" IDENT "(" ParamList ")" "{" EnumEntry { "," EnumEntry } [ "," ] "}" ;
EnumEntry = IDENT "(" ExprList ")" ;
```

A parameterized enum generates the C# `enum` plus an extension class
containing one extension method per parameter.

```prsm
enum Weapon(val damage: Int, val range: Float) {
    Sword(10, 1.5),
    Bow(7, 15.0),
    Staff(15, 8.0)
}
```

```csharp
// Generated C#
public enum Weapon { Sword, Bow, Staff }

public static class WeaponExtensions {
    public static int damage(this Weapon self) => self switch {
        Weapon.Sword => 10,
        Weapon.Bow => 7,
        Weapon.Staff => 15,
        _ => throw new System.ArgumentOutOfRangeException()
    };

    public static float range(this Weapon self) => self switch {
        Weapon.Sword => 1.5f,
        Weapon.Bow => 15.0f,
        Weapon.Staff => 8.0f,
        _ => throw new System.ArgumentOutOfRangeException()
    };
}
```

Enum parameters shall be `val` (E052).

### 5.8 Attribute declarations [decl.attr]

```ebnf
AttributeDecl = "attribute" IDENT [ "(" ParamList ")" ] "{" { ClassMember } "}" ;
```

An `attribute` declaration defines a custom C# attribute. Parameters become
the attribute constructor parameters.

```prsm
attribute Cooldown(val seconds: Float)
```

```csharp
// Generated C#
[System.AttributeUsage(System.AttributeTargets.All)]
public class CooldownAttribute : System.Attribute {
    public float seconds { get; }
    public CooldownAttribute(float seconds) {
        this.seconds = seconds;
    }
}
```

### 5.9 Feature gates [decl.feature] (PrSM 2 부터)

The `.prsmproject` file controls the language version and enabled feature set.
A conforming implementation shall read the `language.version` field to
determine the language level and shall enable only those features listed in the
`features` array.

```json
{
  "language": {
    "version": 2,
    "features": ["pattern-bindings", "input-system", "auto-unlisten"]
  }
}
```

The following feature identifiers are defined:

| Feature ID | Description |
|---|---|
| `pattern-bindings` | Enables `val` binding patterns in `when` branches. |
| `input-system` | Enables Input System integration for `listen` blocks. |
| `auto-unlisten` | Enables automatic unsubscription in `onDisable`. |

A program that uses a gated feature without listing it in the `features` array
is ill-formed (E070).

---

## 6 Fields [field]

### 6.1 Field declarations [field.decl]

```ebnf
FieldDecl = { Annotation } [ VisibilityMod ] FieldKind IDENT ":" TypeRef [ "=" Expr ] NEWLINE ;
FieldKind = "serialize" | "require" | "optional" | "child" | "parent" | "val" | "var" ;
```

A field declaration introduces a named member into the enclosing type. The
field kind determines mutability, serialization, and initialization behavior.

### 6.2 Serialize fields [field.serialize]

The `serialize` keyword explicitly marks a field for Unity serialization.

```prsm
serialize val speed: Float = 5.0
serialize var health: Int = 100
```

The `val` or `var` modifier after `serialize` controls mutability:

- `serialize val` -- the field is serialized and read-only from PrSM code.
- `serialize var` -- the field is serialized and mutable.
- `serialize` alone (without `val`/`var`) is equivalent to `serialize var`.

**Lowering for `serialize val`:**

```csharp
[SerializeField] private float _speed = 5.0f;
public float speed => _speed;
```

**Lowering for `serialize var`:**

```csharp
[SerializeField] public int health = 100;
```

### 6.3 Val and var fields [field.valvar]

| Qualifier | Mutability | Serialized in component/asset? | C# lowering (component) |
|---|---|---|---|
| `val` | Immutable after init | Yes (serializable types) | `[SerializeField] private T _f; public T f => _f;` |
| `var` | Mutable | Yes (serializable types) | `[SerializeField] public T f;` |

In `class` declarations, `val` and `var` fields are **not** serialized. They
lower to plain C# fields or properties.

Assigning to a `val` field after initialization is ill-formed (**E040**).

```prsm
component Player {
    val maxHp: Int = 100
    var currentHp: Int = 100

    func takeDamage(amount: Int) {
        currentHp -= amount
        // maxHp = 200  // E040: cannot assign to val
    }
}
```

```csharp
// Generated C#
public class Player : MonoBehaviour {
    [SerializeField] private int _maxHp = 100;
    public int maxHp => _maxHp;
    [SerializeField] public int currentHp = 100;

    public void takeDamage(int amount) {
        currentHp -= amount;
    }
}
```

### 6.4 Visibility [field.vis]

Fields have three visibility levels:

| PrSM modifier | C# modifier | Default for |
|---|---|---|
| `public` | `public` | component, asset fields |
| `private` | `private` | -- |
| `protected` | `protected` | -- |

If no visibility modifier is specified, component and asset fields default to
`public`. Class fields default to `public`.

### 6.5 Component-lookup fields [field.lookup]

The `require`, `optional`, `child`, and `parent` field kinds are valid **only**
inside `component` declarations. Using them in an `asset` or `class` is
ill-formed (**E013**).

#### 6.5.1 require

```prsm
require rb: Rigidbody
```

Declares a non-nullable dependency on a sibling component. The compiler shall
generate a `GetComponent<T>()` call in `Awake()`. If the component is not
found, the implementation shall log an error and disable the component.

**Lowering:**

```csharp
private Rigidbody _rb;
public Rigidbody rb => _rb;

// In Awake():
_rb = GetComponent<Rigidbody>();
if (_rb == null) {
    Debug.LogError($"[Player] Required component Rigidbody not found on {gameObject.name}", this);
    enabled = false;
    return;
}
```

A `require` field shall not have an initializer (E041). A `require` field
shall refer to a type derived from `UnityEngine.Component` (E042).

#### 6.5.2 optional

```prsm
optional audioSrc: AudioSource
```

Declares a nullable dependency. Fetched via `GetComponent<T>()` in `Awake()`.
If not found, the field remains `null` and no error is issued. The field's
effective type is `T?`.

#### 6.5.3 child

```prsm
child healthBar: HealthBar
```

Like `require`, but uses `GetComponentInChildren<T>()`. The field is
non-nullable; a missing child component triggers an error and disables the
component.

#### 6.5.4 parent

```prsm
parent manager: GameManager
```

Like `require`, but uses `GetComponentInParent<T>()`. The field is
non-nullable; a missing parent component triggers an error and disables the
component.

#### 6.5.5 Lookup summary

| Qualifier | Fetch method | Null on miss | Field nullability |
|---|---|---|---|
| `require` | `GetComponent<T>()` | Error + disable | Non-null `T` |
| `optional` | `GetComponent<T>()` | Silent | Nullable `T?` |
| `child` | `GetComponentInChildren<T>()` | Error + disable | Non-null `T` |
| `parent` | `GetComponentInParent<T>()` | Error + disable | Non-null `T` |

### 6.6 Field annotations [field.ann]

Annotations modify serialization or Inspector presentation. They are placed
before the field declaration.

```ebnf
Annotation = "@" IDENT [ "(" AnnotationArgs ")" ] NEWLINE ;
```

The following built-in annotations are defined:

| Annotation | C# attribute | Description |
|---|---|---|
| `@header("text")` | `[Header("text")]` | Section header in Inspector. |
| `@tooltip("text")` | `[Tooltip("text")]` | Hover tooltip in Inspector. |
| `@range(min, max)` | `[Range(min, max)]` | Numeric slider in Inspector. |
| `@space` | `[Space]` | Visual gap in Inspector. |
| `@space(n)` | `[Space(n)]` | Visual gap of `n` pixels. |
| `@hideInInspector` | `[HideInInspector]` | Hides the field from Inspector. |

```prsm
@header("Movement")
@tooltip("Units per second")
@range(0, 20)
serialize val speed: Float = 5.0

@space
@hideInInspector
var internalTimer: Float = 0.0
```

### 6.7 Initialization order [field.init]

Within a component, fields are initialized in the following order. A
conforming implementation shall preserve this sequence:

1. **Unity `Awake()` entry** -- the runtime calls the generated `Awake` method.
2. **Component-lookup resolution** -- `require`, `optional`, `child`, and
   `parent` fields are resolved via `GetComponent` variants. If any `require`,
   `child`, or `parent` lookup fails, the component is disabled and
   initialization halts; the user `awake` block does **not** execute.
3. **Serialized field defaults** -- Unity applies serialized values from the
   Inspector or asset data (these are injected by Unity before `Awake`).
4. **User `awake` body** -- the body of the `awake` lifecycle block executes.
5. **Unity `Start()` entry** -- on the first frame, the generated `Start`
   method runs the user's `start` lifecycle block body.

```prsm
component Player {
    require rb: Rigidbody
    optional audio: AudioSource
    val maxSpeed: Float = 10.0

    awake {
        rb.useGravity = false
    }

    start {
        // runs on first frame
    }
}
```

```csharp
// Generated C#
public class Player : MonoBehaviour {
    private Rigidbody _rb;
    public Rigidbody rb => _rb;
    private AudioSource _audio;
    public AudioSource audio => _audio;
    [SerializeField] private float _maxSpeed = 10.0f;
    public float maxSpeed => _maxSpeed;

    private void Awake() {
        // Step 2: lookup resolution
        _rb = GetComponent<Rigidbody>();
        if (_rb == null) {
            Debug.LogError($"[Player] Required component Rigidbody not found on {gameObject.name}", this);
            enabled = false;
            return;
        }
        _audio = GetComponent<AudioSource>();

        // Step 4: user awake body
        _rb.useGravity = false;
    }

    private void Start() {
        // Step 5: user start body
    }
}
```

---

## 7 Functions [func]

### 7.1 Function declarations [func.decl]

```ebnf
FuncDecl = [ VisibilityMod ] [ "override" ] "func" IDENT "(" [ ParamList ] ")" [ ":" TypeRef ]
           ( Block | "=" Expr NEWLINE ) ;
```

A function declaration introduces a named callable member. If the return type
is omitted and the body is a block, the return type is `Unit`. If the body is
an expression (`= Expr`), the return type is inferred from the expression.

### 7.2 Parameters [func.param]

```ebnf
ParamList = Param { "," Param } ;
Param     = IDENT ":" TypeRef [ "=" Expr ] ;
```

Parameters shall have explicit type annotations. Default values are permitted;
parameters with defaults shall appear after parameters without defaults.

```prsm
func attack(target: Enemy, damage: Int = 10) {
    target.takeDamage(damage)
}
```

### 7.3 Expression-bodied functions [func.expr]

A function may use `= Expr` as its body in place of a block. This is
syntactic sugar for a block containing a single `return` statement.

```prsm
func isAlive(): Bool = hp > 0
func greeting(): String = "Hello, $name!"
```

```csharp
// Generated C#
public bool isAlive() => hp > 0;
public string greeting() => $"Hello, {name}!";
```

### 7.4 Named arguments [func.named]

At the call site, arguments may be passed by name using the syntax
`name = value`. Named arguments may appear in any order but shall not precede
positional arguments.

```prsm
func spawn(x: Float, y: Float, z: Float = 0.0) { /* ... */ }

// Call sites:
spawn(1.0, 2.0)
spawn(x = 1.0, y = 2.0, z = 3.0)
spawn(1.0, z = 5.0, y = 2.0)
```

### 7.5 Override [func.override]

The `override` modifier shall be used when a function overrides an inherited
or interface method. Omitting `override` when an override is required is
ill-formed (**E030**).

```prsm
component Player : IDamageable {
    override func takeDamage(amount: Int) {
        health -= amount
    }
}
```

```csharp
// Generated C#
public void takeDamage(int amount) {
    health -= amount;
}
```

For well-known overrides such as `toString()`, the compiler shall emit the
appropriate C# `override`:

```prsm
override func toString(): String = "Player($name)"
```

```csharp
// Generated C#
public override string ToString() => $"Player({name})";
```

### 7.6 Visibility [func.vis]

Functions have three visibility levels:

| PrSM modifier | C# modifier |
|---|---|
| `public` (default) | `public` |
| `private` | `private` |
| `protected` | `protected` |

If no modifier is specified, functions default to `public`.

### 7.7 Intrinsic functions [func.intrinsic]

```ebnf
IntrinsicFunc = "intrinsic" "func" IDENT "(" [ ParamList ] ")" [ ":" TypeRef ] Block ;
```

An intrinsic function contains raw C# code in its body. The compiler shall
emit the body verbatim into the generated C# output without transformation.

```prsm
intrinsic func setLayer(layer: Int) {
    gameObject.layer = layer;
}
```

```csharp
// Generated C#
public void setLayer(int layer) {
    gameObject.layer = layer;
}
```

### 7.8 Intrinsic coroutines [func.intrinsic.coro]

```ebnf
IntrinsicCoro = "intrinsic" "coroutine" IDENT "(" [ ParamList ] ")" Block ;
```

An intrinsic coroutine contains raw C# coroutine code. The compiler shall emit
it as a method returning `System.Collections.IEnumerator`.

```prsm
intrinsic coroutine flashEffect() {
    GetComponent<Renderer>().material.color = Color.red;
    yield return new WaitForSeconds(0.1f);
    GetComponent<Renderer>().material.color = Color.white;
}
```

```csharp
// Generated C#
public System.Collections.IEnumerator flashEffect() {
    GetComponent<Renderer>().material.color = Color.red;
    yield return new WaitForSeconds(0.1f);
    GetComponent<Renderer>().material.color = Color.white;
}
```

---

## 8 Lifecycle blocks [lifecycle]

### 8.1 General [lifecycle.general]

A lifecycle block is an anonymous block that lowers to a Unity message method.
Lifecycle blocks shall appear only inside `component` declarations. Using a
lifecycle block inside an `asset` or `class` is ill-formed (**E012**).

```ebnf
LifecycleBlock = LifecycleName [ LifecycleParam ] Block ;
LifecycleParam = "(" IDENT ":" TypeRef ")" ;
```

Each lifecycle kind shall appear at most once per component. A duplicate
lifecycle block is ill-formed (**E014**).

### 8.2 Lifecycle kinds [lifecycle.kinds]

The following lifecycle blocks are defined. Each shall lower to the
corresponding Unity message method:

| PrSM block | C# method | Parameter | Timing |
|---|---|---|---|
| `awake` | `Awake()` | -- | Instance creation |
| `start` | `Start()` | -- | Before first frame |
| `update` | `Update()` | -- | Every frame |
| `fixedUpdate` | `FixedUpdate()` | -- | Fixed timestep |
| `lateUpdate` | `LateUpdate()` | -- | After all `Update` calls |
| `onEnable` | `OnEnable()` | -- | Component enabled |
| `onDisable` | `OnDisable()` | -- | Component disabled |
| `onDestroy` | `OnDestroy()` | -- | Component destroyed |
| `onTriggerEnter` | `OnTriggerEnter(Collider)` | `Collider` | Trigger entered |
| `onTriggerExit` | `OnTriggerExit(Collider)` | `Collider` | Trigger exited |
| `onTriggerStay` | `OnTriggerStay(Collider)` | `Collider` | Trigger ongoing |
| `onCollisionEnter` | `OnCollisionEnter(Collision)` | `Collision` | Collision entered |
| `onCollisionExit` | `OnCollisionExit(Collision)` | `Collision` | Collision exited |
| `onCollisionStay` | `OnCollisionStay(Collision)` | `Collision` | Collision ongoing |

### 8.3 Parameterless lifecycle blocks [lifecycle.noparam]

Blocks without parameters (`awake`, `start`, `update`, `fixedUpdate`,
`lateUpdate`, `onEnable`, `onDisable`, `onDestroy`) lower to `private void`
methods with no parameters.

```prsm
component Spinner {
    var angle: Float = 0.0

    update {
        angle += 90.0 * Time.deltaTime
        transform.rotation = Quaternion.Euler(0, angle, 0)
    }
}
```

```csharp
// Generated C#
public class Spinner : MonoBehaviour {
    [SerializeField] public float angle = 0.0f;

    private void Update() {
        angle += 90.0f * Time.deltaTime;
        transform.rotation = Quaternion.Euler(0, angle, 0);
    }
}
```

### 8.4 Parameterized lifecycle blocks [lifecycle.param]

Trigger and collision blocks accept a single parameter. The parameter name is
user-chosen; the type is determined by the lifecycle kind:

- `onTriggerEnter`, `onTriggerExit`, `onTriggerStay` -- parameter type is
  `Collider`.
- `onCollisionEnter`, `onCollisionExit`, `onCollisionStay` -- parameter type
  is `Collision`.

If the parameter type annotation is provided, it shall match the expected
type; otherwise it is inferred.

```prsm
component Coin {
    onTriggerEnter(other: Collider) {
        if other.CompareTag("Player") {
            destroy()
        }
    }
}
```

```csharp
// Generated C#
public class Coin : MonoBehaviour {
    private void OnTriggerEnter(Collider other) {
        if (other.CompareTag("Player")) {
            Destroy(gameObject);
        }
    }
}
```

### 8.5 Lowering rules [lifecycle.lower]

For each lifecycle block present in a component, the compiler shall generate a
corresponding `private void` C# method. The body of the PrSM block shall
become the method body, with standard expression and statement lowering
applied.

When a component contains both component-lookup fields (S6.5) and an `awake`
block, the compiler shall generate a single `Awake()` method that performs
lookup resolution first and then executes the user block body, as specified in
S6.7.

### 8.6 Lifecycle blocks and coroutines [lifecycle.coro]

Lifecycle block bodies shall not contain `wait` statements directly (E015).
Coroutine operations are only valid inside `coroutine` declarations (S9).
A lifecycle block may, however, start a coroutine via `start coroutineName()`.

```prsm
component FadeIn {
    coroutine doFade() {
        wait 1.0s
        // fade logic
    }

    start {
        start doFade()
    }
}
```

### 8.7 Error summary [lifecycle.errors]

| Code | Condition |
|---|---|
| **E012** | Lifecycle block in `asset` or `class` declaration. |
| **E014** | Duplicate lifecycle block of the same kind in a single component. |
| **E015** | `wait` statement directly inside a lifecycle block body. |
## 9 Expressions

### 9.1 Operator Precedence

The following table lists all operators from **lowest** to **highest** precedence. Operators at the same precedence level associate left-to-right unless otherwise noted.

| Level | Operators | Associativity | Description |
|-------|-----------|---------------|-------------|
| 1 | `?:` | Right | Elvis (null coalescing) |
| 2 | `\|\|` | Left | Logical OR |
| 3 | `&&` | Left | Logical AND |
| 4 | `==` `!=` | Left | Equality |
| 5 | `<` `>` `<=` `>=` `is` | Left | Comparison / type check |
| 6 | `..` `until` `downTo` | None | Range construction |
| 7 | `+` `-` | Left | Additive |
| 8 | `*` `/` `%` | Left | Multiplicative |
| 9 | `!` `-` (unary) | Right (prefix) | Unary |
| 10 | `.` `?.` `!!` `[]` `()` | Left | Postfix / member access |

### 9.2 Binary Operators

The compiler shall support the following binary operators with their standard semantics:

| Operator | Meaning | Operand types |
|----------|---------|---------------|
| `+` | Addition / string concatenation | Numeric, String |
| `-` | Subtraction | Numeric |
| `*` | Multiplication | Numeric |
| `/` | Division | Numeric |
| `%` | Remainder | Numeric |
| `==` | Structural equality | Any |
| `!=` | Structural inequality | Any |
| `<` `>` `<=` `>=` | Ordering | Numeric, IComparable |
| `&&` | Short-circuit logical AND | Boolean |
| `\|\|` | Short-circuit logical OR | Boolean |

The compiler shall lower `==` and `!=` to C# `==` and `!=`. For Unity objects this preserves Unity's custom equality semantics.

### 9.3 Unary Operators

| Operator | Meaning | Operand type |
|----------|---------|--------------|
| `!` | Logical NOT | Boolean |
| `-` | Numeric negation | Numeric |

### 9.4 Null-Safe Operators

#### Safe member access (`?.`)

```prsm
val name = enemy?.name
```

The expression `a?.b` shall evaluate to `null` when `a` is null, and to `a.b` otherwise. The result type is the nullable variant of the member type.

**Lowering:**

```csharp
var name = enemy != null ? enemy.name : null;
```

#### Elvis operator (`?:`)

```prsm
val name = player?.name ?: "Unknown"
```

The expression `a ?: b` shall evaluate to `a` when `a` is non-null, and to `b` otherwise. The right operand shall be type-compatible with the non-null form of the left operand.

**Lowering:**

```csharp
var name = player?.name ?? "Unknown";
```

#### Non-null assertion (`!!`)

```prsm
val name = nullableName!!
```

The expression `a!!` shall assert that `a` is non-null at runtime. If `a` is null the compiler shall emit code that throws `NullReferenceException`. The compiler shall emit warning **W001** when `!!` is applied to an expression whose type is already non-nullable.

**Lowering:**

```csharp
var name = nullableName ?? throw new System.NullReferenceException(
    "Non-null assertion failed");
```

### 9.5 Type Check (`is`)

```prsm
if enemy is Boss {
    enemy.enrage()
}
```

The expression `expr is Type` shall evaluate to `true` when the runtime type of `expr` is `Type` or a subtype thereof. After a successful `is` check in a condition, the compiler shall smart-cast the variable to `Type` within the truthy branch.

**Lowering:**

```csharp
if (enemy is Boss) {
    enemy.enrage();
}
```

### 9.6 `if` Expression

When `if` appears in expression position, both branches are required and the compiler shall produce a value.

```prsm
val max = if a > b { a } else { b }
```

Both branches shall produce types that share a common supertype. The `else` branch is mandatory for expression-form `if` (E100 if missing).

**Lowering:**

```csharp
var max = (a > b) ? a : b;
```

### 9.7 `when` Expression

When `when` appears in expression position, it shall be exhaustive. For enum subjects, every variant shall be covered or an `else` branch shall be present. Non-exhaustive `when` expressions produce **E100**.

```prsm
val label = when state {
    State.Idle => "Idle"
    State.Running => "Moving"
    else => "Unknown"
}
```

**Lowering:**

```csharp
var label = state switch {
    State.Idle => "Idle",
    State.Running => "Moving",
    _ => "Unknown"
};
```

### 9.8 Range Expressions

Range expressions construct sequences for iteration:

| PrSM | Semantics | Lowered `for` equivalent |
|------|-----------|--------------------------|
| `start..end` | Inclusive `[start, end]` | `i <= end` |
| `start until end` | Exclusive upper `[start, end)` | `i < end` |
| `start downTo end` | Descending `[start, end]` | `i >= end; i--` |
| `expr step N` | Stride modifier | `i += N` or `i -= N` |

```prsm
for i in 0 until 10 step 2 {
    log("$i")
}
```

**Lowering:**

```csharp
for (int i = 0; i < 10; i += 2) {
    Debug.Log($"{i}");
}
```

### 9.9 Sugar Calls

The compiler shall recognize the following call-site sugar and lower them to their Unity equivalents. Sugar calls are resolved during lowering, not as identifiers in scope.

| PrSM | C# |
|------|-----|
| `vec2(x, y)` | `new Vector2(x, y)` |
| `vec3(x, y, z)` | `new Vector3(x, y, z)` |
| `color(r, g, b, a)` | `new Color(r, g, b, a)` |
| `get<T>()` | `GetComponent<T>()` |
| `find<T>()` | `FindFirstObjectByType<T>()` |
| `child<T>()` | `GetComponentInChildren<T>()` |
| `parent<T>()` | `GetComponentInParent<T>()` |
| `log(msg)` | `Debug.Log(msg)` |
| `warn(msg)` | `Debug.LogWarning(msg)` |
| `error(msg)` | `Debug.LogError(msg)` |

#### Method sugar (legacy input)

| PrSM | C# |
|------|-----|
| `input.axis(name)` | `Input.GetAxis(name)` |
| `input.getKey(key)` | `Input.GetKey(key)` |
| `input.getKeyDown(key)` | `Input.GetKeyDown(key)` |
| `input.getKeyUp(key)` | `Input.GetKeyUp(key)` |
| `input.getMouseButton(n)` | `Input.GetMouseButton(n)` |

### 9.10 Generic Type Inference (PrSM 2 부터)

When a generic sugar call appears in a context with an unambiguous target type, the compiler shall infer the type argument.

```prsm
val rb: Rigidbody = get()   // infers get<Rigidbody>()
```

Inference rules, in priority order:

1. **Variable type annotation** -- infer from the declared type of the receiving variable.
2. **Return type** -- infer from the enclosing function's return type.
3. **Argument type** -- infer from a parameter type at the call site.

The compiler shall require a single unambiguous solution. If inference fails, the compiler shall emit **E020** requesting an explicit type argument.

### 9.11 Pattern Bindings in `when` (PrSM 2 부터)

When a `when` branch matches a payload enum variant, bindings extract the payload values:

```prsm
when state {
    EnemyState.Chase(target) => moveTo(target)
    EnemyState.Stunned(dur) if dur > 0.0 => waitStun(dur)
    else => idle()
}
```

Binding arity shall match the enum entry's parameter count; a mismatch produces **E082**. Referencing a variant not defined in the enum produces **E081**.

---

## 10 Statements

### 10.1 `val` Declaration

```prsm
val name: Type = initializer
val name = initializer          // type inferred
```

The `val` keyword declares an immutable binding. The variable shall not be reassigned after initialization (**E040**). Either a type annotation or an initializer (or both) shall be present (**E022**).

### 10.2 `var` Declaration

```prsm
var name: Type = initializer
var name: Type                  // uninitialized, type required
var name = initializer          // type inferred
```

The `var` keyword declares a mutable binding. A `var` declaration with neither a type annotation nor an initializer shall produce **E022**.

### 10.3 Assignment

```prsm
target = value
```

Simple assignment. The compiler shall reject assignment to `val` bindings (**E040**) and to `require` fields (**E041**).

#### Compound Assignment

The compound assignment operators `+=`, `-=`, `*=`, `/=`, `%=` shall desugar to `target = target op value`.

```prsm
health -= damage
// equivalent to: health = health - damage
```

### 10.4 `if` / `else`

```prsm
if condition {
    body
} else if otherCondition {
    body
} else {
    body
}
```

The condition shall be of type `Boolean`. Parentheses around the condition are **not required** (unlike C#). The braces are mandatory.

**Lowering:**

```csharp
if (condition) {
    // body
} else if (otherCondition) {
    // body
} else {
    // body
}
```

### 10.5 `when` Statement

#### Subject form

```prsm
when subject {
    Pattern => body
    Pattern => body
    else => fallback
}
```

The compiler shall evaluate the subject once and match branches top-to-bottom. The first matching branch executes.

#### Condition form

```prsm
when {
    health < 20 => flee()
    health < 50 => defend()
    else => attack()
}
```

Without a subject, each branch condition is an independent Boolean expression. The compiler shall lower this to an `if`/`else if` chain.

#### Exhaustiveness

When a `when` statement matches on an enum type and does not cover all variants and has no `else` branch, the compiler shall emit warning **W003**. (PrSM 2 부터)

#### Pattern bindings

```prsm
when result {
    Result.Ok(val value) => log("$value")
    Result.Err(val msg) => error(msg)
}
```

Bindings extract payload values from enum variants. The binding count shall match the variant's parameter count (**E082**). An unknown variant name shall produce **E081**.

#### Guards

```prsm
when state {
    EnemyState.Stunned(dur) if dur > 2.0 => longStun()
    EnemyState.Stunned(dur) => shortStun()
}
```

A guard adds a post-match condition. If the pattern matches but the guard evaluates to `false`, the compiler shall fall through to the next branch.

**Lowering (pattern binding with guard):**

```csharp
switch (state.Tag) {
    case EnemyStateTag.Stunned:
        var dur = state.StunnedPayload.Item1;
        if (dur > 2.0f) { longStun(); break; }
        shortStun();
        break;
}
```

### 10.6 `for`

```prsm
for name in iterable {
    body
}
```

The `for` loop iterates over any value implementing `IEnumerable<T>` or having a compatible `GetEnumerator()` method. Range expressions (see 9.8) are lowered to allocation-free C-style `for` loops.

```prsm
for i in 0 until 10 { log("$i") }
for i in 10 downTo 1 step 2 { log("$i") }
```

#### Destructuring in `for`

```prsm
for Result.Ok(val value) in results {
    log("$value")
}
```

The loop variable may be a destructuring pattern. The compiler shall extract fields from each element using the same rules as pattern bindings.

### 10.7 `while`

```prsm
while condition {
    body
}
```

The compiler shall lower `while` directly to C# `while`. The condition shall be of type `Boolean`.

### 10.8 `return`

```prsm
return expr
return
```

The `return` statement exits the enclosing function. In functions with return type `Unit`, the expression shall be omitted.

### 10.9 `break` and `continue`

```prsm
break
continue
```

`break` and `continue` shall only appear inside `for` or `while` loop bodies. Use outside a loop shall produce **E031**.

### 10.10 Destructuring `val` (PrSM 2 부터)

```prsm
val Result.Ok(value) = expr
```

Destructuring extracts fields from a data class or enum payload into local bindings. The compiler shall lower to individual field access using `Item1`, `Item2`, etc.

### 10.11 `listen`

The `listen` statement wires a code block to a Unity event field. It shall lower to an `AddListener(...)` call appended to the generated `Awake()` body after all component lookups are resolved.

#### Basic form

```prsm
listen button.onClick {
    fire()
}
```

**Lowering:**

```csharp
void Awake() {
    // ... require/optional resolution ...
    button.onClick.AddListener(() => { fire(); });
}
```

#### With parameter

```prsm
listen slider.onValueChanged { val newValue ->
    log("$newValue")
}
```

**Lowering:**

```csharp
slider.onValueChanged.AddListener((newValue) => { Debug.Log($"{newValue}"); });
```

#### Lifetime modifiers

Lifetime modifiers shall only appear inside `component` declarations. Use outside a component shall produce **E083**.

##### `until disable`

```prsm
listen button.onClick until disable {
    fire()
}
```

The compiler shall generate a private handler field, register the listener in the lifecycle registration phase, and emit cleanup code in `OnDisable`.

**Lowering:**

```csharp
private System.Action _prsm_h0;

void Start() {
    _prsm_h0 = () => { fire(); };
    button.onClick.AddListener(_prsm_h0);
}

void OnDisable() {
    button.onClick.RemoveListener(_prsm_h0);
    _prsm_h0 = null;
}
```

##### `until destroy`

```prsm
listen spawner.onSpawn until destroy {
    count += 1
}
```

Same pattern as `until disable`, but cleanup runs in `OnDestroy`.

##### `manual`

```prsm
val token = listen timer.onFinished manual {
    reset()
}
```

The `manual` modifier returns a subscription token. The handler field is created but no automatic cleanup is registered.

##### `unlisten`

```prsm
unlisten token
```

The `unlisten` statement shall resolve the token to its backing handler field, emit `RemoveListener`, and null the field. It is valid in any method within the declaring component.

#### Default behavior

Without a lifetime modifier, `listen` shall register the listener only. No auto-cleanup is generated. This behavior is identical in Language 1 and Language 2.

### 10.12 Input System Sugar (PrSM 2 부터)

Input System sugar requires the `input-system` feature flag in `.prsmproject`. Using input sugar without the flag shall produce **E070**.

#### Basic form

```prsm
if input.action("Jump").pressed {
    jump()
}
val move = input.action("Move").vector2
```

#### State accessors

| PrSM accessor | C# method |
|---------------|-----------|
| `.pressed` | `WasPressedThisFrame()` |
| `.released` | `WasReleasedThisFrame()` |
| `.held` | `IsPressed()` |
| `.vector2` | `ReadValue<Vector2>()` |
| `.scalar` | `ReadValue<float>()` |

**Lowering:**

```csharp
// input.action("Jump").pressed
_prsmInput.actions["Jump"].WasPressedThisFrame()

// input.action("Move").vector2
_prsmInput.actions["Move"].ReadValue<UnityEngine.Vector2>()
```

#### Player form (multiplayer)

```prsm
if input.player("Gameplay").action("Fire").pressed {
    fireWeapon()
}
```

The compiler shall compose the lookup key as `"Map/Action"`:

```csharp
_prsmInput.actions["Gameplay/Fire"].WasPressedThisFrame()
```

#### Generated infrastructure

When the compiler detects input sugar in a component, it shall generate:

1. A private field: `private PlayerInput _prsmInput;`
2. A `GetComponent<PlayerInput>()` call merged into the generated `Awake()` body before user statements.

### 10.13 `intrinsic` Block

```prsm
intrinsic {
    // raw C# code
    var ray = Camera.main.ScreenPointToRay(Input.mousePosition);
}
```

The `intrinsic` block passes its content through to the generated C# verbatim. The PrSM semantic checker shall not inspect intrinsic content; validation is deferred to the C# compiler.

### 10.14 `start` / `stop` / `stopAll`

```prsm
start fadeOut(2.0)
stop fadeOut()
stopAll()
```

| PrSM | C# |
|------|-----|
| `start f()` | `StartCoroutine(f())` |
| `stop f()` | `StopCoroutine(nameof(f))` |
| `stopAll()` | `StopAllCoroutines()` |

These shall only appear inside `component` declarations where `MonoBehaviour` coroutine management is available.

---

## 11 Coroutines

### 11.1 Declaration

```prsm
coroutine fadeOut(duration: Float) {
    var elapsed = 0.0
    while elapsed < duration {
        alpha = 1.0 - (elapsed / duration)
        elapsed += Time.deltaTime
        wait nextFrame
    }
    alpha = 0.0
}
```

A `coroutine` declaration shall only appear inside a `component` body. Declaring a coroutine in an `asset` or `class` shall produce **E060**.

The compiler shall lower a coroutine to a `private IEnumerator` method:

```csharp
private System.Collections.IEnumerator fadeOut(float duration) {
    float elapsed = 0f;
    while (elapsed < duration) {
        alpha = 1f - (elapsed / duration);
        elapsed += Time.deltaTime;
        yield return null;
    }
    alpha = 0f;
}
```

### 11.2 `wait` Forms

The `wait` keyword shall only appear inside a `coroutine` body. Use outside a coroutine shall produce **E032**.

| PrSM | C# | Meaning |
|------|-----|---------|
| `wait 1.5s` | `yield return new WaitForSeconds(1.5f)` | Wait N seconds |
| `wait nextFrame` | `yield return null` | Wait one frame |
| `wait fixedFrame` | `yield return new WaitForFixedUpdate()` | Wait for next FixedUpdate |
| `wait until expr` | `yield return new WaitUntil(() => expr)` | Wait until condition is true |
| `wait while expr` | `yield return new WaitWhile(() => expr)` | Wait while condition is true |

#### Examples

```prsm
coroutine spawnWaves() {
    for wave in 1..10 {
        spawnEnemies(wave * 5)
        wait 30.0s
    }
}

coroutine waitForLanding() {
    wait until isGrounded
    land()
}
```

**Lowering:**

```csharp
private IEnumerator spawnWaves() {
    for (int wave = 1; wave <= 10; wave++) {
        spawnEnemies(wave * 5);
        yield return new WaitForSeconds(30f);
    }
}

private IEnumerator waitForLanding() {
    yield return new WaitUntil(() => isGrounded);
    land();
}
```

### 11.3 Start and Stop

Coroutines are started and stopped via the `start`, `stop`, and `stopAll` statements (see 10.14). The `start` statement may optionally capture a handle for later cancellation:

```prsm
val handle = start spawnWaves()
// ...
stop handle
```

---

## 12 Lowering Rules

This section defines the normative C# output that the compiler shall produce. Conforming implementations shall generate code with equivalent observable behavior.

### 12.1 Serialized Fields

```prsm
serialize speed: Float = 5.0
```

The compiler shall lower `serialize` fields to a `[SerializeField]` private backing field with a public read-only property:

```csharp
[SerializeField] private float _speed = 5.0f;
public float speed => _speed;
```

For `var` fields, the property shall have both a getter and a setter.

### 12.2 Awake Synthesis

When a component declares `require`, `optional`, `child`, or `parent` fields, the compiler shall generate a single `Awake()` method with the following phase order:

1. **Phase 1 -- Dependency resolution.** Resolve all injection fields in declaration order. `require` fields are checked for null; failure logs an error and disables the component via `enabled = false; return;`.
2. **Phase 2 -- Input infrastructure.** If input sugar is present, emit `_prsmInput = GetComponent<PlayerInput>()`.
3. **Phase 3 -- Listen registration.** Emit `AddListener` calls for all `listen` statements.
4. **Phase 4 -- User `awake` body.** Emit the user's `awake` block, if present.

If any `require` check fails, phases 2 through 4 shall not execute.

### 12.3 Decorator Lowering

| PrSM | C# attribute |
|------|-------------|
| `@header("text")` | `[Header("text")]` |
| `@tooltip("text")` | `[Tooltip("text")]` |
| `@range(min, max)` | `[Range(min, max)]` |
| `@space` | `[Space]` |
| `@space(n)` | `[Space(n)]` |
| `@hideInInspector` | `[HideInInspector]` |

### 12.4 Sugar Mapping Table (Complete)

| PrSM | C# |
|------|-----|
| `vec2(x, y)` | `new Vector2(x, y)` |
| `vec3(x, y, z)` | `new Vector3(x, y, z)` |
| `color(r, g, b, a)` | `new Color(r, g, b, a)` |
| `get<T>()` | `GetComponent<T>()` |
| `find<T>()` | `FindFirstObjectByType<T>()` |
| `child<T>()` | `GetComponentInChildren<T>()` |
| `parent<T>()` | `GetComponentInParent<T>()` |
| `input.axis(s)` | `Input.GetAxis(s)` |
| `input.getKey(k)` | `Input.GetKey(k)` |
| `input.getKeyDown(k)` | `Input.GetKeyDown(k)` |
| `input.getKeyUp(k)` | `Input.GetKeyUp(k)` |
| `input.getMouseButton(n)` | `Input.GetMouseButton(n)` |
| `log(msg)` | `Debug.Log(msg)` |
| `warn(msg)` | `Debug.LogWarning(msg)` |
| `error(msg)` | `Debug.LogError(msg)` |
| `start f()` | `StartCoroutine(f())` |
| `stop f()` | `StopCoroutine(nameof(f))` |
| `stopAll()` | `StopAllCoroutines()` |
| `a?.b` | `(a != null) ? a.b : null` |
| `a ?: b` | `a ?? b` |
| `a!!` | `a ?? throw new NullReferenceException(...)` |

### 12.5 Float Literal Normalization

All floating-point literals shall be emitted with the `f` suffix:

```prsm
val speed = 5.0
```

```csharp
float speed = 5.0f;
```

### 12.6 String Interpolation

String interpolation expressions shall be lowered to C# interpolated strings:

```prsm
val msg = "Player $name has ${health} HP"
```

```csharp
var msg = $"Player {name} has {health} HP";
```

The `$identifier` short form and `${expression}` long form shall both lower to `{expression}` inside a C# `$"..."` string.

### 12.7 Listen Lifetime Lowering (PrSM 2 부터)

For each `listen` statement with a lifetime modifier, the compiler shall:

1. Generate a private handler field (`_prsm_h0`, `_prsm_h1`, ...) of the appropriate delegate type.
2. Emit `AddListener` in the registration phase (Start or Awake, depending on context).
3. Emit `RemoveListener` + null assignment in the appropriate cleanup method:
   - `until disable` -- inject into `OnDisable`.
   - `until destroy` -- inject into `OnDestroy`.
   - `manual` -- no cleanup; `unlisten` emits removal inline.

If the component already declares the target lifecycle block, the compiler shall append cleanup code after the user body. If no such block exists, the compiler shall synthesize the lifecycle method.

### 12.8 Pattern Binding Lowering (PrSM 2 부터)

Pattern bindings in `when` branches shall lower to `switch` on the enum tag, followed by tuple field extraction:

```prsm
when result {
    Result.Ok(val value) => log("$value")
    Result.Err(val msg) => error(msg)
}
```

```csharp
switch (result.Tag) {
    case ResultTag.Ok:
        var value = result.OkPayload.Item1;
        Debug.Log($"{value}");
        break;
    case ResultTag.Err:
        var msg = result.ErrPayload.Item1;
        Debug.LogError(msg);
        break;
}
```

### 12.9 Input System Lowering (PrSM 2 부터)

When a component uses input system sugar, the compiler shall:

1. Emit a private field: `private PlayerInput _prsmInput;`
2. Emit `_prsmInput = GetComponent<PlayerInput>();` in Awake Phase 2.
3. Replace each `input.action("Name").accessor` with `_prsmInput.actions["Name"].Method()`.
4. For the player form `input.player("Map").action("Name").accessor`, compose the key as `"Map/Name"`.

---

## 13 Diagnostics

The compiler shall emit the following diagnostic codes. Each code is stable across compiler versions.

### 13.1 Errors

| Code | Message | Condition |
|------|---------|-----------|
| E000 | `Cannot read source file: {path}` | I/O error during compilation. The source file could not be opened or read. |
| E012 | `Lifecycle block '{name}' is only valid inside a component declaration` | A lifecycle block (`update`, `awake`, etc.) appears outside a `component`. |
| E013 | `'{qualifier}' fields are only valid inside a component declaration` | A `require`, `optional`, `child`, or `parent` field appears outside a `component`. |
| E014 | `Duplicate lifecycle block '{name}'; only one per component is allowed` | A lifecycle block appears more than once in a single component. |
| E020 | `Type mismatch: expected '{expected}', found '{found}'` | An expression produces a type incompatible with the surrounding context. |
| E022 | `Variable '{name}' must have a type annotation or an initializer` | A `val` or `var` declaration has neither a type annotation nor an initializer. |
| E031 | `'{keyword}' can only be used inside a loop` | `break` or `continue` appears outside a `for` or `while` body. |
| E032 | `'wait' can only be used inside a coroutine` | A `wait` statement appears outside a `coroutine` declaration. |
| E040 | `Cannot assign to immutable value '{name}'` | Assignment to a `val` binding after initialization. |
| E041 | `Cannot assign to 'require' field '{name}'` | Assignment to a `require` field, which is resolved once in `Awake()`. |
| E050 | `Enum '{name}' must have at least one entry` | An enum is declared with zero entries. |
| E051 | `Enum entry '{entry}' expects {expected} argument(s), but {found} given` | Argument count mismatch when constructing a payload enum value. |
| E052 | `Duplicate enum entry '{name}'` | Two entries in the same enum share a name. |
| E060 | `Coroutines are only valid inside a component declaration` | A `coroutine` is declared in an `asset` or `class`. |
| E070 | `Input System sugar requires the 'input-system' feature flag` | Input sugar is used without `features = ["input-system"]` in `.prsmproject`. |
| E081 | `Unknown variant '{variant}' for enum '{enum}'` | A `when` branch references an enum variant that does not exist. |
| E082 | `Pattern for '{variant}' expects {expected} binding(s), found {found}` | Binding count in a destructuring pattern does not match the variant's parameter count. |
| E083 | `Listen lifetime modifier is only valid inside a component` | A listen lifetime modifier (`until disable`, `until destroy`, `manual`) appears outside a `component`. |
| E100 | `Syntax error: {details}` | Catch-all for parser errors -- missing identifiers, unmatched braces, misplaced keywords. |

E081, E082, E083 are (PrSM 2 부터).

### 13.2 Warnings

| Code | Message | Condition |
|------|---------|-----------|
| W001 | `Unnecessary '!!' on non-nullable type '{type}'` | `!!` applied to an expression whose type is already non-nullable. |
| W003 | `'when' does not cover all variants of '{enum}'; missing: {variants}` | A `when` statement over an enum is missing variants and has no `else` branch. |
| W005 | `Data class '{name}' has no fields` | A `data class` is declared with an empty parameter list. |

### 13.3 Diagnostic Format

The compiler shall emit diagnostics in the following format:

```
severity[CODE]: message
  --> file_path:line:column
   |
NN |     offending source line
   |     ^^^^^^^^^^^^^^^^^^^^^^
```

---

## 14 Grammar

The normative grammar for PrSM is defined in Extended Backus-Naur Form (EBNF). The complete grammar is maintained as a separate document.

See [Formal Grammar](../grammar.md) for the full EBNF definition covering file structure, declarations, members, statements, expressions, patterns, and terminal tokens.

The grammar document is authoritative for parser behavior. Where prose in this standard and the grammar conflict, the grammar shall take precedence.
