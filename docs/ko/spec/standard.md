---
title: PrSM 언어 표준
parent: 사양
nav_order: 1
---

# PrSM 언어 표준

::: warning
Language 2 --- 작업 초안
:::

---

## 1 범위 [scope]

### 1.1 목적

이 문서는 PrSM 프로그래밍 언어를 명세한다. PrSM은 Unity 우선
스크립팅 언어로, Unity의 컴파일 파이프라인이 소비하는 C# 소스 파일로
컴파일(트랜스파일)된다. 적합한 구현은 본 문서에 정의된 모든 유효한
PrSM 프로그램을 수용해야 하며, 해당 프로그램에 대해 S12에 명시된
C# 출력을 생성해야 한다.

### 1.2 언어 버전

이 문서는 **Language 2**(현재)를 정의한다. Language 1에 존재하지 않았던
기능은 도입 시점에 다음과 같이 표시된다: (PrSM 2 부터)

### 1.3 대상 플랫폼

적합한 구현은 **Mono** 또는 **IL2CPP** 스크립팅 백엔드에서 실행되는
**Unity 2022.3 LTS** 이상을 대상으로 해야 한다.

### 1.4 적합성

적합한 구현은:

1. 이 문서의 구문 및 의미 규칙을 충족하는 모든 프로그램을 수용해야 한다.
2. 진단이 필요한 모든 프로그램을 거부해야 한다(오류 코드 **E0xx**).
3. Unity의 C# 컴파일러로 컴파일했을 때 각 섹션에 기술된 의미론과 동일한
   런타임 동작을 산출하는 C# 출력을 생성해야 한다.
4. 이 문서에서 요구하는 것 이상의 추가 경고를 발행할 수 있다.

---

## 2 참조 규격 [norm.ref]

다음 문서는 본 문서의 내용 일부 또는 전부가 요구사항을 구성하는 방식으로
참조된다.

| 참조 | 설명 |
|---|---|
| **Unity Engine API** | Unity Technologies, Unity Scripting Reference, 버전 2022.3 LTS 이상. |
| **C# Language Specification** | ECMA-334, 6판 이상. 생성된 C# 출력은 이 사양에 따라 유효해야 한다. |
| **`.prsmproject` 구성 형식** | 언어 버전 및 기능 게이트를 제어하는 프로젝트 수준 구성 파일. S5.8 참조. |

---

## 3 어휘 구조 [lex]

### 3.1 소스 파일 [lex.source]

PrSM 소스 파일은 **UTF-8**로 인코딩되어야 한다. 각 파일은 정확히
하나의 최상위 선언(S5)을 포함한다. 문장은 줄바꿈으로 종결되며,
세미콜론은 사용하지 않는다.

### 3.2 주석 [lex.comment]

두 가지 형태의 주석이 정의된다:

```
// 한 줄 주석 (줄 끝까지 확장)

/* 여러 줄 주석
   여러 줄에 걸칠 수 있음 */
```

주석은 렉싱 과정에서 제거되며 의미적 효과가 없다.

### 3.3 키워드 [lex.keyword]

다음 토큰은 예약 키워드이다. 키워드는 식별자로 사용할 수 없다.

**선언 키워드:**
`component` `asset` `class` `data` `enum` `attribute`

**필드 키워드:**
`serialize` `require` `optional` `child` `parent` `val` `var`

**함수 키워드:**
`func` `coroutine` `intrinsic` `override`

**제어 흐름 키워드:**
`if` `else` `when` `for` `while` `return` `break` `continue`

**이벤트 키워드:**
`listen` `unlisten`

**코루틴 키워드:**
`start` `stop` `stopAll` `wait` `nextFrame` `fixedFrame`

**연산자 키워드:**
`is` `in` `until` `downTo` `step`

**생명주기 키워드:**
`awake` `start` `update` `fixedUpdate` `lateUpdate`
`onEnable` `onDisable` `onDestroy`
`onTriggerEnter` `onTriggerExit` `onTriggerStay`
`onCollisionEnter` `onCollisionExit` `onCollisionStay`

**접근 제한자 키워드:**
`public` `private` `protected` `manual` `disable` `destroy`

**임포트 키워드:**
`using`

**리터럴 키워드:**
`true` `false` `null` `this`

### 3.4 식별자 [lex.ident]

```ebnf
IDENT = LETTER { LETTER | DIGIT | "_" } ;
LETTER = "a".."z" | "A".."Z" | "_" ;
DIGIT  = "0".."9" ;
```

대문자로 시작하는 식별자는 관례적으로 타입 이름이다(PascalCase).
멤버 및 필드 이름은 camelCase를 사용한다. 이러한 관례는 문법에 의해
강제되지 않지만 표준 라이브러리와 로우어링 규칙에서 전제된다.

