# PrSM Language 4 Specification — Draft

**Status:** Draft v0.1
**Date:** 2026-04-06
**Prerequisite:** Language 3 (PrSM Language Standard)
**Target:** Unity 2022.3+ (IL2CPP / Mono)
**Tool version:** Prism v2.0.0 (Language 4 = Prism 2.0)

---

Language 4 adds 30 features across four categories: syntax extensions, async + performance, pattern sugar, and developer experience. All Language 3 features are retained. This document defines additions and changes only. The full specification will be merged into `docs/en/spec/standard.md` upon Language 4 finalization.

---

# Part I. Syntax Extensions (1-21)

---

## 1. try/catch/finally + throw [stmt.try]

### 1.1 Grammar

```ebnf
TryStmt      = "try" Block { CatchClause } [ FinallyClause ]
CatchClause  = "catch" "(" Identifier ":" TypeRef ")" Block
FinallyClause = "finally" Block
ThrowStmt    = "throw" Expr
TryExpr      = "try" Block CatchClause
```

### 1.2 Semantics

`try` executes a block and routes exceptions to matching `catch` clauses by type. `finally` always executes. `throw` raises an exception; the `new` keyword is omitted (Kotlin-style). `try` may also be used as an expression when it has exactly one `catch` clause.

### 1.3 Example

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

### 1.4 Lowering

```csharp
try
{
    var data = File.ReadAllText(path);
}
catch (FileNotFoundException e)
{
    Debug.LogWarning(e.Message);
}
catch (Exception e)
{
    Debug.LogError(e.Message);
}
finally
{
    Cleanup();
}

throw new ArgumentException("Invalid value");
```

### 1.5 Diagnostics

| Code | Condition |
|------|-----------|
| E100 | `catch` clause with unreachable type (supertype already caught above) |
| E101 | `throw` expression is not an Exception subtype |
| W020 | Empty `catch` block (swallowed exception) |

---

## 2. Lambda / Closures / Function Types [expr.lambda]

### 2.1 Grammar

```ebnf
LambdaExpr    = "{" [ LambdaParams "=>" ] ExprOrBlock "}"
LambdaParams  = Identifier { "," Identifier }
FuncTypeRef   = "(" [ TypeRef { "," TypeRef } ] ")" "=>" TypeRef
TrailingLambda = Expr "{" [ LambdaParams "=>" ] ExprOrBlock "}"
```

### 2.2 Semantics

Lambdas are anonymous functions enclosed in `{ }`. Parameter types are inferred from context. A single-parameter lambda may use the implicit `it` identifier. When the last argument of a call is a lambda, it may be placed outside the parentheses (trailing lambda). Closure capture follows reference semantics identical to C# lambdas.

### 2.3 Example

```prsm
val callback: (Int) => Unit = { x => log(x) }
val add: (Int, Int) => Int = { a, b => a + b }

list.filter { it > 10 }
list.where({ x => x > 10 }).select({ x => x * 2 })

func doWork(callback: (Int) => Unit) { }
func getPredicate(): (Int) => Bool { }
```

### 2.4 Lowering

```csharp
Action<int> callback = (x) => Debug.Log(x);
Func<int, int, int> add = (a, b) => a + b;

list.Where(x => x > 10).Select(x => x * 2);
```

Function type `(A, B) => R` lowers to `Func<A, B, R>`. `() => Unit` lowers to `Action`.

### 2.5 Diagnostics

| Code | Condition |
|------|-----------|
| E102 | Lambda parameter count mismatch with expected function type |
| E103 | Cannot infer lambda parameter types without context |
| E104 | `it` used in lambda with multiple parameters |

---

## 3. static + const [decl.static]

### 3.1 Grammar

```ebnf
StaticMember = "static" ( VarDecl | ValDecl | FuncDecl )
ConstDecl    = "const" Identifier ":" TypeRef "=" Literal
```

### 3.2 Semantics

`static` declares class-level members accessible without an instance. `const` declares compile-time constants; only literal values (numbers, strings, booleans) are permitted.

### 3.3 Example

```prsm
class MathHelper {
    static val PI: Float = 3.14159
    static func lerp(a: Float, b: Float, t: Float): Float = a + (b - a) * t
}

const MAX_HEALTH: Int = 100
const VERSION: String = "1.0.0"

component ScoreManager : MonoBehaviour {
    static var highScore: Int = 0
}
```

### 3.4 Lowering

```csharp
public class MathHelper
{
    public static readonly float PI = 3.14159f;
    public static float lerp(float a, float b, float t) => a + (b - a) * t;
}

public const int MAX_HEALTH = 100;
public const string VERSION = "1.0.0";
```

### 3.5 Diagnostics

| Code | Condition |
|------|-----------|
| E105 | `const` with non-literal initializer |
| E106 | `static` on a lifecycle block (awake, update, etc.) |

---

## 4. Collection Literals [expr.collection]

### 4.1 Grammar

```ebnf
ListLiteral  = "[" [ Expr { "," Expr } ] "]"
MapLiteral   = "{" [ MapEntry { "," MapEntry } ] "}"
MapEntry     = Expr ":" Expr
ArrayOf      = "arrayOf" "(" [ Expr { "," Expr } ] ")"
SetOf        = "setOf" "(" [ Expr { "," Expr } ] ")"
```

### 4.2 Semantics

`[elements]` creates a `List<T>` with element type inferred. `{key: value}` creates a `Map<K, V>` (`Dictionary`). `arrayOf()` creates a `T[]`. `setOf()` creates a `HashSet<T>`. Empty collections require explicit type annotation.

### 4.3 Example

```prsm
val numbers = [1, 2, 3, 4, 5]
val names = ["Alice", "Bob", "Charlie"]
val empty: List<Int> = []

val lookup = {"hp": 100, "mp": 50}
val arr: Array<Int> = arrayOf(1, 2, 3)
val unique = setOf("Alice", "Bob", "Alice")
```

