# Expressions

## Arithmetic Operators

`+`, `-`, `*`, `/`, `%` — C#과 동일.

## Comparison / Logical Operators

`==`, `!=`, `<`, `>`, `<=`, `>=`, `&&`, `||`, `!` — C#과 동일.

## Assignment Operators

`=`, `+=`, `-=`, `*=`, `/=`, `%=`

Null coalesce assign: `x ?:= default` -> `x = x ?? default`

## Safe Call (?.)

```prsm
animator?.play("Run")
val speed = rb?.velocity?.magnitude
```
```csharp
if (_animator != null) _animator.Play("Run");
var speed = _rb?.velocity.magnitude;  // 또는 null check chain
```

## Elvis Operator (?:)

```prsm
val name = playerName ?: "Unknown"
val target = findTarget() ?: this.transform
```
```csharp
var name = _playerName ?? "Unknown";
var target = FindTarget() ?? this.transform;
```

## Non-null Assert (!!)

```prsm
val rb = getComponent<Rigidbody>()!!
```
```csharp
var rb = GetComponent<Rigidbody>() ?? throw new System.NullReferenceException("...");
```

## Cast Operators

### Safe cast (as Type?)

```prsm
val enemy = obj as Enemy?
if enemy != null { enemy.attack() }
```
```csharp
var enemy = obj as Enemy;
if (enemy != null) { enemy.Attack(); }
```

### Force cast (as! Type)

```prsm
val boss = obj as! Boss
```
```csharp
var boss = ((Boss)obj);
```

## Is Operator

```prsm
if obj is Enemy {
    print("found enemy")
}
```
```csharp
if (obj is Enemy) {
    Debug.Log("found enemy");
}
```

## String Interpolation

```prsm
val msg = "Player $name has $hp HP"
val detail = "Score: ${score * multiplier}"
```
```csharp
var msg = $"Player {_name} has {_hp} HP";
var detail = $"Score: {_score * _multiplier}";
```

`$variable` (단순 변수) 또는 `${expression}` (식).

## Raw String

```prsm
val json = """
{
    "name": "Player",
    "level": 42
}
"""
```
```csharp
var json = @"
{
    ""name"": ""Player"",
    ""level"": 42
}
";
```

## Lambda

### Expression body

```prsm
val doubled = list.select { x => x * 2 }
val filtered = enemies.where { e => e.isAlive }
```

### Block body

```prsm
list.forEach { item =>
    print(item)
    process(item)
}
```

### Trailing lambda

`{ }` 블록을 메서드 호출 뒤에 바로 붙임:

```prsm
list.filter { it > 10 }
enemies.sortBy { it.hp }
```

## Tuple

```prsm
val pair = (42, "hello")
val named = (hp: 100, mp: 50)
val (x, y) = getPosition()  // destructure
```
```csharp
var pair = (42, "hello");
var named = (hp: 100, mp: 50);
var (x, y) = GetPosition();
```

## Collection Literals

```prsm
val nums = [1, 2, 3, 4, 5]
val config = {"width": 800, "height": 600}
```
```csharp
var nums = new System.Collections.Generic.List<int> { 1, 2, 3, 4, 5 };
var config = new System.Collections.Generic.Dictionary<string, int> { {"width", 800}, {"height", 600} };
```

## Unity Convenience Functions

### vec2 / vec3 / vec4

```prsm
val v = vec3(1, 2, 3)
val uv = vec2(0.5, 0.5)
```
```csharp
var v = new Vector3(1, 2, 3);
var uv = new Vector2(0.5f, 0.5f);
```

### print

```prsm
print("message")
print(variable)
```
```csharp
Debug.Log("message");
Debug.Log(variable);
```

### input

Legacy Input:

```prsm
val h = input.axis("Horizontal")
```
```csharp
var h = Input.GetAxis("Horizontal");
```

New Input System (requires `features = ["input-system"]`):

```prsm
val jumped = input.action("Jump").pressed
val move = input.action("Move").vector2
val aim = input.map("Gameplay").action("Aim").scalar
```
```csharp
var jumped = InputSystem.actions.FindAction("Jump").WasPressedThisFrame();
var move = InputSystem.actions.FindAction("Move").ReadValue<UnityEngine.Vector2>();
var aim = InputSystem.actions.FindAction("Gameplay/Aim").ReadValue<float>();
```

### get<T>() / find<T>()

```prsm
val rb = get<Rigidbody>()
val player = find<PlayerController>()
```
```csharp
var rb = GetComponent<Rigidbody>();
var player = FindFirstObjectByType<PlayerController>();
```

## Conversion Methods

```prsm
val f = 42.toFloat()       // (float)42
val s = 100.toString()      // 100.ToString()
val i = "42".toInt()        // int.Parse("42")
```

## nameof

```prsm
val fieldName = nameof(hp)        // "hp"
val className = nameof(Player)    // "Player"
```
```csharp
var fieldName = nameof(_hp);      // "hp" (또는 nameof(hp))
var className = nameof(Player);
```

## with Expression

불변 데이터의 복사 + 변경:

```prsm
val updated = player with { hp = 100, mp = 50 }
```
```csharp
var updated = player with { hp = 100, mp = 50 };
```

## stackalloc

스택 할당:

```prsm
val buffer = stackalloc[Byte](256)
```
```csharp
Span<byte> buffer = stackalloc byte[256];
```

## Index Access

```prsm
val first = list[0]
val safe = list?[0]          // null이면 null
val slice = array[2..5]       // range
val tail = array[3..]         // open-ended
val head = array[..3]         // open-ended
```

## if Expression

```prsm
val status = if hp > 0 { "Alive" } else { "Dead" }
func sign(x: Int): Int = if x > 0 { 1 } else if x < 0 { -1 } else { 0 }
```
```csharp
var status = _hp > 0 ? "Alive" : "Dead";
```

## when Expression

값 위치에서 when 사용 (상세: [05-control-flow.md](./05-control-flow.md)):

```prsm
val grade = when score {
    in 90..100 => "A"
    in 80..89 => "B"
    else => "F"
}
```
```csharp
var grade = _score switch {
    >= 90 and <= 100 => "A",
    >= 80 and <= 89 => "B",
    _ => "F",
};
```

## try Expression

```prsm
val result = try {
    parseInt(input)
} catch (e: FormatException) {
    0
}
```

## await

```prsm
val data = await fetchData(url)
```
```csharp
var data = await FetchData(url);
```
