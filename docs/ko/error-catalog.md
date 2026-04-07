---
title: 오류 카탈로그
parent: 언어 가이드
grand_parent: 한국어 문서
nav_order: 12
---

# 오류 카탈로그

PrSM 컴파일러가 출력하는 모든 진단 메시지에는 고정된 코드가 부여됩니다. 이 페이지에서는 모든 코드, 심각도, 메시지 텍스트, 그리고 근본 원인을 해결하는 방법을 안내합니다.

---

## 오류(Error)

### E000 -- 컴파일 중 I/O 오류

**심각도:** Error
**메시지:** `Cannot read source file: {path}`
**설명:** 컴파일러가 `.prsm` 소스 파일을 열거나 읽을 수 없었습니다. 일반적으로 파일 목록이 확정된 이후에 파일이 삭제, 이동되었거나 다른 프로세스가 파일을 잠근 경우에 발생합니다.
**해결 방법:** 파일이 존재하고 잠겨 있지 않은지 확인하세요. `.prsmproject`의 include/exclude 패턴에 오래된 항목이 없는지 점검하세요.

---

### E012 -- 잘못된 컨텍스트의 라이프사이클 블록

**심각도:** Error
**메시지:** `Lifecycle block '{name}' is only valid inside a component declaration`
**설명:** `update`나 `awake` 같은 라이프사이클 블록은 `component` 본문 안에서만 사용할 수 있습니다. `asset`, `class` 또는 다른 선언에서는 유효하지 않습니다.

```prsm
// E012 발생
asset GameConfig : ScriptableObject {
    update {
        tick()
    }
}
```

**해결 방법:** 라이프사이클 블록을 `component`로 이동하거나, 프레임 콜백이 필요하다면 해당 선언을 `component`로 변환하세요.

---

### E013 -- 잘못된 컨텍스트의 component 전용 필드 한정자

**심각도:** Error
**메시지:** `'{qualifier}' fields are only valid inside a component declaration`
**설명:** `require`, `optional`, `child`, `parent` 필드 한정자는 `Awake()`에서 생성되는 `GetComponent` 조회에 의존합니다. 이들은 `component` 안에서만 의미가 있습니다.

```prsm
// E013 발생
class Utility {
    require rb: Rigidbody
}
```

**해결 방법:** 일반 `val` 또는 `var` 필드를 대신 사용하거나, 선언을 `component`로 변경하세요.

---

### E014 -- 중복된 라이프사이클 블록

**심각도:** Error
**메시지:** `Duplicate lifecycle block '{name}'; only one per component is allowed`
**설명:** 각 라이프사이클 블록은 component당 최대 한 번만 나타날 수 있습니다. 컴파일러는 블록을 하나의 Unity 메서드로 병합하여 생성하므로 중복을 처리할 수 없습니다.

```prsm
component Player : MonoBehaviour {
    update { movePlayer() }
    update { rotatePlayer() }  // E014
}
```

**해결 방법:** 로직을 하나의 라이프사이클 블록으로 합치거나, 한쪽을 헬퍼 함수로 분리하세요.

---

### E020 -- 타입 불일치

**심각도:** Error
**메시지:** `Type mismatch: expected '{expected}', found '{found}'`
**설명:** 식이 주변 컨텍스트가 요구하는 타입과 일치하지 않는 타입을 생성했습니다.

```prsm
component Demo : MonoBehaviour {
    serialize speed: Float = "fast"  // E020: Float 예상, String 발견
}
```

**해결 방법:** 식이 예상 타입을 생성하도록 변경하거나, 타입 어노테이션을 수정하세요.

---

### E022 -- 타입과 초기화 값이 모두 없는 변수

**심각도:** Error
**메시지:** `Variable '{name}' must have a type annotation or an initializer`
**설명:** PrSM은 모든 변수의 타입을 추론하기에 충분한 정보를 요구합니다. 타입도 초기값도 없는 선언은 모호합니다.

```prsm
func demo() {
    val x  // E022: 타입 없음, 초기화 값 없음
}
```