### 4.4 Lowering

```csharp
var numbers = new List<int> { 1, 2, 3, 4, 5 };
var names = new List<string> { "Alice", "Bob", "Charlie" };
var empty = new List<int>();

var lookup = new Dictionary<string, int> { {"hp", 100}, {"mp", 50} };
var arr = new int[] { 1, 2, 3 };
var unique = new HashSet<string> { "Alice", "Bob", "Alice" };
```

### 4.5 Diagnostics

| Code | Condition |
|------|-----------|
| E107 | Empty collection literal without type annotation |
| E108 | Mixed element types in collection literal |

---

## 5. Type Casting [expr.cast]

### 5.1 Grammar

```ebnf
SafeCast   = Expr "as" TypeRef "?"
ForceCast  = Expr "as!" TypeRef
SmartCast  = "is" check in if/when narrows type in subsequent block
ConvMethod = Expr "." ( "toInt" | "toFloat" | "toDouble" | "toString" ) "()"
```

### 5.2 Semantics

`as Type?` performs a safe cast returning `null` on failure. `as! Type` performs a force cast throwing `InvalidCastException` on failure. After an `is` check, the variable is smart-cast to the checked type within the same scope. Numeric conversion methods (`toInt()`, `toFloat()`, etc.) provide explicit widening/narrowing.

### 5.3 Example

```prsm
val enemy = collider as Enemy?
val boss = collider as! Boss

if collider is BoxCollider {
    log(collider.size)   // smart-cast to BoxCollider
}

when target {
    is Enemy => target.takeDamage(10)
    is Ally  => target.heal(5)
}

val f = 42.toFloat()
val s = 100.toString()
```

### 5.4 Lowering

```csharp
var enemy = collider as Enemy;          // safe cast
var boss = (Boss)collider;              // force cast

if (collider is BoxCollider _bc)
{
    Debug.Log(_bc.size);                // pattern variable
}

switch (target)
{
    case Enemy _e: _e.takeDamage(10); break;
    case Ally _a: _a.heal(5); break;
}

var f = (float)42;
var s = 100.ToString();
```

### 5.5 Diagnostics

| Code | Condition |
|------|-----------|
| E109 | `as!` cast to unrelated type (provably always fails) |
| W021 | `as?` result never null-checked |

---

## 6. Property get/set [decl.property]

### 6.1 Grammar

```ebnf
PropertyDecl = ( "val" | "var" ) Identifier ":" TypeRef [ "=" Expr ]
               [ Getter ] [ Setter ]
Getter       = "get" ( "=" Expr | Block )
Setter       = "set" "(" Identifier ")" Block
```

### 6.2 Semantics

Properties may have custom `get` and `set` accessors. `val` with only `get` creates a computed read-only property. Inside a `set` block, the implicit `field` keyword refers to the auto-generated backing field (Kotlin-style).

### 6.3 Example

```prsm
var hp: Int
    get = _hp
    set(value) {
        _hp = Mathf.clamp(value, 0, maxHp)
        onHpChanged?.invoke(_hp)
    }

val isAlive: Bool
    get = hp > 0

var name: String = "Default"
    set(value) {
        field = value.trim()
        log("Name changed to $field")
    }
```

### 6.4 Lowering

```csharp
private int __hp;
public int hp
{
    get => __hp;
    set
    {
        __hp = Mathf.Clamp(value, 0, maxHp);
        onHpChanged?.Invoke(__hp);
    }
}

public bool isAlive => hp > 0;

private string __name = "Default";
public string name
{
    get => __name;
    set
    {
        __name = value.Trim();
        Debug.Log($"Name changed to {__name}");
    }
}
```

### 6.5 Diagnostics

| Code | Condition |
|------|-----------|
| E110 | `set` on a `val` property |
| E111 | `field` used outside a property accessor |

---

## 7. Extension Methods (extend) [decl.extend]

### 7.1 Grammar

```ebnf
ExtendDecl   = "extend" TypeRef "{" { ExtendMember } "}"
ExtendMember = FuncDecl | PropertyDecl
```

### 7.2 Semantics

`extend Type { }` adds methods and computed properties to an existing type without modifying its source. Inside the block, `this` refers to the receiver instance. Lowers to a static class with extension methods.

### 7.3 Example

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

### 7.4 Lowering

```csharp
public static class TransformExtensions
{
    public static void resetLocal(this Transform self)
    {
        self.localPosition = Vector3.zero;
        self.localRotation = Quaternion.identity;
        self.localScale = Vector3.one;
    }
}

public static class Vector3Extensions
{
    public static Vector3 flat(this Vector3 self)
        => new Vector3(self.x, 0, self.z);
}
```

### 7.5 Diagnostics

| Code | Condition |
|------|-----------|
| E112 | Extension member name conflicts with existing member on target type |
| E113 | `extend` block contains state (var field) |

---

## 8. abstract / sealed / open [decl.modifier]

### 8.1 Grammar

```ebnf
ClassModifier = "abstract" | "sealed" | "open"
FuncModifier  = "abstract" | "open" | "override"

ClassDecl     = [ ClassModifier ] "class" Identifier ...
FuncDecl      = [ FuncModifier ] "func" Identifier ...
```

### 8.2 Semantics

By default, classes and methods are final (non-inheritable, non-overridable). `open` permits inheritance/override. `abstract` requires subclass implementation. `sealed` restricts subclasses to the same file, enabling exhaustive `when` checks. `override` marks a method that replaces a parent `open` or `abstract` method.

### 8.3 Example

```prsm
abstract class Weapon {
    abstract func attack()
    open func reload() { }
}

class Sword : Weapon {
    override func attack() { swing() }
}

sealed class Shape {
    class Circle(radius: Float) : Shape
    class Rect(width: Float, height: Float) : Shape
    class Triangle(base: Float, height: Float) : Shape
}

when shape {
    is Shape.Circle   => PI * shape.radius * shape.radius
    is Shape.Rect     => shape.width * shape.height
    is Shape.Triangle => shape.base * shape.height / 2
}
```