PrSM 식별자가 C# 키워드와 충돌하는 경우, 생성된 코드는 `@` 접두사를
붙여야 한다(예: 매개변수 `base`는 C#에서 `@base`가 됨).

### 3.5 리터럴 [lex.literal]

#### 3.5.1 정수 리터럴

```ebnf
INT_LIT = DIGIT { DIGIT } ;
```

정수 리터럴은 `Int` 타입의 값을 나타낸다. 예: `42`, `0`, `1000`.

#### 3.5.2 부동소수점 리터럴

```ebnf
FLOAT_LIT = DIGIT { DIGIT } "." DIGIT { DIGIT } [ "f" ] ;
```

부동소수점 리터럴은 `Float` 타입의 값을 나타낸다. 후행 `f` 접미사는
선택적이다. 예: `3.14`, `1.0f`.

#### 3.5.3 지속시간 리터럴

숫자 리터럴 바로 뒤에 시간 접미사가 오면 지속시간을 나타낸다:

| 구문 | 의미 | C# 로우어링 |
|---|---|---|
| `1.5s` | 1.5초 | `new WaitForSeconds(1.5f)` |
| `500ms` | 500밀리초 | `new WaitForSeconds(0.5f)` |

지속시간 리터럴은 `wait` 문(S10)에서만 유효하다.

#### 3.5.4 문자열 리터럴

```ebnf
STRING_LIT = '"' { CHAR | ESCAPE | "$" IDENT | "${" Expr "}" } '"' ;
```

문자열 리터럴은 보간(S3.6)과 이스케이프 시퀀스(S3.7)를 지원한다.

#### 3.5.5 불리언 리터럴

`true`와 `false`는 `Bool` 타입의 값을 나타낸다.

#### 3.5.6 Null 리터럴

`null`은 널 참조를 나타낸다. 모든 nullable 타입 `T?`에 할당 가능하다.

### 3.6 문자열 보간 [lex.interp]

문자열 리터럴 내에서 두 가지 형태의 보간이 정의된다:

| 형태 | 예제 | 설명 |
|---|---|---|
| 짧은 형태 | `"hello $name"` | 식별자 `name`의 값을 삽입한다. |
| 표현식 형태 | `"score: ${player.score + 1}"` | 임의의 표현식 결과를 삽입한다. |

**로우어링.** 보간된 문자열은 C# 보간 문자열(`$"..."`)로 로우어링되어야 한다.

```prsm
val msg = "Player $name has ${hp} HP"
```

```csharp
// Generated C#
var msg = $"Player {name} has {hp} HP";
```

### 3.7 이스케이프 시퀀스 [lex.escape]

문자열 리터럴 내에서 다음 이스케이프 시퀀스가 인식된다:

| 이스케이프 | 문자 |
|---|---|
| `\n` | 줄바꿈 (U+000A) |
| `\t` | 수평 탭 (U+0009) |
| `\r` | 캐리지 리턴 (U+000D) |
| `\\` | 백슬래시 |
| `\"` | 큰따옴표 |
| `\$` | 달러 기호 (보간 억제) |

그 외의 이스케이프 시퀀스는 비정형이며, 적합한 구현은 진단을 발행해야 한다.

### 3.8 연산자 및 구두점 [lex.op]

#### 3.8.1 연산자

다음 연산자 토큰이 정의되며, 낮은 우선순위에서 높은 우선순위 순으로
나열된다:

| 우선순위 | 연산자 | 결합성 | 설명 |
|:---:|---|---|---|
| 1 | `?:` | 오른쪽 | Elvis (null 병합) |
| 2 | `\|\|` | 왼쪽 | 논리 OR |
| 3 | `&&` | 왼쪽 | 논리 AND |
| 4 | `==` `!=` | 왼쪽 | 동등성 |
| 5 | `<` `>` `<=` `>=` `is` | 왼쪽 | 비교, 타입 검사 |
| 6 | `..` `until` `downTo` | --- | 범위 |
| 7 | `+` `-` | 왼쪽 | 가산 |
| 8 | `*` `/` `%` | 왼쪽 | 승산 |
| 9 | `!` `-` (단항) | 오른쪽 | 단항 접두사 |
| 10 | `.` `?.` `!!` `[]` `()` | 왼쪽 | 접미사 |

#### 3.8.2 대입 연산자

대입은 표현식이 아닌 문장이다. 다음 대입 연산자가 정의된다:

| 연산자 | 설명 |
|---|---|
| `=` | 단순 대입 |
| `+=` `-=` `*=` `/=` `%=` | 복합 대입 |

`val` 바인딩에 대한 대입은 비정형이다(E040).

---

## 4 타입 [type]

### 4.1 원시 타입 [type.prim]

PrSM은 다음 원시 타입을 정의한다. 각각은 표시된 대로 C# 동등 타입으로
로우어링되어야 한다:

| PrSM 타입 | C# 타입 | 카테고리 |
|---|---|---|
| `Int` | `int` | 값 |
| `Float` | `float` | 값 |
| `Double` | `double` | 값 |
| `Bool` | `bool` | 값 |
| `String` | `string` | 참조 |
| `Char` | `char` | 값 |
| `Long` | `long` | 값 |
| `Byte` | `byte` | 값 |
| `Unit` | `void` | 반환 타입에만 사용 |

`Unit`은 반환 타입 위치에서만 나타나야 한다. 명시적 반환 타입이 없고
블록 본문을 가진 함수의 반환 타입은 `Unit`이다.

### 4.2 Nullable 타입 [type.nullable]

타입 뒤에 `?`가 오면 해당 타입의 nullable 변형을 나타낸다.

- 참조 타입의 경우, `T?`는 `null` 값을 허용한다.
- 값 타입의 경우, `T?`는 `System.Nullable<T>`로 로우어링된다(예: `Int?`는
  `int?`가 됨).

`?`가 없는 타입은 non-nullable이다. non-nullable 위치에 `null`을
할당하는 것은 비정형이다.

```prsm
var name: String = "Alice"     // non-nullable
var title: String? = null      // nullable
```

**Null 안전 연산자.** nullable 타입을 위해 다음 연산자가 제공된다:

| 연산자 | 이름 | 의미론 |
|---|---|---|
| `?.` | 안전 호출 | 수신자가 `null`이면 `null`로 단락한다. |
| `?:` | Elvis | 왼쪽 피연산자가 non-null이면 반환하고, 그렇지 않으면 오른쪽 피연산자를 반환한다. |
| `!!` | Non-null 단언 | 런타임에 non-null을 단언한다. `null`이면 `NullReferenceException`을 던진다. 피연산자가 이미 non-nullable이면 경고 **W001**을 발생시킨다. |

**Unity null 의미론.** `UnityEngine.Object`에서 파생된 타입의 경우,
컴파일러는 null 검사에 Unity의 오버로딩된 동등 연산자를 사용해야 한다.
`require` 필드는 `Awake()` 이후 non-null이 보장되지만 런타임에
"Unity-null"(파괴됨)이 될 수 있으며, 이는 정적으로 추적되지 않는다.

### 4.3 제네릭 타입 [type.generic]

PrSM은 다음 제네릭 타입 별칭을 제공한다. 각각은 해당하는 .NET 타입으로
로우어링되어야 한다:

| PrSM 타입 | C# 타입 |
|---|---|
| `Array<T>` | `T[]` |
| `List<T>` | `System.Collections.Generic.List<T>` |
| `Map<K, V>` | `System.Collections.Generic.Dictionary<K, V>` |
| `Set<T>` | `System.Collections.Generic.HashSet<T>` |
| `Queue<T>` | `System.Collections.Generic.Queue<T>` |
| `Stack<T>` | `System.Collections.Generic.Stack<T>` |
| `Seq<T>` | `System.Collections.Generic.IEnumerable<T>` |

### 4.4 Unity 및 외부 타입 [type.unity]

PrSM 원시 타입이나 제네릭 별칭이 아닌 타입 이름은 변경 없이 C#로
전달되어야 한다. 여기에는 모든 Unity 타입(`Transform`, `Rigidbody`,
`Vector3`, `Quaternion` 등)과 Unity 컴파일 파이프라인에 보이는
사용자 정의 C# 타입이 포함된다.

```prsm
serialize target: Transform = null
val offset: Vector3 = Vector3.zero
```

### 4.5 타입 추론 [type.infer]

이니셜라이저가 있는 지역 변수 선언은 이니셜라이저의 타입이 모호하지
않을 때 타입 주석을 생략할 수 있다:

```prsm
val name = "Player"       // String으로 추론
val hp = 100              // Int로 추론
val speed = 5.0           // Float로 추론
var alive = true          // Bool로 추론
```

타입 추론은 **로컬에서만** 적용된다. 다음 위치에서는 항상 명시적 타입
주석이 필요하다:

- 함수 매개변수.
- 블록 본문을 가진 함수의 반환 타입.
- `require`, `optional`, `child`, `parent` 필드.
- 이니셜라이저가 없는 필드.

### 4.6 제네릭 타입 추론 [type.infer.generic] (PrSM 2 부터)

변수 선언에 명시적 타입 주석이 있는 경우, 제네릭 편의 메서드(`get`,
`find`, `child`, `parent`, `require`)는 타입 인자를 생략할 수 있다.
컴파일러는 선언의 타입 주석에서 타입 인자를 추론해야 한다.

```prsm
val rb: Rigidbody = get()         // 추론: GetComponent<Rigidbody>()
val health: Health? = child()     // 추론: GetComponentInChildren<Health>()
```

대상 타입을 결정할 수 없는 경우, 컴파일러는 **E060**을 발행해야 한다.

### 4.7 타입 변환 [type.conv]

PrSM은 암시적 타입 변환을 정의하지 않는다. 모든 타입 관계는
C# 타입 시스템에 위임된다. 명시적 변환은 `intrinsic` 블록을 통하거나
생성된 코드 수준에서 C# 암시적 변환에 의존하여 수행할 수 있다.

---

## 5 선언 [decl]

### 5.1 파일 구조 [decl.file]

PrSM 소스 파일은 다음 구조를 가진다:

```ebnf
File = { UsingDecl } Declaration ;
```

각 파일은 정확히 하나의 최상위 선언을 포함해야 한다. 파일 이름은
선언된 타입 이름과 일치해야 한다(예: `component Player`의 경우
`Player.prsm`).

### 5.2 Using 선언 [decl.using]

```ebnf
UsingDecl = "using" QualifiedName NEWLINE ;
```

`using` 선언은 .NET 또는 Unity 네임스페이스를 스코프에 도입한다.
최상위 선언 앞에 나타나야 한다.

```prsm
using UnityEngine
using System.Collections.Generic
```

**로우어링.** 각 `using` 선언은 C# `using` 디렉티브로 로우어링된다.

### 5.3 Component 선언 [decl.component]

```ebnf
ComponentDecl = "component" IDENT [ ":" TypeRef { "," TypeRef } ] "{" { ComponentMember } "}" ;
```

`component` 선언은 Unity MonoBehaviour를 정의한다. `:` 뒤의 선택적
타입 참조는 구현할 인터페이스를 지정한다. component는 암묵적으로
`MonoBehaviour`를 확장하며, 기본 클래스를 지정하는 것은 비정형이다.

**로우어링:**

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

**제약사항:**

1. component는 다른 component를 확장할 수 없다(E001).
2. component는 생성자를 선언할 수 없다(E002).
3. 파일당 정확히 하나의 component만 허용된다(E003).
4. component는 다른 선언 내부에 중첩될 수 없다(E004).

### 5.4 Asset 선언 [decl.asset]

```ebnf
AssetDecl = "asset" IDENT [ ":" TypeRef ] "{" { AssetMember } "}" ;
```

`asset` 선언은 Unity ScriptableObject를 정의한다. asset은 필드와 함수를
포함할 수 있지만 생명주기 블록(E012)이나 컴포넌트 룩업 필드(E013)를
포함해서는 안 된다.

**로우어링:**

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

컴파일러는 `[CreateAssetMenu]` 어트리뷰트를 자동으로 생성해야 한다.

### 5.5 Class 선언 [decl.class]

```ebnf
ClassDecl = "class" IDENT [ ":" TypeRef { "," TypeRef } ] "{" { ClassMember } "}" ;
```

`class` 선언은 일반 C# 클래스를 정의한다. 클래스는 단일 상속과
다중 인터페이스 구현을 지원한다. 클래스는 생명주기 블록(E012)이나
컴포넌트 룩업 필드(E013)를 포함해서는 안 된다. 클래스의 필드는
기본적으로 직렬화되지 **않는다**.

**로우어링:**

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

### 5.6 Data class 선언 [decl.data]

```ebnf
DataClassDecl = "data" "class" IDENT "(" ParamList ")" ;
```

`data class`는 값 의미론을 가진 클래스를 선언한다. 컴파일러는 다음을
생성해야 한다:

1. 선언된 모든 필드를 받는 생성자.
2. 모든 필드를 기반으로 한 `Equals(object)` 및 `GetHashCode()`.
3. 사람이 읽을 수 있는 표현을 반환하는 `ToString()`.

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

### 5.7 Enum 선언 [decl.enum]

#### 5.7.1 단순 enum

```ebnf
EnumDecl = "enum" IDENT "{" EnumEntry { "," EnumEntry } [ "," ] "}" ;
```

단순 enum은 C# `enum`으로 로우어링된다. 항목은 쉼표로 구분되며,
후행 쉼표가 허용된다.

```prsm
enum Direction {
    Up, Down, Left, Right
}
```

```csharp
// Generated C#
public enum Direction { Up, Down, Left, Right }
```

enum은 최소 하나의 항목을 가져야 한다(E050). 중복된 항목 이름은
비정형이다(E051).

#### 5.7.2 매개변수화된 enum

```ebnf
EnumDecl = "enum" IDENT "(" ParamList ")" "{" EnumEntry { "," EnumEntry } [ "," ] "}" ;
EnumEntry = IDENT "(" ExprList ")" ;
```

매개변수화된 enum은 C# `enum`과 함께 매개변수당 하나의 확장 메서드를
포함하는 확장 클래스를 생성한다.

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

enum 매개변수는 `val`이어야 한다(E052).

### 5.8 Attribute 선언 [decl.attr]

```ebnf
AttributeDecl = "attribute" IDENT [ "(" ParamList ")" ] "{" { ClassMember } "}" ;
```

`attribute` 선언은 사용자 정의 C# 어트리뷰트를 정의한다. 매개변수는
어트리뷰트 생성자 매개변수가 된다.

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

### 5.9 기능 게이트 [decl.feature] (PrSM 2 부터)

`.prsmproject` 파일은 언어 버전과 활성화된 기능 세트를 제어한다.
적합한 구현은 `language.version` 필드를 읽어 언어 수준을 결정하고
`features` 배열에 나열된 기능만 활성화해야 한다.

```json
{
  "language": {
    "version": 2,
    "features": ["pattern-bindings", "input-system", "auto-unlisten"]
  }
}
```

다음 기능 식별자가 정의된다:

| 기능 ID | 설명 |
|---|---|
| `pattern-bindings` | `when` 분기에서 `val` 바인딩 패턴을 활성화한다. |
| `input-system` | `listen` 블록에 대한 Input System 통합을 활성화한다. |
| `auto-unlisten` | `onDisable`에서 자동 구독 해제를 활성화한다. |

게이트된 기능을 `features` 배열에 나열하지 않고 사용하는 프로그램은
비정형이다(E070).

---

## 6 필드 [field]

### 6.1 필드 선언 [field.decl]

```ebnf
FieldDecl = { Annotation } [ VisibilityMod ] FieldKind IDENT ":" TypeRef [ "=" Expr ] NEWLINE ;
FieldKind = "serialize" | "require" | "optional" | "child" | "parent" | "val" | "var" ;
```

필드 선언은 둘러싸는 타입에 이름 있는 멤버를 도입한다.
필드 종류는 가변성, 직렬화 및 초기화 동작을 결정한다.

### 6.2 Serialize 필드 [field.serialize]

`serialize` 키워드는 Unity 직렬화를 위해 필드를 명시적으로 표시한다.

```prsm
serialize val speed: Float = 5.0
serialize var health: Int = 100
```

`serialize` 뒤의 `val` 또는 `var` 수정자는 가변성을 제어한다:

- `serialize val` -- 필드가 직렬화되고 PrSM 코드에서 읽기 전용이다.
- `serialize var` -- 필드가 직렬화되고 가변이다.
- `serialize` 단독(`val`/`var` 없이)은 `serialize var`와 동일하다.

**`serialize val` 로우어링:**

```csharp
[SerializeField] private float _speed = 5.0f;
public float speed => _speed;
```

**`serialize var` 로우어링:**

```csharp
[SerializeField] public int health = 100;
```

### 6.3 Val 및 var 필드 [field.valvar]

| 한정자 | 가변성 | component/asset에서 직렬화 여부 | C# 로우어링 (component) |
|---|---|---|---|
| `val` | 초기화 후 불변 | 예 (직렬화 가능 타입) | `[SerializeField] private T _f; public T f => _f;` |
| `var` | 가변 | 예 (직렬화 가능 타입) | `[SerializeField] public T f;` |

`class` 선언에서 `val` 및 `var` 필드는 직렬화되지 **않는다**. 일반 C#
필드 또는 프로퍼티로 로우어링된다.

초기화 후 `val` 필드에 대입하는 것은 비정형이다(**E040**).

```prsm
component Player {
    val maxHp: Int = 100
    var currentHp: Int = 100

    func takeDamage(amount: Int) {
        currentHp -= amount
        // maxHp = 200  // E040: val에 대입 불가
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

### 6.4 가시성 [field.vis]

필드에는 세 가지 가시성 수준이 있다:

| PrSM 수정자 | C# 수정자 | 기본 적용 대상 |
|---|---|---|
| `public` | `public` | component, asset 필드 |
| `private` | `private` | -- |
| `protected` | `protected` | -- |

가시성 수정자가 지정되지 않으면 component 및 asset 필드는 `public`이
기본값이다. class 필드도 `public`이 기본값이다.

### 6.5 컴포넌트 룩업 필드 [field.lookup]

`require`, `optional`, `child`, `parent` 필드 종류는 `component` 선언
**내부에서만** 유효하다. `asset`이나 `class`에서 사용하는 것은
비정형이다(**E013**).

#### 6.5.1 require

```prsm
require rb: Rigidbody
```

형제 컴포넌트에 대한 non-nullable 의존성을 선언한다. 컴파일러는
`Awake()`에서 `GetComponent<T>()` 호출을 생성해야 한다. 컴포넌트를
찾지 못하면 구현은 오류를 로그하고 컴포넌트를 비활성화해야 한다.

**로우어링:**

```csharp
private Rigidbody _rb;
public Rigidbody rb => _rb;

// Awake()에서:
_rb = GetComponent<Rigidbody>();
if (_rb == null) {
    Debug.LogError($"[Player] Required component Rigidbody not found on {gameObject.name}", this);
    enabled = false;
    return;
}
```

`require` 필드는 이니셜라이저를 가질 수 없다(E041). `require` 필드는
`UnityEngine.Component`에서 파생된 타입을 참조해야 한다(E042).

#### 6.5.2 optional

```prsm
optional audioSrc: AudioSource
```

nullable 의존성을 선언한다. `Awake()`에서 `GetComponent<T>()`를 통해
가져온다. 찾지 못하면 필드는 `null`로 유지되고 오류가 발행되지 않는다.
필드의 유효 타입은 `T?`이다.

#### 6.5.3 child

```prsm
child healthBar: HealthBar
```

`require`와 유사하지만 `GetComponentInChildren<T>()`를 사용한다.
필드는 non-nullable이며, 자식 컴포넌트가 없으면 오류를 발생시키고
컴포넌트를 비활성화한다.

#### 6.5.4 parent

```prsm
parent manager: GameManager
```

`require`와 유사하지만 `GetComponentInParent<T>()`를 사용한다.
필드는 non-nullable이며, 부모 컴포넌트가 없으면 오류를 발생시키고
컴포넌트를 비활성화한다.

#### 6.5.5 룩업 요약

| 한정자 | 가져오기 메서드 | 미발견 시 null | 필드 nullable 여부 |
|---|---|---|---|
| `require` | `GetComponent<T>()` | 오류 + 비활성화 | Non-null `T` |
| `optional` | `GetComponent<T>()` | 무시 | Nullable `T?` |
| `child` | `GetComponentInChildren<T>()` | 오류 + 비활성화 | Non-null `T` |
| `parent` | `GetComponentInParent<T>()` | 오류 + 비활성화 | Non-null `T` |

### 6.6 필드 어노테이션 [field.ann]

어노테이션은 직렬화 또는 Inspector 표시를 수정한다. 필드 선언 앞에
배치된다.

```ebnf
Annotation = "@" IDENT [ "(" AnnotationArgs ")" ] NEWLINE ;
```

다음 내장 어노테이션이 정의된다:

| 어노테이션 | C# 어트리뷰트 | 설명 |
|---|---|---|
| `@header("text")` | `[Header("text")]` | Inspector의 섹션 헤더. |
| `@tooltip("text")` | `[Tooltip("text")]` | Inspector의 호버 툴팁. |
| `@range(min, max)` | `[Range(min, max)]` | Inspector의 숫자 슬라이더. |
| `@space` | `[Space]` | Inspector의 시각적 간격. |
| `@space(n)` | `[Space(n)]` | `n` 픽셀의 시각적 간격. |
| `@hideInInspector` | `[HideInInspector]` | Inspector에서 필드를 숨긴다. |

```prsm
@header("Movement")
@tooltip("Units per second")
@range(0, 20)
serialize val speed: Float = 5.0

@space
@hideInInspector
var internalTimer: Float = 0.0
```

### 6.7 초기화 순서 [field.init]

component 내에서 필드는 다음 순서로 초기화된다. 적합한 구현은
이 순서를 보존해야 한다:

1. **Unity `Awake()` 진입** -- 런타임이 생성된 `Awake` 메서드를 호출한다.
2. **컴포넌트 룩업 해석** -- `require`, `optional`, `child`, `parent`
   필드가 `GetComponent` 변형을 통해 해석된다. `require`, `child`,
   `parent` 룩업이 실패하면 컴포넌트가 비활성화되고 초기화가 중단된다.
   사용자 `awake` 블록은 실행되지 **않는다**.
3. **직렬화된 필드 기본값** -- Unity가 Inspector 또는 에셋 데이터의
   직렬화된 값을 적용한다(이는 `Awake` 전에 Unity에 의해 주입됨).
4. **사용자 `awake` 본문** -- `awake` 생명주기 블록의 본문이 실행된다.
5. **Unity `Start()` 진입** -- 첫 번째 프레임에서 생성된 `Start` 메서드가
   사용자의 `start` 생명주기 블록 본문을 실행한다.

```prsm
component Player {
    require rb: Rigidbody
    optional audio: AudioSource
    val maxSpeed: Float = 10.0

    awake {
        rb.useGravity = false
    }

    start {
        // 첫 번째 프레임에서 실행
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
        // 2단계: 룩업 해석
        _rb = GetComponent<Rigidbody>();
        if (_rb == null) {
            Debug.LogError($"[Player] Required component Rigidbody not found on {gameObject.name}", this);
            enabled = false;
            return;
        }
        _audio = GetComponent<AudioSource>();

        // 4단계: 사용자 awake 본문
        _rb.useGravity = false;
    }

    private void Start() {
        // 5단계: 사용자 start 본문
    }
}
```

---

## 7 함수 [func]

### 7.1 함수 선언 [func.decl]

```ebnf
FuncDecl = [ VisibilityMod ] [ "override" ] "func" IDENT "(" [ ParamList ] ")" [ ":" TypeRef ]
           ( Block | "=" Expr NEWLINE ) ;
```

함수 선언은 이름 있는 호출 가능 멤버를 도입한다. 반환 타입이 생략되고
본문이 블록인 경우 반환 타입은 `Unit`이다. 본문이 표현식(`= Expr`)인
경우 반환 타입은 표현식에서 추론된다.

### 7.2 매개변수 [func.param]

```ebnf
ParamList = Param { "," Param } ;
Param     = IDENT ":" TypeRef [ "=" Expr ] ;
```

매개변수는 명시적 타입 주석을 가져야 한다. 기본값이 허용되며,
기본값이 있는 매개변수는 기본값이 없는 매개변수 뒤에 나타나야 한다.

```prsm
func attack(target: Enemy, damage: Int = 10) {
    target.takeDamage(damage)
}
```

### 7.3 표현식 본문 함수 [func.expr]

함수는 블록 대신 `= Expr`를 본문으로 사용할 수 있다. 이는 단일 `return`
문을 포함하는 블록의 문법적 설탕이다.

```prsm
func isAlive(): Bool = hp > 0
func greeting(): String = "Hello, $name!"
```

```csharp
// Generated C#
public bool isAlive() => hp > 0;
public string greeting() => $"Hello, {name}!";
```

### 7.4 이름 있는 인자 [func.named]

호출 위치에서 인자는 `name = value` 구문으로 이름으로 전달할 수 있다.
이름 있는 인자는 임의의 순서로 나타날 수 있지만, 위치 인자 앞에
나타나서는 안 된다.

```prsm
func spawn(x: Float, y: Float, z: Float = 0.0) { /* ... */ }

// 호출 위치:
spawn(1.0, 2.0)
spawn(x = 1.0, y = 2.0, z = 3.0)
spawn(1.0, z = 5.0, y = 2.0)
```

### 7.5 Override [func.override]

`override` 수정자는 함수가 상속된 또는 인터페이스 메서드를 오버라이드할 때
사용해야 한다. 오버라이드가 필요한 곳에서 `override`를 생략하는 것은
비정형이다(**E030**).

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

`toString()`과 같은 잘 알려진 오버라이드의 경우, 컴파일러는 적절한
C# `override`를 생성해야 한다:

```prsm
override func toString(): String = "Player($name)"
```

```csharp
// Generated C#
public override string ToString() => $"Player({name})";
```

### 7.6 가시성 [func.vis]

함수에는 세 가지 가시성 수준이 있다:

| PrSM 수정자 | C# 수정자 |
|---|---|
| `public` (기본값) | `public` |
| `private` | `private` |
| `protected` | `protected` |

수정자가 지정되지 않으면 함수는 `public`이 기본값이다.

### 7.7 Intrinsic 함수 [func.intrinsic]

```ebnf
IntrinsicFunc = "intrinsic" "func" IDENT "(" [ ParamList ] ")" [ ":" TypeRef ] Block ;
```

intrinsic 함수는 본문에 원시 C# 코드를 포함한다. 컴파일러는 변환 없이
본문을 생성된 C# 출력에 그대로 내보내야 한다.

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

### 7.8 Intrinsic 코루틴 [func.intrinsic.coro]

```ebnf
IntrinsicCoro = "intrinsic" "coroutine" IDENT "(" [ ParamList ] ")" Block ;
```

intrinsic 코루틴은 원시 C# 코루틴 코드를 포함한다. 컴파일러는 이를
`System.Collections.IEnumerator`를 반환하는 메서드로 내보내야 한다.

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

## 8 생명주기 블록 [lifecycle]

### 8.1 일반 사항 [lifecycle.general]

생명주기 블록은 Unity 메시지 메서드로 로우어링되는 익명 블록이다.
생명주기 블록은 `component` 선언 내부에서만 나타나야 한다.
`asset`이나 `class` 내에서 생명주기 블록을 사용하는 것은
비정형이다(**E012**).

```ebnf
LifecycleBlock = LifecycleName [ LifecycleParam ] Block ;
LifecycleParam = "(" IDENT ":" TypeRef ")" ;
```

각 생명주기 종류는 component당 최대 한 번 나타나야 한다. 중복된
생명주기 블록은 비정형이다(**E014**).

### 8.2 생명주기 종류 [lifecycle.kinds]

다음 생명주기 블록이 정의된다. 각각은 해당하는 Unity 메시지 메서드로
로우어링되어야 한다:

| PrSM 블록 | C# 메서드 | 매개변수 | 시점 |
|---|---|---|---|
| `awake` | `Awake()` | -- | 인스턴스 생성 |
| `start` | `Start()` | -- | 첫 프레임 전 |
| `update` | `Update()` | -- | 매 프레임 |
| `fixedUpdate` | `FixedUpdate()` | -- | 고정 타임스텝 |
| `lateUpdate` | `LateUpdate()` | -- | 모든 `Update` 호출 후 |
| `onEnable` | `OnEnable()` | -- | 컴포넌트 활성화 |
| `onDisable` | `OnDisable()` | -- | 컴포넌트 비활성화 |
| `onDestroy` | `OnDestroy()` | -- | 컴포넌트 파괴 |
| `onTriggerEnter` | `OnTriggerEnter(Collider)` | `Collider` | 트리거 진입 |
| `onTriggerExit` | `OnTriggerExit(Collider)` | `Collider` | 트리거 이탈 |
| `onTriggerStay` | `OnTriggerStay(Collider)` | `Collider` | 트리거 지속 |
| `onCollisionEnter` | `OnCollisionEnter(Collision)` | `Collision` | 충돌 진입 |
| `onCollisionExit` | `OnCollisionExit(Collision)` | `Collision` | 충돌 이탈 |
| `onCollisionStay` | `OnCollisionStay(Collision)` | `Collision` | 충돌 지속 |

### 8.3 매개변수 없는 생명주기 블록 [lifecycle.noparam]

매개변수가 없는 블록(`awake`, `start`, `update`, `fixedUpdate`,
`lateUpdate`, `onEnable`, `onDisable`, `onDestroy`)은 매개변수가 없는
`private void` 메서드로 로우어링된다.

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

### 8.4 매개변수화된 생명주기 블록 [lifecycle.param]

트리거 및 충돌 블록은 단일 매개변수를 받는다. 매개변수 이름은
사용자가 선택하며, 타입은 생명주기 종류에 의해 결정된다:

- `onTriggerEnter`, `onTriggerExit`, `onTriggerStay` -- 매개변수 타입은
  `Collider`.
- `onCollisionEnter`, `onCollisionExit`, `onCollisionStay` -- 매개변수 타입은
  `Collision`.

매개변수 타입 주석이 제공되면 예상 타입과 일치해야 한다. 그렇지 않으면
추론된다.

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

### 8.5 로우어링 규칙 [lifecycle.lower]

component에 존재하는 각 생명주기 블록에 대해, 컴파일러는 해당하는
`private void` C# 메서드를 생성해야 한다. PrSM 블록의 본문은
표준 표현식 및 문장 로우어링이 적용된 메서드 본문이 되어야 한다.

component가 컴포넌트 룩업 필드(S6.5)와 `awake` 블록을 모두 포함하는
경우, 컴파일러는 먼저 룩업 해석을 수행한 다음 S6.7에 명시된 대로
사용자 블록 본문을 실행하는 단일 `Awake()` 메서드를 생성해야 한다.

### 8.6 생명주기 블록과 코루틴 [lifecycle.coro]

생명주기 블록 본문에는 `wait` 문을 직접 포함할 수 없다(E015).
코루틴 연산은 `coroutine` 선언(S9) 내부에서만 유효하다.
그러나 생명주기 블록은 `start coroutineName()`을 통해 코루틴을
시작할 수 있다.

```prsm
component FadeIn {
    coroutine doFade() {
        wait 1.0s
        // 페이드 로직
    }

    start {
        start doFade()
    }
}
```

### 8.7 오류 요약 [lifecycle.errors]

| 코드 | 조건 |
|---|---|
| **E012** | `asset` 또는 `class` 선언에 생명주기 블록이 있음. |
| **E014** | 단일 component에 같은 종류의 생명주기 블록이 중복됨. |
| **E015** | 생명주기 블록 본문 내에 직접 `wait` 문이 있음. |
## 9 표현식

### 9.1 연산자 우선순위

다음 표는 모든 연산자를 **낮은** 우선순위에서 **높은** 우선순위 순으로
나열한다. 같은 우선순위 수준의 연산자는 별도 표기가 없으면
왼쪽에서 오른쪽으로 결합한다.

| 수준 | 연산자 | 결합성 | 설명 |
|-------|-----------|---------------|-------------|
| 1 | `?:` | 오른쪽 | Elvis (null 병합) |
| 2 | `\|\|` | 왼쪽 | 논리 OR |
| 3 | `&&` | 왼쪽 | 논리 AND |
| 4 | `==` `!=` | 왼쪽 | 동등성 |
| 5 | `<` `>` `<=` `>=` `is` | 왼쪽 | 비교 / 타입 검사 |
| 6 | `..` `until` `downTo` | 없음 | 범위 생성 |
| 7 | `+` `-` | 왼쪽 | 가산 |
| 8 | `*` `/` `%` | 왼쪽 | 승산 |
| 9 | `!` `-` (단항) | 오른쪽 (접두사) | 단항 |
| 10 | `.` `?.` `!!` `[]` `()` | 왼쪽 | 접미사 / 멤버 접근 |

### 9.2 이항 연산자

컴파일러는 다음 이항 연산자를 표준 의미론으로 지원해야 한다:

| 연산자 | 의미 | 피연산자 타입 |
|----------|---------|---------------|
| `+` | 덧셈 / 문자열 연결 | 숫자, String |
| `-` | 뺄셈 | 숫자 |
| `*` | 곱셈 | 숫자 |
| `/` | 나눗셈 | 숫자 |
| `%` | 나머지 | 숫자 |
| `==` | 구조적 동등 | 모든 타입 |
| `!=` | 구조적 부등 | 모든 타입 |
| `<` `>` `<=` `>=` | 순서 비교 | 숫자, IComparable |
| `&&` | 단락 논리 AND | Boolean |
| `\|\|` | 단락 논리 OR | Boolean |

컴파일러는 `==`와 `!=`를 C# `==`와 `!=`로 로우어링해야 한다. Unity 객체의 경우 이는 Unity의 사용자 정의 동등 의미론을 보존한다.

### 9.3 단항 연산자

| 연산자 | 의미 | 피연산자 타입 |
|----------|---------|--------------|
| `!` | 논리 NOT | Boolean |
| `-` | 수치 부정 | 숫자 |

### 9.4 Null 안전 연산자

#### 안전 멤버 접근 (`?.`)

```prsm
val name = enemy?.name
```

표현식 `a?.b`는 `a`가 null이면 `null`로, 그렇지 않으면 `a.b`로 평가되어야 한다. 결과 타입은 멤버 타입의 nullable 변형이다.

**로우어링:**

```csharp
var name = enemy != null ? enemy.name : null;
```

#### Elvis 연산자 (`?:`)

```prsm
val name = player?.name ?: "Unknown"
```

표현식 `a ?: b`는 `a`가 non-null이면 `a`로, 그렇지 않으면 `b`로 평가되어야 한다. 오른쪽 피연산자는 왼쪽 피연산자의 non-null 형태와 타입 호환이어야 한다.

**로우어링:**

```csharp
var name = player?.name ?? "Unknown";
```

#### Non-null 단언 (`!!`)

```prsm
val name = nullableName!!
```

표현식 `a!!`는 런타임에 `a`가 non-null임을 단언해야 한다. `a`가 null이면 컴파일러는 `NullReferenceException`을 던지는 코드를 내보내야 한다. 타입이 이미 non-nullable인 표현식에 `!!`가 적용되면 컴파일러는 경고 **W001**을 내보내야 한다.

**로우어링:**

```csharp
var name = nullableName ?? throw new System.NullReferenceException(
    "Non-null assertion failed");
```

### 9.5 타입 검사 (`is`)

```prsm
if enemy is Boss {
    enemy.enrage()
}
```

표현식 `expr is Type`은 `expr`의 런타임 타입이 `Type` 또는 그 하위 타입일 때 `true`로 평가되어야 한다. 조건에서 `is` 검사가 성공한 후, 컴파일러는 참 분기 내에서 변수를 `Type`으로 스마트 캐스트해야 한다.

**로우어링:**

```csharp
if (enemy is Boss) {
    enemy.enrage();
}
```

### 9.6 `if` 표현식

`if`가 표현식 위치에 나타나면 두 분기 모두 필요하며 컴파일러는 값을 생성해야 한다.

```prsm
val max = if a > b { a } else { b }
```

두 분기는 공통 상위 타입을 공유하는 타입을 생성해야 한다. 표현식 형태 `if`에서 `else` 분기는 필수이다(없으면 E100).

**로우어링:**

```csharp
var max = (a > b) ? a : b;
```

### 9.7 `when` 표현식

`when`이 표현식 위치에 나타나면 완전해야 한다. enum 대상의 경우 모든 변형이 커버되거나 `else` 분기가 있어야 한다. 완전하지 않은 `when` 표현식은 **E100**을 생성한다.

```prsm
val label = when state {
    State.Idle => "Idle"
    State.Running => "Moving"
    else => "Unknown"
}
```

**로우어링:**

```csharp
var label = state switch {
    State.Idle => "Idle",
    State.Running => "Moving",
    _ => "Unknown"
};
```

### 9.8 범위 표현식

범위 표현식은 반복을 위한 시퀀스를 생성한다:

| PrSM | 의미론 | 로우어링된 `for` 동등물 |
|------|-----------|--------------------------|
| `start..end` | 포함 `[start, end]` | `i <= end` |
| `start until end` | 상한 제외 `[start, end)` | `i < end` |
| `start downTo end` | 내림차순 `[start, end]` | `i >= end; i--` |
| `expr step N` | 보폭 수정자 | `i += N` 또는 `i -= N` |

```prsm
for i in 0 until 10 step 2 {
    log("$i")
}
```

**로우어링:**

```csharp
for (int i = 0; i < 10; i += 2) {
    Debug.Log($"{i}");
}
```

### 9.9 편의 호출

컴파일러는 다음 호출 위치 편의 구문을 인식하고 Unity 동등물로 로우어링해야 한다. 편의 호출은 스코프 내 식별자가 아닌 로우어링 과정에서 해석된다.

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

#### 메서드 편의 구문 (레거시 입력)

| PrSM | C# |
|------|-----|
| `input.axis(name)` | `Input.GetAxis(name)` |
| `input.getKey(key)` | `Input.GetKey(key)` |
| `input.getKeyDown(key)` | `Input.GetKeyDown(key)` |
| `input.getKeyUp(key)` | `Input.GetKeyUp(key)` |
| `input.getMouseButton(n)` | `Input.GetMouseButton(n)` |

### 9.10 제네릭 타입 추론 (PrSM 2 부터)

제네릭 편의 호출이 모호하지 않은 대상 타입이 있는 컨텍스트에 나타나면, 컴파일러는 타입 인자를 추론해야 한다.

```prsm
val rb: Rigidbody = get()   // get<Rigidbody>()로 추론
```

추론 규칙 (우선순위 순):

1. **변수 타입 주석** -- 수신 변수의 선언된 타입에서 추론한다.
2. **반환 타입** -- 둘러싸는 함수의 반환 타입에서 추론한다.
3. **인자 타입** -- 호출 위치의 매개변수 타입에서 추론한다.

컴파일러는 단일 모호하지 않은 해를 요구해야 한다. 추론이 실패하면 컴파일러는 명시적 타입 인자를 요청하는 **E020**을 발행해야 한다.

### 9.11 `when`의 패턴 바인딩 (PrSM 2 부터)

`when` 분기가 페이로드 enum 변형과 일치할 때, 바인딩이 페이로드 값을 추출한다:

```prsm
when state {
    EnemyState.Chase(target) => moveTo(target)
    EnemyState.Stunned(dur) if dur > 0.0 => waitStun(dur)
    else => idle()
}
```

바인딩 개수는 enum 항목의 매개변수 수와 일치해야 한다. 불일치 시 **E082**가 생성된다. enum에 정의되지 않은 변형을 참조하면 **E081**이 생성된다.

---

## 10 문장

### 10.1 `val` 선언

```prsm
val name: Type = initializer
val name = initializer          // 타입 추론
```

`val` 키워드는 불변 바인딩을 선언한다. 변수는 초기화 후 재할당할 수 없다(**E040**). 타입 주석 또는 이니셜라이저(또는 둘 다)가 있어야 한다(**E022**).

### 10.2 `var` 선언

```prsm
var name: Type = initializer
var name: Type                  // 초기화되지 않음, 타입 필수
var name = initializer          // 타입 추론
```

`var` 키워드는 가변 바인딩을 선언한다. 타입 주석도 이니셜라이저도 없는 `var` 선언은 **E022**를 생성해야 한다.

### 10.3 대입

```prsm
target = value
```

단순 대입. 컴파일러는 `val` 바인딩(**E040**)과 `require` 필드(**E041**)에 대한 대입을 거부해야 한다.

#### 복합 대입

복합 대입 연산자 `+=`, `-=`, `*=`, `/=`, `%=`는 `target = target op value`로 역설탕화되어야 한다.

```prsm
health -= damage
// 동등: health = health - damage
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

조건은 `Boolean` 타입이어야 한다. C#과 달리 조건 주위의 괄호는 **필수가 아니다**. 중괄호는 필수이다.

**로우어링:**

```csharp
if (condition) {
    // body
} else if (otherCondition) {
    // body
} else {
    // body
}
```

### 10.5 `when` 문

#### 대상 형태

```prsm
when subject {
    Pattern => body
    Pattern => body
    else => fallback
}
```

컴파일러는 대상을 한 번 평가하고 분기를 위에서 아래로 일치시켜야 한다. 첫 번째로 일치하는 분기가 실행된다.

#### 조건 형태

```prsm
when {
    health < 20 => flee()
    health < 50 => defend()
    else => attack()
}
```

대상이 없으면 각 분기 조건은 독립적인 Boolean 표현식이다. 컴파일러는 이를 `if`/`else if` 체인으로 로우어링해야 한다.

#### 완전성

`when` 문이 enum 타입에 대해 매칭하고 모든 변형을 커버하지 않으며 `else` 분기가 없는 경우, 컴파일러는 경고 **W003**을 발행해야 한다. (PrSM 2 부터)

#### 패턴 바인딩

```prsm
when result {
    Result.Ok(val value) => log("$value")
    Result.Err(val msg) => error(msg)
}
```

바인딩은 enum 변형에서 페이로드 값을 추출한다. 바인딩 수는 변형의 매개변수 수와 일치해야 한다(**E082**). 알 수 없는 변형 이름은 **E081**을 생성해야 한다.

#### 가드

```prsm
when state {
    EnemyState.Stunned(dur) if dur > 2.0 => longStun()
    EnemyState.Stunned(dur) => shortStun()
}
```

가드는 매칭 후 조건을 추가한다. 패턴이 일치하지만 가드가 `false`로 평가되면 컴파일러는 다음 분기로 넘어가야 한다.

**로우어링 (가드가 있는 패턴 바인딩):**

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

`for` 루프는 `IEnumerable<T>`를 구현하거나 호환 가능한 `GetEnumerator()` 메서드를 가진 모든 값에 대해 반복한다. 범위 표현식(9.8 참조)은 할당 없는 C 스타일 `for` 루프로 로우어링된다.

```prsm
for i in 0 until 10 { log("$i") }
for i in 10 downTo 1 step 2 { log("$i") }
```

#### `for`에서의 구조 분해

```prsm
for Result.Ok(val value) in results {
    log("$value")
}
```

루프 변수는 구조 분해 패턴일 수 있다. 컴파일러는 패턴 바인딩과 동일한 규칙을 사용하여 각 요소에서 필드를 추출해야 한다.

### 10.7 `while`

```prsm
while condition {
    body
}
```

컴파일러는 `while`을 C# `while`로 직접 로우어링해야 한다. 조건은 `Boolean` 타입이어야 한다.

### 10.8 `return`

```prsm
return expr
return
```

`return` 문은 둘러싸는 함수를 종료한다. 반환 타입이 `Unit`인 함수에서는 표현식을 생략해야 한다.

### 10.9 `break`와 `continue`

```prsm
break
continue
```

`break`와 `continue`는 `for` 또는 `while` 루프 본문 내에서만 나타나야 한다. 루프 외부에서 사용하면 **E031**이 생성된다.

### 10.10 구조 분해 `val` (PrSM 2 부터)

```prsm
val Result.Ok(value) = expr
```

구조 분해는 data class 또는 enum 페이로드에서 필드를 지역 바인딩으로 추출한다. 컴파일러는 `Item1`, `Item2` 등을 사용하여 개별 필드 접근으로 로우어링해야 한다.

### 10.11 `listen`

`listen` 문은 코드 블록을 Unity 이벤트 필드에 연결한다. 모든 컴포넌트 룩업이 해석된 후 생성된 `Awake()` 본문에 `AddListener(...)` 호출로 로우어링되어야 한다.

#### 기본 형태

```prsm
listen button.onClick {
    fire()
}
```

**로우어링:**

```csharp
void Awake() {
    // ... require/optional 해석 ...
    button.onClick.AddListener(() => { fire(); });
}
```

#### 매개변수 포함

```prsm
listen slider.onValueChanged { val newValue ->
    log("$newValue")
}
```

**로우어링:**

```csharp
slider.onValueChanged.AddListener((newValue) => { Debug.Log($"{newValue}"); });
```

#### 수명 수정자

수명 수정자는 `component` 선언 내부에서만 나타나야 한다. component 외부에서 사용하면 **E083**이 생성된다.

##### `until disable`

```prsm
listen button.onClick until disable {
    fire()
}
```

컴파일러는 private 핸들러 필드를 생성하고, 생명주기 등록 단계에서 리스너를 등록하며, `OnDisable`에서 정리 코드를 내보내야 한다.

**로우어링:**

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

`until disable`와 동일한 패턴이지만, 정리가 `OnDestroy`에서 실행된다.

##### `manual`

```prsm
val token = listen timer.onFinished manual {
    reset()
}
```

`manual` 수정자는 구독 토큰을 반환한다. 핸들러 필드가 생성되지만 자동 정리는 등록되지 않는다.

##### `unlisten`

```prsm
unlisten token
```

`unlisten` 문은 토큰을 그 배킹 핸들러 필드로 해석하고, `RemoveListener`를 내보내고, 필드를 null로 설정해야 한다. 선언하는 component 내 모든 메서드에서 유효하다.

#### 기본 동작

수명 수정자 없이 `listen`은 리스너만 등록한다. 자동 정리는 생성되지 않는다. 이 동작은 Language 1과 Language 2에서 동일하다.

### 10.12 Input System 편의 구문 (PrSM 2 부터)

Input System 편의 구문은 `.prsmproject`에서 `input-system` 기능 플래그를 필요로 한다. 플래그 없이 입력 편의 구문을 사용하면 **E070**이 생성된다.

#### 기본 형태

```prsm
if input.action("Jump").pressed {
    jump()
}
val move = input.action("Move").vector2
```

#### 상태 접근자

| PrSM 접근자 | C# 메서드 |
|---------------|-----------|
| `.pressed` | `WasPressedThisFrame()` |
| `.released` | `WasReleasedThisFrame()` |
| `.held` | `IsPressed()` |
| `.vector2` | `ReadValue<Vector2>()` |
| `.scalar` | `ReadValue<float>()` |

**로우어링:**

```csharp
// input.action("Jump").pressed
_prsmInput.actions["Jump"].WasPressedThisFrame()

// input.action("Move").vector2
_prsmInput.actions["Move"].ReadValue<UnityEngine.Vector2>()
```

#### 플레이어 형태 (멀티플레이어)

```prsm
if input.player("Gameplay").action("Fire").pressed {
    fireWeapon()
}
```

컴파일러는 룩업 키를 `"Map/Action"`으로 구성해야 한다:

```csharp
_prsmInput.actions["Gameplay/Fire"].WasPressedThisFrame()
```

#### 생성되는 인프라

컴파일러가 component에서 입력 편의 구문을 감지하면 다음을 생성해야 한다:

1. private 필드: `private PlayerInput _prsmInput;`
2. 사용자 문장 전에 생성된 `Awake()` 본문에 병합되는 `GetComponent<PlayerInput>()` 호출.

### 10.13 `intrinsic` 블록

```prsm
intrinsic {
    // raw C# code
    var ray = Camera.main.ScreenPointToRay(Input.mousePosition);
}
```

`intrinsic` 블록은 그 내용을 생성된 C#에 그대로 전달한다. PrSM 의미 검사기는 intrinsic 내용을 검사하지 않으며, 유효성 검사는 C# 컴파일러에 위임된다.

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

이들은 `MonoBehaviour` 코루틴 관리가 가능한 `component` 선언 내에서만 나타나야 한다.

---

## 11 코루틴

### 11.1 선언

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

`coroutine` 선언은 `component` 본문 내에서만 나타나야 한다. `asset`이나 `class`에서 코루틴을 선언하면 **E060**이 생성된다.

컴파일러는 코루틴을 `private IEnumerator` 메서드로 로우어링해야 한다:

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

### 11.2 `wait` 형태

`wait` 키워드는 `coroutine` 본문 내에서만 나타나야 한다. 코루틴 외부에서 사용하면 **E032**가 생성된다.

| PrSM | C# | 의미 |
|------|-----|---------|
| `wait 1.5s` | `yield return new WaitForSeconds(1.5f)` | N초 대기 |
| `wait nextFrame` | `yield return null` | 한 프레임 대기 |
| `wait fixedFrame` | `yield return new WaitForFixedUpdate()` | 다음 FixedUpdate까지 대기 |
| `wait until expr` | `yield return new WaitUntil(() => expr)` | 조건이 true가 될 때까지 대기 |
| `wait while expr` | `yield return new WaitWhile(() => expr)` | 조건이 true인 동안 대기 |

#### 예제

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

**로우어링:**

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

### 11.3 시작과 중지

코루틴은 `start`, `stop`, `stopAll` 문을 통해 시작하고 중지한다(10.14 참조). `start` 문은 선택적으로 나중에 취소하기 위한 핸들을 캡처할 수 있다:

```prsm
val handle = start spawnWaves()
// ...
stop handle
```

---

## 12 로우어링 규칙

이 섹션은 컴파일러가 생성해야 하는 규범적 C# 출력을 정의한다. 적합한 구현은 동등한 관찰 가능 동작을 가진 코드를 생성해야 한다.

### 12.1 직렬화된 필드

```prsm
serialize speed: Float = 5.0
```

컴파일러는 `serialize` 필드를 `[SerializeField]` private 배킹 필드와 public 읽기 전용 프로퍼티로 로우어링해야 한다:

```csharp
[SerializeField] private float _speed = 5.0f;
public float speed => _speed;
```

`var` 필드의 경우 프로퍼티에 getter와 setter가 모두 있어야 한다.

### 12.2 Awake 합성

component가 `require`, `optional`, `child`, `parent` 필드를 선언하면, 컴파일러는 다음 단계 순서를 가진 단일 `Awake()` 메서드를 생성해야 한다:

1. **1단계 -- 의존성 해석.** 선언 순서대로 모든 주입 필드를 해석한다. `require` 필드는 null 검사를 받으며, 실패 시 오류를 로그하고 `enabled = false; return;`을 통해 컴포넌트를 비활성화한다.
2. **2단계 -- 입력 인프라.** 입력 편의 구문이 있으면 `_prsmInput = GetComponent<PlayerInput>()`를 내보낸다.
3. **3단계 -- Listen 등록.** 모든 `listen` 문에 대한 `AddListener` 호출을 내보낸다.
4. **4단계 -- 사용자 `awake` 본문.** 사용자의 `awake` 블록이 있으면 내보낸다.

`require` 검사가 실패하면 2단계에서 4단계까지는 실행되지 않아야 한다.

### 12.3 데코레이터 로우어링

| PrSM | C# 어트리뷰트 |
|------|-------------|
| `@header("text")` | `[Header("text")]` |
| `@tooltip("text")` | `[Tooltip("text")]` |
| `@range(min, max)` | `[Range(min, max)]` |
| `@space` | `[Space]` |
| `@space(n)` | `[Space(n)]` |
| `@hideInInspector` | `[HideInInspector]` |

### 12.4 편의 매핑 테이블 (전체)

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

### 12.5 부동소수점 리터럴 정규화

모든 부동소수점 리터럴은 `f` 접미사와 함께 내보내져야 한다:

```prsm
val speed = 5.0
```

```csharp
float speed = 5.0f;
```

### 12.6 문자열 보간

문자열 보간 표현식은 C# 보간 문자열로 로우어링되어야 한다:

```prsm
val msg = "Player $name has ${health} HP"
```

```csharp
var msg = $"Player {name} has {health} HP";
```

`$identifier` 짧은 형태와 `${expression}` 긴 형태 모두 C# `$"..."` 문자열 내의 `{expression}`으로 로우어링되어야 한다.

### 12.7 Listen 수명 로우어링 (PrSM 2 부터)

수명 수정자가 있는 각 `listen` 문에 대해 컴파일러는:

1. 적절한 delegate 타입의 private 핸들러 필드(`_prsm_h0`, `_prsm_h1`, ...)를 생성한다.
2. 등록 단계(컨텍스트에 따라 Start 또는 Awake)에서 `AddListener`를 내보낸다.
3. 적절한 정리 메서드에서 `RemoveListener` + null 대입을 내보낸다:
   - `until disable` -- `OnDisable`에 주입.
   - `until destroy` -- `OnDestroy`에 주입.
   - `manual` -- 정리 없음; `unlisten`이 인라인으로 제거를 내보냄.

component가 이미 대상 생명주기 블록을 선언하고 있으면, 컴파일러는 사용자 본문 뒤에 정리 코드를 추가해야 한다. 해당 블록이 없으면 컴파일러는 생명주기 메서드를 합성해야 한다.

### 12.8 패턴 바인딩 로우어링 (PrSM 2 부터)

`when` 분기의 패턴 바인딩은 enum 태그에 대한 `switch`로 로우어링되고, 이어서 튜플 필드 추출이 수행되어야 한다:

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

### 12.9 Input System 로우어링 (PrSM 2 부터)

component가 Input System 편의 구문을 사용하면, 컴파일러는:

1. private 필드를 내보낸다: `private PlayerInput _prsmInput;`
2. Awake 2단계에서 `_prsmInput = GetComponent<PlayerInput>();`를 내보낸다.
3. 각 `input.action("Name").accessor`를 `_prsmInput.actions["Name"].Method()`로 대체한다.
4. 플레이어 형태 `input.player("Map").action("Name").accessor`의 경우 키를 `"Map/Name"`으로 구성한다.

---

## 13 진단

컴파일러는 다음 진단 코드를 내보내야 한다. 각 코드는 컴파일러 버전 간에 안정적이다.

### 13.1 오류

| 코드 | 메시지 | 조건 |
|------|---------|-----------|
| E000 | `Cannot read source file: {path}` | 컴파일 중 I/O 오류. 소스 파일을 열거나 읽을 수 없음. |
| E012 | `Lifecycle block '{name}' is only valid inside a component declaration` | 생명주기 블록(`update`, `awake` 등)이 `component` 외부에 나타남. |
| E013 | `'{qualifier}' fields are only valid inside a component declaration` | `require`, `optional`, `child`, `parent` 필드가 `component` 외부에 나타남. |
| E014 | `Duplicate lifecycle block '{name}'; only one per component is allowed` | 생명주기 블록이 단일 component에서 두 번 이상 나타남. |
| E020 | `Type mismatch: expected '{expected}', found '{found}'` | 표현식이 주변 컨텍스트와 호환되지 않는 타입을 생성함. |
| E022 | `Variable '{name}' must have a type annotation or an initializer` | `val` 또는 `var` 선언에 타입 주석도 이니셜라이저도 없음. |
| E031 | `'{keyword}' can only be used inside a loop` | `break` 또는 `continue`가 `for` 또는 `while` 본문 외부에 나타남. |
| E032 | `'wait' can only be used inside a coroutine` | `wait` 문이 `coroutine` 선언 외부에 나타남. |
| E040 | `Cannot assign to immutable value '{name}'` | 초기화 후 `val` 바인딩에 대입. |
| E041 | `Cannot assign to 'require' field '{name}'` | `Awake()`에서 한 번 해석되는 `require` 필드에 대입. |
| E050 | `Enum '{name}' must have at least one entry` | 항목이 없는 enum 선언. |
| E051 | `Enum entry '{entry}' expects {expected} argument(s), but {found} given` | 페이로드 enum 값 생성 시 인자 수 불일치. |
| E052 | `Duplicate enum entry '{name}'` | 같은 enum 내 두 항목이 이름을 공유. |
| E060 | `Coroutines are only valid inside a component declaration` | `asset` 또는 `class`에서 `coroutine`이 선언됨. |
| E070 | `Input System sugar requires the 'input-system' feature flag` | `.prsmproject`에 `features = ["input-system"]` 없이 입력 편의 구문 사용. |
| E081 | `Unknown variant '{variant}' for enum '{enum}'` | `when` 분기가 존재하지 않는 enum 변형을 참조. |
| E082 | `Pattern for '{variant}' expects {expected} binding(s), found {found}` | 구조 분해 패턴의 바인딩 수가 변형의 매개변수 수와 불일치. |
| E083 | `Listen lifetime modifier is only valid inside a component` | listen 수명 수정자(`until disable`, `until destroy`, `manual`)가 `component` 외부에 나타남. |
| E100 | `Syntax error: {details}` | 파서 오류 총괄 -- 누락된 식별자, 짝이 맞지 않는 중괄호, 잘못된 위치의 키워드. |

E081, E082, E083은 (PrSM 2 부터) 추가.

### 13.2 경고

| 코드 | 메시지 | 조건 |
|------|---------|-----------|
| W001 | `Unnecessary '!!' on non-nullable type '{type}'` | 이미 non-nullable인 타입의 표현식에 `!!`가 적용됨. |
| W003 | `'when' does not cover all variants of '{enum}'; missing: {variants}` | enum에 대한 `when` 문이 변형을 누락하고 `else` 분기가 없음. |
| W005 | `Data class '{name}' has no fields` | 빈 매개변수 목록으로 `data class`가 선언됨. |

### 13.3 진단 형식

컴파일러는 다음 형식으로 진단을 내보내야 한다:

```
severity[CODE]: message
  --> file_path:line:column
   |
NN |     offending source line
   |     ^^^^^^^^^^^^^^^^^^^^^^
```

---

## 14 문법

PrSM의 규범적 문법은 확장 배커스-나우르 형식(EBNF)으로 정의된다. 전체 문법은 별도의 문서로 유지 관리된다.

전체 EBNF 정의는 [공식 문법](../grammar.md)을 참조하라. 파일 구조, 선언, 멤버, 문장, 표현식, 패턴, 터미널 토큰을 포괄한다.

문법 문서는 파서 동작에 대해 권위적이다. 이 표준의 산문과 문법이 충돌하는 경우 문법이 우선한다.