**해결 방법:** 타입 어노테이션(`val x: Int`)이나 초기화 값(`val x = 0`), 또는 둘 다 추가하세요.

---

### E031 -- 루프 외부의 break/continue

**심각도:** Error
**메시지:** `'{keyword}' can only be used inside a loop`
**설명:** `break`와 `continue`는 `for` 또는 `while` 본문 안에서만 사용해야 합니다.

```prsm
func demo() {
    break  // E031
}
```

**해결 방법:** 해당 문을 루프 안으로 이동하거나, 함수를 종료하려면 `return`을 사용하세요.

---

### E032 -- 코루틴 외부의 wait

**심각도:** Error
**메시지:** `'wait' can only be used inside a coroutine`
**설명:** `wait`는 `yield return`으로 변환되며 `coroutine` 선언 안에서만 유효합니다.

```prsm
func fire() {
    wait 1.0s  // E032
}
```

**해결 방법:** `func`를 `coroutine`으로 변경하거나, `wait`를 제거하고 다른 타이밍 전략을 사용하세요.

---

### E040 -- 불변 val에 대한 대입

**심각도:** Error
**메시지:** `Cannot assign to immutable value '{name}'`
**설명:** `val` 바인딩은 초기화 이후 불변입니다. 재대입을 시도하면 오류가 발생합니다.

```prsm
func demo() {
    val hp = 100
    hp = 50  // E040
}
```

**해결 방법:** 값이 변경되어야 한다면 선언을 `val`에서 `var`로 변경하세요.

---

### E041 -- require 필드에 대한 대입

**심각도:** Error
**메시지:** `Cannot assign to 'require' field '{name}'`
**설명:** `require` 필드는 `Awake()`에서 한 번 해석되며 component의 수명 동안 불변으로 취급됩니다.

```prsm
component Demo : MonoBehaviour {
    require rb: Rigidbody

    func reset() {
        rb = null  // E041
    }
}
```

**해결 방법:** 런타임에 참조를 변경해야 한다면 `require` 대신 `optional`을 사용하세요.

---

### E050 -- 빈 enum

**심각도:** Error
**메시지:** `Enum '{name}' must have at least one entry`
**설명:** 항목이 없는 enum은 유효하지 않습니다. 컴파일러는 C# enum을 생성하기 위해 최소 하나의 변형이 필요합니다.

```prsm
enum Status {}  // E050
```

**해결 방법:** enum 본문에 최소 하나의 항목을 추가하세요.

---

### E051 -- enum 항목 인수 개수 불일치

**심각도:** Error
**메시지:** `Enum entry '{entry}' expects {expected} argument(s), but {found} given`
**설명:** 페이로드를 가진 enum 값을 생성할 때, 인수의 수가 항목 정의와 일치해야 합니다.

```prsm
enum Result {
    Ok(Int),
    Err(String)
}

func demo() {
    val r = Result.Ok(1, 2)  // E051: Ok는 1개 예상, 2개 제공
}
```

**해결 방법:** enum 항목에 선언된 인수 수와 정확히 일치하도록 전달하세요.

---

### E052 -- 중복된 enum 항목 이름

**심각도:** Error
**메시지:** `Duplicate enum entry '{name}'`
**설명:** 하나의 enum 내에서 각 항목은 고유한 이름을 가져야 합니다.

```prsm
enum Direction {
    Up,
    Down,
    Up  // E052
}
```

**해결 방법:** 중복된 항목의 이름을 변경하거나 제거하세요.

---

### E060 -- component가 아닌 선언에서의 코루틴

**심각도:** Error
**메시지:** `Coroutines are only valid inside a component declaration`
**설명:** 코루틴은 `StartCoroutine` 호출로 변환되며 `MonoBehaviour` 컨텍스트가 필요합니다. `asset`이나 `class` 본문에서는 사용할 수 없습니다.

```prsm
class Utility {
    coroutine delay() {  // E060
        wait 1.0s
    }
}
```