### 8.4 Lowering

```csharp
public abstract class Weapon
{
    public abstract void attack();
    public virtual void reload() { }
}

public class Sword : Weapon
{
    public override void attack() { swing(); }
}
```

`sealed class` lowers to a C# abstract class with nested subclasses. The compiler verifies `when` exhaustiveness at compile time.

### 8.5 Diagnostics

| Code | Condition |
|------|-----------|
| E114 | `override` without matching `open`/`abstract` in parent |
| E115 | Non-exhaustive `when` on sealed class (missing subtype) |
| E116 | Instantiation of `abstract` class |

---

## 9. Tuple [expr.tuple]

### 9.1 Grammar

```ebnf
TupleType   = "(" TypeRef "," TypeRef { "," TypeRef } ")"
TupleLit    = "(" Expr "," Expr { "," Expr } ")"
NamedTuple  = "(" Identifier ":" Expr { "," Identifier ":" Expr } ")"
TupleDestr  = "val" "(" Identifier { "," Identifier } ")" "=" Expr
```

### 9.2 Semantics

Tuples group multiple values into a single compound value. Named tuples allow access by label. Tuples support destructuring. Lowers to C# `ValueTuple`.

### 9.3 Example

```prsm
func getResult(): (Int, String) = (42, "answer")
val (num, name) = getResult()

func getStats(): (hp: Int, mp: Int) = (hp: 100, mp: 50)
val stats = getStats()
log(stats.hp)
```

### 9.4 Lowering

```csharp
public (int, string) getResult() => (42, "answer");
var (num, name) = getResult();

public (int hp, int mp) getStats() => (hp: 100, mp: 50);
var stats = getStats();
Debug.Log(stats.hp);
```

### 9.5 Diagnostics

| Code | Condition |
|------|-----------|
| E117 | Tuple destructuring count mismatch |
| E118 | Named tuple field access with wrong label |

---

## 10. use (IDisposable) [stmt.use]

### 10.1 Grammar

```ebnf
UseBlock = "use" Identifier "=" Expr Block
UseDecl  = "use" "val" Identifier "=" Expr
```

### 10.2 Semantics

`use` ensures automatic disposal of `IDisposable` resources. The block form disposes at block exit. The declaration form (`use val`) disposes at the enclosing scope exit. `using` is reserved for namespace imports, so `use` is chosen.

### 10.3 Example

```prsm
use stream = FileStream(path, FileMode.Open) {
    val data = stream.readToEnd()
}

use val conn = DbConnection(connString)
// conn auto-disposed at scope end
```

### 10.4 Lowering

```csharp
using (var stream = new FileStream(path, FileMode.Open))
{
    var data = stream.ReadToEnd();
}

using var conn = new DbConnection(connString);
```

### 10.5 Diagnostics

| Code | Condition |
|------|-----------|
| E119 | `use` on a type that does not implement IDisposable |

---

## 11. struct [decl.struct]

### 11.1 Grammar

```ebnf
StructDecl = "struct" Identifier "(" ParamList ")" [ "{" { Member } "}" ]
```

### 11.2 Semantics

`struct` declares a value type with copy semantics and stack allocation. Similar to `data class` but stored as a C# `struct`. May include static members and methods in an optional body block.

### 11.3 Example

```prsm
struct DamageInfo(amount: Int, type: DamageType)

struct Color32(r: Byte, g: Byte, b: Byte, a: Byte) {
    static val white = Color32(255, 255, 255, 255)
    static val black = Color32(0, 0, 0, 255)
}
```

### 11.4 Lowering

```csharp
public struct DamageInfo
{
    public int amount;
    public DamageType type;
    public DamageInfo(int amount, DamageType type)
    {
        this.amount = amount;
        this.type = type;
    }
}

public struct Color32
{
    public byte r, g, b, a;
    public static readonly Color32 white = new Color32(255, 255, 255, 255);
    public static readonly Color32 black = new Color32(0, 0, 0, 255);
    // constructor generated
}
```

### 11.5 Diagnostics

| Code | Condition |
|------|-----------|
| E120 | `struct` inheriting from a class (value types cannot inherit) |
| W022 | Large struct (> 16 bytes) may cause performance issues |

---

## 12. delegate / event [decl.event]

### 12.1 Grammar

```ebnf
EventDecl    = "event" Identifier ":" FuncTypeRef
EventSubscribe = Identifier "+=" LambdaExpr
EventUnsub     = Identifier "-=" Expr
EventInvoke    = Identifier "." "invoke" "(" [ ArgList ] ")"
```

### 12.2 Semantics

Function types `(A, B) => Unit` may be used directly as callback variables (single-cast). The `event` keyword declares a multicast delegate that supports `+=`/`-=` subscription and `.invoke()` dispatch.

### 12.3 Example

```prsm
// Single-cast callback (function type)
var onDamaged: ((Int, DamageType) => Unit)? = null
onDamaged?.invoke(amount, type)

// Multicast event
event onHealthChanged: (Int) => Unit

onHealthChanged += { hp => log("HP: $hp") }
onHealthChanged.invoke(currentHp)
```

### 12.4 Lowering

```csharp
// Single-cast
public Action<int, DamageType> onDamaged;
onDamaged?.Invoke(amount, type);

// Multicast
public event Action<int> onHealthChanged;
onHealthChanged += (hp) => Debug.Log($"HP: {hp}");
onHealthChanged?.Invoke(currentHp);
```

### 12.5 Diagnostics

| Code | Condition |
|------|-----------|
| E121 | `event` invoked from outside the declaring type |
| E122 | `event` with non-Unit return type |

---

## 13. Operator Overloading [decl.operator]

### 13.1 Grammar

