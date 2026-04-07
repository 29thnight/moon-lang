---
title: Functions
parent: 언어 가이드
grand_parent: 한국어 문서
nav_order: 8
---

# Functions

PrSM의 함수는 `func`로 선언되며 최상위 선언의 멤버로 존재합니다.

## 블록 본문 함수

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

## 표현식 본문 함수

본문이 단일 식인 함수는 `=`을 사용합니다:

```prsm
func isDead(): Bool = hp <= 0
func label(): String = "HP: $hp"
```

## 반환 타입

반환 타입은 명시적이며 `:` 뒤에 옵니다:

```prsm
func getCurrentSpeed(): Float {
    return rb.velocity.magnitude
}
```

`Unit` 반환 타입의 함수는 주석을 생략할 수 있습니다.

## 가시성 수정자

- `public` — 다른 C# 코드에서 접근 가능 (대부분의 멤버 기본값)
- `private` — component 클래스로 스코프 제한
- `protected` — 서브클래스에서 접근 가능

```prsm
private func handleInput() {
    // ...
}

public func TakeDamage(amount: Int) {
    hp -= amount
}
```

## Override

기본 클래스 메서드를 오버라이드하는 함수에는 `override`를 사용합니다:

```prsm
override func ToString(): String = "Player[$name]"
```

## 매개변수

모든 매개변수는 명시적 타입을 가진 위치 기반입니다:

```prsm
func move(direction: Vector3, speed: Float) {
    transform.Translate(direction * speed * Time.deltaTime)
}
```

## 람다식 (PrSM 4 부터)

람다는 `{ }`로 둘러싸인 익명 함수입니다. 단일 매개변수 람다는 암묵적 `it` 식별자를 사용할 수 있습니다. 호출의 마지막 인자가 람다이면 괄호 밖으로 빼낼 수 있습니다 (후행 람다).

```prsm
val callback: (Int) => Unit = { x => log(x) }
val add: (Int, Int) => Int = { a, b => a + b }

list.filter { it > 10 }
list.where({ x => x > 10 }).select({ x => x * 2 })
```

`(A, B) => R`은 `Func<A, B, R>`로, `() => Unit`은 `Action`으로 변환됩니다. 클로저 캡처는 C# 람다와 동일한 참조 의미를 따릅니다.

## `static` 및 `const` 멤버 (PrSM 4 부터)

`static`은 인스턴스 없이 접근 가능한 멤버를 선언합니다. `const`는 초기화자가 리터럴이어야 하는 컴파일 타임 상수를 선언합니다.

```prsm
class MathHelper {
    static val PI: Float = 3.14159
    static func lerp(a: Float, b: Float, t: Float): Float = a + (b - a) * t
}

const MAX_HEALTH: Int = 100
const VERSION: String = "1.0.0"
```

`static`은 라이프사이클 블록에 허용되지 않습니다 (E106). `const` 초기화자는 리터럴이어야 합니다 (E105).

## `abstract` / `open` / `override` (PrSM 4 부터)

기본적으로 클래스와 메서드는 final입니다. 수정자가 상속을 제어합니다:

| 수정자 | 효과 |
|---|---|
| `open` | 상속 / 오버라이드 허용 |
| `abstract` | 서브클래스 구현 요구; 인스턴스화 불가 |
| `sealed` | 서브클래스를 같은 파일로 제한 (`when` 완전성 활성화) |
| `override` | 부모 `open`/`abstract` 메서드를 대체하는 메서드 표시 |

```prsm
abstract class Weapon {
    abstract func attack()
    open func reload() { }
}

class Sword : Weapon {
    override func attack() { swing() }
}
```

일치하는 부모 메서드 없는 `override`는 E114를 발생시킵니다. `abstract` 클래스의 인스턴스화는 E116을 발생시킵니다.

## `async` / `await` (PrSM 4 부터)

`async func`는 비동기 함수를 선언합니다. `await`는 awaited 태스크가 완료될 때까지 일시 중단합니다. 컴파일러는 Unity 컨텍스트에서 UniTask를 우선하며, 사용 불가 시 `Task`로 폴백합니다.

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

`Unit`을 반환하는 `async` 함수는 `UniTask`로 변환됩니다. `async func` 외부의 `await`는 E135를 발생시킵니다.

## 연산자 오버로딩 (PrSM 4 부터)

연산자 함수는 기호 연산자를 명명된 메서드에 매핑합니다. PrSM은 Kotlin 규칙을 따릅니다: `plus`, `minus`, `times`, `div`, `mod`, `compareTo`, `equals`, `unaryMinus`, `not`.

```prsm
data class Vec2i(x: Int, y: Int) {
    operator plus(other: Vec2i): Vec2i = Vec2i(x + other.x, y + other.y)
    operator minus(other: Vec2i): Vec2i = Vec2i(x - other.x, y - other.y)
}

val c = Vec2i(1, 2) + Vec2i(3, 4)  // Vec2i(4, 6)
```

`operator get`과 `operator set`은 `[]` 문법을 위한 인덱서 접근을 정의합니다.