**해결 방법:** 코루틴을 `component`로 이동하거나, 콜백 패턴을 사용하는 일반 함수를 사용하세요.

---

### E070 -- 기능 플래그 없이 Input System 편의 구문 사용

**심각도:** Error
**메시지:** `Input System sugar requires the 'input-system' feature flag`
**설명:** 입력 바인딩 축약 구문은 `.prsmproject`에서 활성화해야 하는 기능 플래그로 제어됩니다.

**해결 방법:** `.prsmproject` 파일의 `language.features` 배열에 `"input-system"`을 추가하세요.

---

### E081 -- 패턴에서 알 수 없는 enum 변형

**심각도:** Error
**메시지:** `Unknown variant '{variant}' for enum '{enum}'`
**설명:** `when` 분기에서 enum 정의에 존재하지 않는 변형을 참조하고 있습니다.

```prsm
enum State { Idle, Running }

func demo(s: State) {
    when s {
        State.Idle    => idle()
        State.Flying  => fly()  // E081: Flying은 State에 없음
    }
}
```

**해결 방법:** 오타가 없는지 확인하고 변형 이름이 enum 정의와 일치하는지 검증하세요.

---

### E082 -- 패턴 바인딩 개수 불일치

**심각도:** Error
**메시지:** `Pattern for '{variant}' expects {expected} binding(s), found {found}`
**설명:** 페이로드 enum 항목을 구조 분해할 때, 항목이 선언한 것과 동일한 수의 값을 바인딩해야 합니다.

```prsm
enum Result { Ok(Int), Err(String) }

func demo(r: Result) {
    when r {
        Result.Ok(val a, val b) => log(a)  // E082: Ok는 필드 1개, 바인딩 2개
        Result.Err(val msg)     => log(msg)
    }
}
```

**해결 방법:** 바인딩 수를 enum 항목의 페이로드 수와 일치시키세요.

---

### E083 -- 잘못된 컨텍스트의 listen 수명 한정자

**심각도:** Error
**메시지:** `Listen lifetime modifier is only valid inside a component`
**설명:** `.once`와 `.whileEnabled` listen 수명 한정자는 정리를 관리하기 위해 component 라이프사이클 훅에 의존합니다. `asset`이나 `class` 본문에서는 사용할 수 없습니다.

**해결 방법:** `listen` 문을 `component`로 이동하거나, 이벤트를 수동으로 연결하세요.

---

### E100 -- 파서 / 구문 오류

**심각도:** Error
**메시지:** `Syntax error: {details}`
**설명:** 파서가 예상하지 못한 토큰을 만났습니다. 이것은 잘못된 소스 텍스트에 대한 범용 오류입니다.

```prsm
component Demo : MonoBehaviour {
    func () { }  // E100: 'func' 뒤에 식별자가 예상됨
}
```

**해결 방법:** 진단에 표시된 줄에서 누락된 식별자, 짝이 맞지 않는 중괄호, 잘못 배치된 키워드가 없는지 확인하세요.

---

### E090 -- Interface 멤버가 구현되지 않음 (PrSM 3 부터)

**심각도:** Error
**메시지:** `Interface member '{name}' is not implemented`
**설명:** component 또는 class가 interface를 구현한다고 선언했지만 필수 멤버에 대한 구현을 제공하지 않았습니다.

```prsm
interface IDamageable {
    func takeDamage(amount: Int)
    val isAlive: Bool
}

component Enemy : MonoBehaviour, IDamageable {
    var hp: Int = 100
    // E090: takeDamage와 isAlive 누락
}
```

**해결 방법:** interface 계약을 만족하도록 누락된 메서드 또는 프로퍼티를 추가하세요.

---

### E091 -- Interface 멤버에 구현 본문이 있음 (PrSM 3 부터)

**심각도:** Error
**메시지:** `Interface members shall not have implementation bodies`
**설명:** Interface 멤버는 시그니처만 가집니다. Interface 선언 내부에 본문을 제공하는 것은 허용되지 않습니다.

```prsm
interface IDamageable {
    func takeDamage(amount: Int) { }  // E091
}
```