```ebnf
OperatorDecl = "operator" OperatorName "(" ParamList ")" ":" TypeRef ( "=" Expr | Block )
OperatorName = "plus" | "minus" | "times" | "div" | "mod"
             | "compareTo" | "equals" | "unaryMinus" | "not"
```

### 13.2 Semantics

Operator functions map symbolic operators to named methods. `plus` maps to `+`, `minus` to `-`, `times` to `*`, `div` to `/`, `mod` to `%`, `compareTo` to `<`/`>`/`<=`/`>=`, `equals` to `==`/`!=`. Follows Kotlin conventions.

### 13.3 Example

```prsm
data class Vec2i(x: Int, y: Int) {
    operator plus(other: Vec2i): Vec2i = Vec2i(x + other.x, y + other.y)
    operator minus(other: Vec2i): Vec2i = Vec2i(x - other.x, y - other.y)
    operator times(scalar: Int): Vec2i = Vec2i(x * scalar, y * scalar)
}

val c = Vec2i(1, 2) + Vec2i(3, 4)  // Vec2i(4, 6)
```

### 13.4 Lowering

```csharp
public static Vec2i operator +(Vec2i self, Vec2i other)
    => new Vec2i(self.x + other.x, self.y + other.y);
public static Vec2i operator -(Vec2i self, Vec2i other)
    => new Vec2i(self.x - other.x, self.y - other.y);
public static Vec2i operator *(Vec2i self, int scalar)
    => new Vec2i(self.x * scalar, self.y * scalar);
```

### 13.5 Diagnostics

| Code | Condition |
|------|-----------|
| E123 | Unknown operator name in `operator` declaration |
| E124 | `operator equals` without matching `GetHashCode` override |

---

## 14. Indexer [decl.indexer]

### 14.1 Grammar

```ebnf
IndexerGet = "operator" "get" "(" ParamList ")" ":" TypeRef ( "=" Expr | Block )
IndexerSet = "operator" "set" "(" ParamList "," Identifier ":" TypeRef ")" Block
```

### 14.2 Semantics

`operator get` and `operator set` define indexer access using `[]` syntax. May have any parameter type (not limited to `Int`).

### 14.3 Example

```prsm
class Inventory {
    var items: List<Item> = []

    operator get(index: Int): Item = items[index]
    operator set(index: Int, value: Item) { items[index] = value }
}

val sword = inventory[0]
inventory[1] = shield
```

### 14.4 Lowering

```csharp
public Item this[int index]
{
    get => items[index];
    set => items[index] = value;
}
```

### 14.5 Diagnostics

| Code | Condition |
|------|-----------|
| E125 | `operator get` without matching `operator set` on a `var` indexer |

---

## 15. typealias [decl.typealias]

### 15.1 Grammar

```ebnf
TypeAlias = "typealias" Identifier "=" TypeRef
```

### 15.2 Semantics

`typealias` introduces a compile-time alias for an existing type. The alias is replaced with the original type during lowering. No runtime representation.

### 15.3 Example

```prsm
typealias Position = Vector3
typealias EnemyList = List<Enemy>

val pos: Position = vec3(1, 2, 3)
val enemies: EnemyList = []
```

### 15.4 Lowering

```csharp
// typealias erased at compile time
Vector3 pos = new Vector3(1, 2, 3);
List<Enemy> enemies = new List<Enemy>();
```

### 15.5 Diagnostics

| Code | Condition |
|------|-----------|
| E126 | `typealias` creating a circular reference |
| E127 | `typealias` shadowing a built-in type name |

---

## 16. Raw String Literal [expr.rawstring]

### 16.1 Grammar

```ebnf
RawString = '"""' { any character } '"""'
```

### 16.2 Semantics

Triple-quoted strings preserve newlines and special characters without escaping. Leading whitespace is trimmed to the closing delimiter's indentation level. Interpolation (`$var`, `${expr}`) is still active inside raw strings.

### 16.3 Example

```prsm
val json = """
    {
        "name": "Player",
        "level": 42
    }
    """

val query = """
    SELECT * FROM users
    WHERE name = '${userName}'
    """
```

### 16.4 Lowering

```csharp
// C# 11 raw string literal (Unity 2022.3+)
var json = """
    {
        "name": "Player",
        "level": 42
    }
    """;

// Fallback for older C# versions: @"" verbatim string
```

### 16.5 Diagnostics

| Code | Condition |
|------|-----------|
| E128 | Unterminated raw string literal |

---

## 17. in Operator [expr.in]

### 17.1 Grammar

```ebnf
InExpr = Expr "in" ( RangeExpr | Expr )
```

### 17.2 Semantics

`in` tests membership. For ranges (`a..b`), it checks `x >= a && x <= b`. For lists, it calls `Contains()`. For maps, it calls `ContainsKey()`.

### 17.3 Example

```prsm
if x in 1..10 { log("In range") }
if name in ["Alice", "Bob"] { log("Known user") }
if key in lookup { log("Key exists") }
```

### 17.4 Lowering

```csharp
if (x >= 1 && x <= 10) { Debug.Log("In range"); }
if (new List<string>{"Alice", "Bob"}.Contains(name)) { Debug.Log("Known user"); }
if (lookup.ContainsKey(key)) { Debug.Log("Key exists"); }
```

### 17.5 Diagnostics

| Code | Condition |
|------|-----------|
| E129 | `in` used with a type that has no `Contains` or `ContainsKey` method |

---

## 18. OR Pattern [pattern.or]

### 18.1 Grammar

```ebnf
OrPattern = Pattern "," Pattern { "," Pattern }
```

### 18.2 Semantics

Multiple patterns separated by commas in a `when` arm match if any individual pattern matches. All patterns in an OR group must bind the same set of variables (or none).

### 18.3 Example

```prsm
when direction {
    Direction.Up, Direction.Down       => handleVertical()
    Direction.Left, Direction.Right    => handleHorizontal()
}
```

### 18.4 Lowering

```csharp
switch (direction)
{
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

### 18.5 Diagnostics

| Code | Condition |
|------|-----------|
| E130 | OR pattern arms bind different variables |

---

## 19. Range Pattern [pattern.range]

### 19.1 Grammar

```ebnf
RangePattern = "in" Expr ".." Expr
```

### 19.2 Semantics

`in low..high` inside a `when` arm matches values in the inclusive range `[low, high]`. Only integral and floating-point types are supported.

### 19.3 Example

```prsm
when score {
    in 90..100 => "A"
    in 80..89  => "B"
    in 70..79  => "C"
    else       => "F"
}
```

### 19.4 Lowering

```csharp
switch (score)
{
    case int _s when _s >= 90 && _s <= 100: return "A";
    case int _s when _s >= 80 && _s <= 89:  return "B";
    case int _s when _s >= 70 && _s <= 79:  return "C";
    default: return "F";
}
```

### 19.5 Diagnostics

| Code | Condition |
|------|-----------|
| E131 | Range pattern with low > high |
| W023 | Overlapping range patterns in when |

---

## 20. Null Coalescing Assignment (?:=) [expr.nullcoalesce]

### 20.1 Grammar

```ebnf
NullCoalesceAssign = Expr "?:=" Expr
```

### 20.2 Semantics

`a ?:= b` assigns `b` to `a` only if `a` is currently `null`. The left-hand side must be a nullable mutable variable.

### 20.3 Example

```prsm
var _instance: GameManager? = null

func getInstance(): GameManager {
    _instance ?:= FindFirstObjectByType<GameManager>()
    return _instance!!
}
```

### 20.4 Lowering

```csharp
_instance ??= FindFirstObjectByType<GameManager>();
return _instance;
```

### 20.5 Diagnostics

| Code | Condition |
|------|-----------|
| E132 | `?:=` on a non-nullable variable |
| E133 | `?:=` on a `val` (immutable) |

---

## 21. Default Interface Implementation [decl.interface.default]

### 21.1 Grammar

```ebnf
InterfaceFunc = "func" Identifier "(" [ ParamList ] ")" [ ":" TypeRef ] [ Block ]
```

(Extends Language 3 InterfaceFunc to allow an optional Block.)

### 21.2 Semantics

Interface methods may include a default body. Implementors may omit the method to use the default. Requires C# 8+ default interface methods. If the target runtime does not support DIM, the compiler emits a diagnostic.

### 21.3 Example

```prsm
interface IMovable {
    val speed: Float
    func move(direction: Vector3) {
        transform.translate(direction * speed * Time.deltaTime)
    }
}

component Player : MonoBehaviour, IMovable {
    val speed: Float = 5.0
    // move() uses default implementation
}
```

### 21.4 Lowering

```csharp
public interface IMovable
{
    float speed { get; }
    void move(Vector3 direction)
    {
        transform.Translate(direction * speed * Time.deltaTime);
    }
}
```

### 21.5 Diagnostics

| Code | Condition |
|------|-----------|
| E134 | Default interface method uses feature unavailable in target C# version |
| W024 | Default implementation references instance state that may not be accessible |

---

# Part II. Async + Performance (22-24)

---

## 22. async/await [stmt.async]

### 22.1 Grammar

```ebnf
AsyncFunc  = "async" "func" Identifier "(" [ ParamList ] ")" ":" TypeRef Block
AwaitExpr  = "await" Expr
```

### 22.2 Semantics

`async func` declares an asynchronous function that returns a `Task<T>` (or `UniTask<T>` when UniTask is detected). `await` suspends execution until the awaited task completes. Inside Unity lifecycle contexts, the compiler prefers UniTask if available, falling back to `Task`.

### 22.3 Example

```prsm
async func loadData(url: String): String {
    val response = await Http.get(url)
    return response.body
}

async func initialize() {
    val config = await loadData("/api/config")
    val assets = await loadData("/api/assets")
    setup(config, assets)
}
```

### 22.4 Lowering

```csharp
// With UniTask (preferred in Unity)
public async UniTask<string> loadData(string url)
{
    var response = await Http.Get(url);
    return response.body;
}

// Fallback to Task
public async Task<string> loadData(string url)
{
    var response = await Http.Get(url);
    return response.body;
}
```

### 22.5 Diagnostics

| Code | Condition |
|------|-----------|
| E135 | `await` outside of `async func` |
| E136 | `async func` with non-awaitable return type |
| W025 | `async func` that never uses `await` |

---

## 23. Optimizer Enhancement [opt.v4]

### 23.1 Overview

Extends the Language 3 optimizer (Section 3) with additional passes targeting Unity-specific performance patterns. Activated by the same `--optimize` flag.

### 23.2 New Rules

#### 23.2.1 String Interpolation Caching [opt.string]

Repeated identical `$"..."` expressions in hot paths (update, fixedUpdate) are cached into a field.

Before:
```csharp
void Update() {
    label.text = $"HP: {hp}";  // allocates every frame
}
```

After:
```csharp
private int __prev_hp;
private string __cached_label;
void Update() {
    if (__prev_hp != hp) {
        __prev_hp = hp;
        __cached_label = $"HP: {hp}";
    }
    label.text = __cached_label;
}
```

#### 23.2.2 LINQ Elimination [opt.linq]

Simple LINQ chains on `List<T>` in hot paths are replaced with `for` loops to avoid allocations.

Before:
```csharp
var alive = enemies.Where(e => e.IsAlive).ToList();
```

After:
```csharp
var alive = new List<Enemy>();
for (int i = 0; i < enemies.Count; i++)
{
    if (enemies[i].IsAlive) alive.Add(enemies[i]);
}
```

#### 23.2.3 Struct Copy Reduction [opt.structcopy]

Avoids unnecessary struct copies by using `ref` locals when the struct is only read.

### 23.3 Diagnostics

| Code | Condition |
|------|-----------|
| W026 | String allocation in hot path (optimizable) |
| W027 | LINQ allocation in Update/FixedUpdate |

---

## 24. Burst Compatibility Analysis [analysis.burst]

### 24.1 Overview

Static analysis pass that checks whether a method or struct is compatible with Unity Burst compiler constraints. Reports diagnostics for patterns that prevent Burst compilation.

### 24.2 Checked Constraints

- No managed type references (class, string, delegate) in Burst-annotated methods
- No `try`/`catch` in Burst context
- No virtual calls or interface dispatch
- No boxing of value types
- All called methods must also be Burst-compatible

### 24.3 Usage

```prsm
@burst
func calculateForces(positions: NativeArray<Float3>, forces: NativeArray<Float3>) {
    for i in 0..positions.length {
        forces[i] = computeGravity(positions[i])
    }
}
```

### 24.4 Lowering

```csharp
[BurstCompile]
public void calculateForces(NativeArray<float3> positions, NativeArray<float3> forces)
{
    for (int i = 0; i < positions.Length; i++)
    {
        forces[i] = computeGravity(positions[i]);
    }
}
```

### 24.5 Diagnostics

| Code | Condition |
|------|-----------|
| E137 | Managed type reference in `@burst` method |
| E138 | `try`/`catch` in `@burst` method |
| E139 | Virtual/interface call in `@burst` context |
| W028 | Boxing detected in `@burst` method |

---

# Part III. Pattern Sugar (25-27)

---

## 25. state machine [sugar.state]

### 25.1 Grammar

```ebnf
StateMachine = "state" "machine" Identifier "{" { StateDecl } "}"
StateDecl    = "state" Identifier [ Block ]
             | "on" EventRef "=>" Identifier
             | "enter" Block
             | "exit" Block