**해결 방법:** 본문을 제거하세요 -- interface 멤버는 시그니처만 가집니다.

---

### E095 -- 제네릭 타입 제약 위반 (PrSM 3 부터)

**심각도:** Error
**메시지:** `Type argument '{T}' does not satisfy constraint '{constraint}'`
**설명:** 제네릭 타입 파라미터가 선언된 제약을 충족하지 않는 타입으로 인스턴스화되었습니다.

```prsm
class Registry<T> where T : Component {
    var items: List<T> = []
}

val r = Registry<String>()  // E095: String은 Component를 충족하지 않음
```

**해결 방법:** 제약을 충족하는 타입을 사용하세요.

---

### E096 -- 지원되지 않는 선언에 제네릭 파라미터 사용 (PrSM 3 부터)

**심각도:** Error
**메시지:** `Generic type parameters cannot be declared on component/asset/enum/data class`
**설명:** 제네릭 타입 파라미터는 `class`와 `func` 선언에서만 지원됩니다. Unity 직렬화 타입(`component`, `asset`)과 값 유사 타입(`enum`, `data class`)은 제네릭이 될 수 없습니다.

```prsm
component Foo<T> : MonoBehaviour { }  // E096
```

**해결 방법:** 제네릭은 `class`와 `func`에서만 사용하세요.

---

### E097 -- component가 아닌 곳에 singleton 사용 (PrSM 3 부터)

**심각도:** Error
**메시지:** `'singleton' can only be used before 'component'`
**설명:** `singleton` 수정자는 `component`에서만 의미 있는 Unity 전용 `Awake` 및 `DontDestroyOnLoad` 코드를 생성합니다.

```prsm
singleton class Foo { }  // E097
```

**해결 방법:** `singleton`은 `component`에서만 사용하세요.

---

### E098 -- Pool에 프리팹 없음 (PrSM 3 부터)

**심각도:** Error
**메시지:** `Pool type '{T}' has no matching serialize prefab field`
**설명:** `pool` 선언은 프리팹 소스로 사용할 동일한 타입의 `serialize` 필드가 필요합니다.

```prsm
component Spawner : MonoBehaviour {
    pool bullets: Bullet(capacity = 20, max = 100)  // E098: serialize Bullet 필드 없음
}
```

**해결 방법:** 풀의 타입과 일치하는 `serialize` 필드를 추가하세요 (예: `serialize bulletPrefab: Bullet`).

---

### E099 -- component 외부의 pool (PrSM 3 부터)

**심각도:** Error
**메시지:** `'pool' is only valid inside a component declaration`
**설명:** 오브젝트 풀은 Unity 라이프사이클 훅에 의존하므로 `component` 본문 안에서만 유효합니다.

```prsm
class Utility {
    pool items: Bullet(capacity = 10, max = 50)  // E099
}
```

**해결 방법:** `pool` 선언을 `component`로 이동하세요.

---

### E101 -- 예약된 내장 메서드 이름 (PrSM 3 부터)

**심각도:** Error
**메시지:** `'{name}' is a reserved built-in method name`
**설명:** 특정 메서드 이름은 컴파일러가 생성 코드를 위해 예약하고 있어 사용자 정의 함수 이름으로 사용할 수 없습니다.

```prsm
component Demo : MonoBehaviour {
    func get() { }  // E101
}
```

**해결 방법:** 함수 이름을 변경하세요 (예: `getData`, `findTarget`).

---

## 경고(Warning)

### W001 -- 불필요한 non-null 단언

**심각도:** Warning
**메시지:** `Unnecessary '!!' on non-nullable type '{type}'`
**설명:** 이미 non-nullable인 타입의 값에 `!!`를 적용하면 아무런 효과가 없습니다.

```prsm
val x: Int = 10
val y = x!!  // W001: Int는 이미 non-nullable
```

**해결 방법:** `!!` 연산자를 제거하세요.

---

### W003 -- 불완전한 when 패턴