```

### 25.2 Semantics

`state machine` declares a finite state machine with named states, transitions via `on event => targetState`, and optional `enter`/`exit` lifecycle hooks. The compiler generates an enum for states, a current-state field, and a transition dispatch method. Exhaustiveness: every declared event must be handled in every state, or a default transition must exist.

### 25.3 Example

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

### 25.4 Lowering

```csharp
private enum AiState { Idle, Chase, Attack }
private AiState _aiState = AiState.Idle;

private void _enterAiState(AiState state)
{
    switch (state)
    {
        case AiState.Idle: playAnimation("idle"); break;
        case AiState.Chase: playAnimation("run"); break;
        case AiState.Attack: playAnimation("attack"); break;
    }
}

private void _exitAiState(AiState state)
{
    switch (state)
    {
        case AiState.Attack: resetCooldown(); break;
    }
}

public void TransitionAiState(string eventName)
{
    var prev = _aiState;
    _aiState = (_aiState, eventName) switch
    {
        (AiState.Idle, "playerDetected") => AiState.Chase,
        (AiState.Chase, "playerLost") => AiState.Idle,
        (AiState.Chase, "inAttackRange") => AiState.Attack,
        (AiState.Attack, "attackDone") => AiState.Chase,
        _ => _aiState,
    };
    if (prev != _aiState) { _exitAiState(prev); _enterAiState(_aiState); }
}
```

### 25.5 Diagnostics

| Code | Condition |
|------|-----------|
| E140 | Transition to undeclared state |
| E141 | Duplicate state name in state machine |
| W029 | State with no outgoing transitions (dead state) |

---

## 26. command [sugar.command]

### 26.1 Grammar

```ebnf
CommandDecl = "command" Identifier "(" [ ParamList ] ")" Block
             [ "undo" Block ]
             [ "canExecute" "=" Expr ]
```

### 26.2 Semantics

`command` declares an executable action with optional undo support and a guard condition. Used for UI binding, input handling, and action replay. The compiler generates an `ICommand` implementation with `Execute()`, `CanExecute()`, and optionally `Undo()`.

### 26.3 Example

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

### 26.4 Lowering

```csharp
public class MoveUnitCommand : ICommand
{
    private UnitController _owner;
    private Vector3 _target;
    private Vector3 _prevPos;

    public MoveUnitCommand(UnitController owner, Vector3 target)
    {
        _owner = owner;
        _target = target;
    }

    public bool CanExecute() => _owner.isAlive && !_owner.isStunned;

    public void Execute()
    {
        _prevPos = _owner.transform.position;
        _owner.transform.position = _target;
    }

    public void Undo()
    {
        _owner.transform.position = _prevPos;
    }
}
```

### 26.5 Diagnostics

| Code | Condition |
|------|-----------|
| E142 | `undo` block references variable not captured during `execute` |
| W030 | `command` without `canExecute` guard |

---

## 27. bind (MVVM) [sugar.bind]

### 27.1 Grammar

```ebnf
BindDecl = "bind" Identifier ":" TypeRef "=" Expr
         | "bind" Identifier "to" Expr
```

### 27.2 Semantics

`bind` declares a reactive property that automatically notifies subscribers when its value changes. In UI contexts, `bind prop to uiElement` creates a two-way data binding. The compiler generates `INotifyPropertyChanged` plumbing, change-detection fields, and UI update subscriptions.

### 27.3 Example

```prsm
component PlayerHUD : MonoBehaviour {
    bind hp: Int = 100
    bind playerName: String = "Hero"

    serialize hpLabel: TextMeshProUGUI
    serialize nameLabel: TextMeshProUGUI

    awake {
        bind hp to hpLabel.text
        bind playerName to nameLabel.text
    }
}
```

### 27.4 Lowering

```csharp
private int _hp = 100;
public int hp
{
    get => _hp;
    set
    {
        if (_hp != value)
        {
            _hp = value;
            OnPropertyChanged(nameof(hp));
            // bound UI update
            if (_hpLabel != null) _hpLabel.text = _hp.ToString();
        }
    }
}

// INotifyPropertyChanged implementation generated
public event PropertyChangedEventHandler PropertyChanged;
private void OnPropertyChanged(string name)
    => PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(name));
```

### 27.5 Diagnostics

| Code | Condition |
|------|-----------|
| E143 | `bind to` target is not a writable property |
| E144 | `bind` type mismatch between source and target |
| W031 | `bind` property never read (unnecessary binding overhead) |

---

# Part IV. Developer Experience (28-30)

---

## 28. Error Message Enhancement [dx.errors]

### 28.1 Overview

Improves compiler diagnostics with contextual suggestions, fix hints, and common-mistake detection. Not a language syntax feature; this defines standards for diagnostic output formatting.

### 28.2 Enhanced Diagnostic Format

```
error[E090]: Interface member 'takeDamage' not implemented
  --> src/Enemy.prsm:15:1
   |
15 | component Enemy : MonoBehaviour, IDamageable {
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = help: Add the missing method:
   |   func takeDamage(amount: Int) {
   |       // TODO: implement
   |   }
   = note: Required by interface 'IDamageable' declared in src/IDamageable.prsm:3
```

### 28.3 Enhancement Categories

#### 28.3.1 Did-You-Mean Suggestions

Levenshtein distance matching for misspelled identifiers:

```
error[E050]: Unresolved reference 'trasform'
  --> src/Player.prsm:10:5
   |
   = help: Did you mean 'transform'?
```

#### 28.3.2 Type Mismatch Explanation

```
error[E030]: Type mismatch: expected 'Float', found 'Int'
  --> src/Player.prsm:8:20
   |
 8 |     val speed: Float = 10
   |                         ^^
   |
   = help: Use '10.toFloat()' or change the type to 'Int'
```

#### 28.3.3 Common Mistake Detection

```
error[E145]: Assignment in condition (use '==' for comparison)
  --> src/Game.prsm:12:8
   |
12 |     if hp = 0 { die() }
   |        ^^^^^^
   |
   = help: Replace '=' with '==' for comparison
```

### 28.4 Diagnostics

| Code | Condition |
|------|-----------|
| E145 | Assignment used as condition expression |
| E146 | Common API misuse detected (with suggested fix) |

---

## 29. Refactoring Tools [dx.refactor]

### 29.1 Overview

IDE-integrated refactoring actions exposed through the compiler's Language Server Protocol (LSP) implementation. These are code actions, not language syntax changes.

### 29.2 Supported Refactorings

#### 29.2.1 Extract Method

Extracts selected statements into a new method, computing parameters and return values automatically.

```prsm
// Before (selected region)
val dx = target.x - origin.x
val dz = target.z - origin.z
val dist = sqrt(dx * dx + dz * dz)

// After
func computeDistance(origin: Vector3, target: Vector3): Float {
    val dx = target.x - origin.x
    val dz = target.z - origin.z
    return sqrt(dx * dx + dz * dz)
}
val dist = computeDistance(origin, target)
```

#### 29.2.2 Extract Component

Moves selected fields and methods into a new component file, replacing them with a `require` reference.

#### 29.2.3 Inline Variable

Replaces a single-use `val` with its initializer expression at the usage site.

#### 29.2.4 Rename Symbol

Renames a declaration and all references across the project.

#### 29.2.5 Convert to State Machine

Detects `enum` + `switch`-style state patterns and offers conversion to the `state machine` sugar (Feature 25).

### 29.3 Diagnostics

No new error codes. Refactoring actions produce preview diffs for user confirmation.

---

## 30. Debugger Integration [dx.debugger]

### 30.1 Overview

Provides source-level debugging for PrSM files by generating source maps from `.prsm` to the generated `.cs` files, allowing breakpoints, step-through, and variable inspection in PrSM source.

### 30.2 Source Map Format

The compiler generates `.prsm.map` files alongside generated C# output:

```json
{
    "version": 1,
    "source": "src/Player.prsm",
    "generated": "Generated/Player.cs",
    "mappings": [
        { "prsmLine": 5, "csLine": 12 },
        { "prsmLine": 6, "csLine": 13 },
        { "prsmLine": 10, "csLine": 20 }
    ]
}
```

### 30.3 Features

#### 30.3.1 Breakpoint Mapping

IDE sets breakpoints on `.prsm` lines; the debugger adapter translates to the corresponding `.cs` line.

#### 30.3.2 Variable Display

The debugger adapter maps C# generated variable names (e.g., `_prsm_d`, `__hp`) back to their PrSM identifiers for display.

#### 30.3.3 Step Filtering

The debugger skips compiler-generated boilerplate (singleton setup, pool initialization, state machine dispatch) and steps through user-written PrSM logic only.

### 30.4 Compiler Flag

```bash
prism build --debug-maps
```

```toml
[compiler]
debug_maps = true   # default: true in Debug, false in Release
```

### 30.5 Diagnostics

| Code | Condition |
|------|-----------|
| W032 | Source map generation failed for a file (non-fatal) |

---

# Error Codes Summary

| Code | Severity | Feature | Message |
|------|----------|---------|---------|
| E100 | Error | try/catch | Unreachable catch clause (supertype caught above) |
| E101 | Error | throw | Expression is not an Exception subtype |
| E102 | Error | Lambda | Lambda parameter count mismatch |
| E103 | Error | Lambda | Cannot infer lambda parameter types |
| E104 | Error | Lambda | `it` used in multi-parameter lambda |
| E105 | Error | const | Non-literal initializer for `const` |
| E106 | Error | static | `static` on lifecycle block |
| E107 | Error | Collection | Empty collection without type annotation |
| E108 | Error | Collection | Mixed element types in literal |
| E109 | Error | Cast | `as!` to provably unrelated type |
| E110 | Error | Property | `set` on a `val` property |
| E111 | Error | Property | `field` outside property accessor |
| E112 | Error | extend | Extension conflicts with existing member |
| E113 | Error | extend | Extension block contains state |
| E114 | Error | Modifier | `override` without matching parent method |
| E115 | Error | sealed | Non-exhaustive `when` on sealed class |
| E116 | Error | abstract | Instantiation of abstract class |
| E117 | Error | Tuple | Destructuring count mismatch |
| E118 | Error | Tuple | Named tuple field access with wrong label |
| E119 | Error | use | Type does not implement IDisposable |
| E120 | Error | struct | struct inheriting from a class |
| E121 | Error | event | Event invoked from outside declaring type |
| E122 | Error | event | Event with non-Unit return type |
| E123 | Error | Operator | Unknown operator name |
| E124 | Error | Operator | `operator equals` without GetHashCode |
| E125 | Error | Indexer | `operator get` without `operator set` on mutable indexer |
| E126 | Error | typealias | Circular type alias |
| E127 | Error | typealias | Alias shadows built-in type |
| E128 | Error | Raw String | Unterminated raw string literal |
| E129 | Error | in | `in` on type without Contains/ContainsKey |
| E130 | Error | OR Pattern | OR pattern arms bind different variables |
| E131 | Error | Range | Range low > high |
| E132 | Error | ?:= | Null coalescing assign on non-nullable |
| E133 | Error | ?:= | Null coalescing assign on immutable `val` |
| E134 | Error | Interface | Default method unsupported in target C# version |
| E135 | Error | async | `await` outside `async func` |
| E136 | Error | async | Non-awaitable return type |
| E137 | Error | Burst | Managed type in `@burst` method |
| E138 | Error | Burst | try/catch in `@burst` method |
| E139 | Error | Burst | Virtual/interface call in `@burst` context |
| E140 | Error | State | Transition to undeclared state |
| E141 | Error | State | Duplicate state name |
| E142 | Error | Command | Undo references uncaptured variable |
| E143 | Error | bind | Bind target not writable |
| E144 | Error | bind | Bind type mismatch |
| E145 | Error | DX | Assignment in condition |
| E146 | Error | DX | Common API misuse |
| W020 | Warning | try/catch | Empty catch block |
| W021 | Warning | Cast | `as?` result never null-checked |
| W022 | Warning | struct | Large struct (> 16 bytes) |
| W023 | Warning | Range | Overlapping range patterns |
| W024 | Warning | Interface | Default method references instance state |
| W025 | Warning | async | `async func` never awaits |
| W026 | Warning | Optimizer | String allocation in hot path |
| W027 | Warning | Optimizer | LINQ in Update/FixedUpdate |
| W028 | Warning | Burst | Boxing in `@burst` method |
| W029 | Warning | State | Dead state (no outgoing transitions) |
| W030 | Warning | Command | Command without canExecute guard |
| W031 | Warning | bind | Bind property never read |
| W032 | Warning | Debugger | Source map generation failed |

---

# Implementation Order

Spec finalization proceeds in this order; compiler implementation follows the same sequence:

**Phase 1 — Core Syntax (Sprint 1-2)**
1. try/catch/finally + throw (Feature 1)
2. Lambda / Closures / Function Types (Feature 2)
3. static + const (Feature 3)
4. Collection Literals (Feature 4)
5. Type Casting (Feature 5)
6. Property get/set (Feature 6)

**Phase 2 — Type System (Sprint 3-4)**
7. Extension Methods (Feature 7)
8. abstract / sealed / open (Feature 8)
9. Tuple (Feature 9)
10. use (IDisposable) (Feature 10)
11. struct (Feature 11)
12. delegate / event (Feature 12)

**Phase 3 — Operators + Sugar (Sprint 5-6)**
13. Operator Overloading (Feature 13)
14. Indexer (Feature 14)
15. typealias (Feature 15)
16. Raw String (Feature 16)
17. in Operator (Feature 17)
18. OR Pattern (Feature 18)
19. Range Pattern (Feature 19)
20. Null Coalescing Assignment (Feature 20)
21. Default Interface Implementation (Feature 21)

**Phase 4 — Async + Performance (Sprint 7-8)**
22. async/await (Feature 22)
23. Optimizer Enhancement (Feature 23)
24. Burst Compatibility Analysis (Feature 24)

**Phase 5 — Pattern Sugar (Sprint 9-10)**
25. state machine (Feature 25)
26. command (Feature 26)
27. bind / MVVM (Feature 27)

**Phase 6 — Developer Experience (Sprint 11-12)**
28. Error Message Enhancement (Feature 28)
29. Refactoring Tools (Feature 29)
30. Debugger Integration (Feature 30)

---

# Feature Gate Extension

Language 4 features are controlled by:

```toml
[language]
version = "4"
features = [
    "try-catch", "lambda", "static-const", "collection-literals",
    "type-cast", "property", "extend", "class-modifiers",
    "tuple", "use", "struct", "event", "operator", "indexer",
    "typealias", "raw-string", "in-operator", "or-pattern",
    "range-pattern", "null-coalesce-assign", "default-interface",
    "async-await", "optimizer-v4", "burst-analysis",
    "state-machine", "command", "bind",
    "error-enhancement", "refactor-tools", "debugger"
]
```

`version = "4"` implicitly enables all Language 4 features. Individual flags allow selective adoption from Language 3:

```toml
[language]
version = "3"
features = ["try-catch", "lambda", "static-const"]
```

---

# Release Criteria

- All 30 features implemented and tested
- Golden tests exist for each syntax feature (Features 1-21)
- Async/performance features (22-24) validated against Unity 2022.3 IL2CPP builds
- Pattern sugar (25-27) includes end-to-end integration tests with Unity components
- Error message enhancements (28) pass UX review
- Refactoring tools (29) integrated into VS Code extension
- Debugger source maps (30) verified with Visual Studio and Rider
- `docs/en/spec/standard.md` updated to Language 4
- `docs/en/spec/changes-lang-4.md` written
- `prism version` outputs 2.0.0
- MSI / winget / Marketplace release successful