**심각도:** Warning
**메시지:** `'when' does not cover all variants of '{enum}'; missing: {variants}`
**설명:** enum에 대한 `when` 식이 모든 변형을 나열하지 않았고 `else` 분기도 없습니다. 런타임에 일치하지 않는 값은 조용히 통과됩니다.

```prsm
enum Dir { Up, Down, Left, Right }

func demo(d: Dir) {
    when d {
        Dir.Up   => moveUp()
        Dir.Down => moveDown()
        // W003: Left, Right 누락
    }
}
```

**해결 방법:** 누락된 변형에 대한 분기를 추가하거나, `else` 분기를 추가하세요.

---

### W005 -- 필드가 없는 data class

**심각도:** Warning
**메시지:** `Data class '{name}' has no fields`
**설명:** 빈 매개변수 목록을 가진 `data class`는 기술적으로 유효하지만 거의 확실히 의도하지 않은 것입니다.

```prsm
data class Empty()  // W005
```

**해결 방법:** 매개변수 목록에 필드를 추가하거나, 사용하지 않는다면 data class를 제거하세요.

---

### W010 -- public 메서드가 너무 많음 (PrSM 3 부터)

**심각도:** Warning
**메시지:** `Component '{name}' has {n} public methods. Consider splitting responsibilities.`
**설명:** public 메서드가 8개 이상인 component는 너무 많은 책임을 맡고 있을 수 있습니다. 이 경고는 SOLID 단일 책임 원칙 준수를 권장합니다.

**해결 방법:** component를 명확한 책임을 가진 더 작고 집중된 component로 분리하세요.

---

### W011 -- 의존성이 너무 많음 (PrSM 3 부터)

**심각도:** Warning
**메시지:** `Component '{name}' has {n} dependency fields. Consider reducing dependencies.`
**설명:** `require`/`optional`/`child`/`parent` 필드가 6개 이상인 component는 의존성이 너무 많을 수 있습니다. 이 경고는 SOLID 의존성 역전 원칙 준수를 권장합니다.

**해결 방법:** 중간 component를 추출하거나 이벤트를 사용하여 의존성 필드 수를 줄이세요.

---

### W012 -- 메서드가 너무 긺 (PrSM 3 부터)

**심각도:** Warning
**메시지:** `Method '{name}' has {n} statements. Consider extracting helper methods.`
**설명:** 50개 이상의 구문을 가진 메서드 또는 라이프사이클 블록은 읽고 유지하기 어렵습니다. 이 경고는 SOLID 단일 책임 원칙 준수를 권장합니다.

**해결 방법:** 로직을 더 작은 헬퍼 메서드로 분리하세요.

---

## 언어 4 진단 (PrSM 4 부터)

PrSM 4는 예외, 람다, 수정자, 캐스트, 프로퍼티, 구조체, 비동기, Burst, v4 패턴 편의 기능을 다루는 47개의 새로운 진단 코드 (E100–E146, W020–W032)를 도입합니다. 각 코드의 조건은 아래에 요약되어 있습니다.

### 오류

| 코드 | 기능 | 조건 |
|------|------|------|
| E100 | try/catch | 도달 불가능한 catch 절 (위에서 슈퍼타입이 잡힘) |
| E101 | throw | throw 표현식이 Exception 서브타입이 아님 |
| E102 | Lambda | 예상 함수 타입과 람다 매개변수 개수 불일치 |
| E103 | Lambda | 컨텍스트 없이 람다 매개변수 타입 추론 불가 |
| E104 | Lambda | 다중 매개변수 람다에 `it` 사용 |
| E105 | const | `const`의 비리터럴 초기화자 |
| E106 | static | 라이프사이클 블록에 `static` |
| E107 | Collection | 타입 주석 없는 빈 컬렉션 리터럴 |
| E108 | Collection | 컬렉션 리터럴의 혼합된 원소 타입 |
| E109 | Cast | 명백히 무관한 타입으로의 `as!` |
| E110 | Property | `val` 프로퍼티에 `set` |
| E111 | Property | 프로퍼티 접근자 외부의 `field` |
| E112 | extend | 확장 멤버가 타깃 타입의 기존 멤버와 충돌 |
| E113 | extend | 확장 블록이 상태 (`var` 필드)를 포함 |
| E114 | Modifier | 일치하는 `open`/`abstract` 부모 없는 `override` |
| E115 | sealed | sealed 클래스에 비완전 `when` |
| E116 | abstract | `abstract` 클래스의 인스턴스화 |
| E117 | Tuple | 튜플 구조 분해 개수 불일치 |
| E118 | Tuple | 잘못된 라벨로 명명 튜플 필드 접근 |
| E119 | use | 타입이 `IDisposable`을 구현하지 않음 |
| E120 | struct | 클래스를 상속하는 구조체 |
| E121 | event | 선언 타입 외부에서 이벤트 호출 |
| E122 | event | 비-Unit 반환 타입의 이벤트 |
| E123 | Operator | `operator` 선언의 알 수 없는 연산자 이름 |
| E124 | Operator | 일치하는 `GetHashCode` 오버라이드 없는 `operator equals` |
| E125 | Indexer | 가변 인덱서에 `operator set` 없는 `operator get` |
| E126 | typealias | 순환 참조를 만드는 typealias |
| E127 | typealias | 내장 타입 이름을 가리는 typealias |
| E128 | Raw string | 종료되지 않은 원시 문자열 리터럴 |
| E129 | in | `Contains`/`ContainsKey` 없는 타입에 `in` 사용 |
| E130 | OR pattern | 다른 변수를 바인딩하는 OR 패턴 분기 |
| E131 | Range pattern | low > high인 범위 패턴 |
| E132 | ?:= | non-nullable 변수에 null 병합 대입 |
| E133 | ?:= | `val`(불변)에 null 병합 대입 |
| E134 | Interface | 타깃 C# 버전에서 사용 불가능한 디폴트 인터페이스 메서드 기능 |
| E135 | async | `async func` 외부의 `await` |
| E136 | async | awaitable이 아닌 반환 타입의 `async func` |
| E137 | Burst | `@burst` 메서드의 관리 타입 참조 |
| E138 | Burst | `@burst` 메서드의 `try`/`catch` |
| E139 | Burst | `@burst` 컨텍스트의 가상 또는 인터페이스 호출 |
| E140 | State machine | 선언되지 않은 상태로의 전이 |
| E141 | State machine | 중복된 상태 이름 |
| E142 | Command | `undo`가 `execute`에서 캡처되지 않은 변수 참조 |
| E143 | bind | `bind to` 타깃이 쓰기 가능 프로퍼티가 아님 |
| E144 | bind | 소스와 타깃 사이 `bind` 타입 불일치 |
| E145 | DX | 조건 표현식으로 사용된 대입 |
| E146 | DX | 일반적인 API 오용 감지 |

### 경고

| 코드 | 기능 | 조건 |
|------|------|------|
| W020 | try/catch | 빈 `catch` 블록 |
| W021 | Cast | `as?` 결과가 null 검사되지 않음 |
| W022 | struct | 큰 구조체 (16바이트 초과) |
| W023 | Range pattern | `when`에서 겹치는 범위 패턴 |
| W024 | Interface | 디폴트 인터페이스 메서드가 인스턴스 상태 참조 |
| W025 | async | `async func`가 `await`를 사용하지 않음 |
| W026 | Optimizer | 핫 패스의 문자열 할당 (`opt.string`이 캐시) |
| W027 | Optimizer | `Update`/`FixedUpdate`의 LINQ 체인 (`opt.linq`이 재작성) |
| W028 | Burst | `@burst` 메서드에서 박싱 감지 |
| W029 | State machine | 나가는 전이가 없는 상태 (죽은 상태) |
| W030 | Command | `canExecute` 가드 없이 선언된 `command` |
| W031 | bind | 읽히지 않는 `bind` 프로퍼티 |
| W032 | Debugger | 파일에 대한 소스 맵 생성 실패 (치명적 아님) |
