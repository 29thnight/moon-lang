# Moon v1 최소 문법 스펙

## 1. 파일 구조

```
using <namespace>

<declaration>
```

- `using`: .NET 네임스페이스 임포트
- 파일당 하나의 최상위 선언
- 파일명 = 선언 이름
- 세미콜론 없음, 줄바꿈이 문장 종결

## 2. 선언

### component
```
component Name : MonoBehaviour {
    <members>
}
```
→ `public class Name : MonoBehaviour`

### asset
```
asset Name : ScriptableObject {
    <members>
}
```
→ `[CreateAssetMenu] public class Name : ScriptableObject`

### class
```
class Name {
    <members>
}

class Name : BaseClass {
    <members>
}
```
→ `public class Name`

### data class
```
data class Name(
    val field1: Type,
    val field2: Type
)
```
→ `[System.Serializable] public class Name` + Equals/GetHashCode/ToString

### enum
```
enum Name {
    Entry1,
    Entry2
}

// 파라미터 있는 enum
enum Name(val param: Type) {
    Entry1(value),
    Entry2(value)
}
```
→ `public enum Name { Entry1, Entry2 }` (파라미터 있으면 + 확장 메서드)

## 3. 타입

| Moon | C# |
|---|---|
| `Int` | `int` |
| `Float` | `float` |
| `Double` | `double` |
| `Bool` | `bool` |
| `String` | `string` |
| `Long` | `long` |
| `Byte` | `byte` |
| `Unit` | `void` |

Unity 타입: `Vector2`, `Vector3`, `Quaternion`, `GameObject`, `Transform` 등 그대로 사용.

편의 sugar:
| Sugar | C# |
|---|---|
| `vec2(x, y)` | `new Vector2(x, y)` |
| `vec3(x, y, z)` | `new Vector3(x, y, z)` |
| `color(r, g, b, a)` | `new Color(r, g, b, a)` |
| `quat.euler(x, y, z)` | `Quaternion.Euler(x, y, z)` |

## 4. 필드 모델

### serialize (직렬화 필드)
```
serialize name: Type = value
```
→ `[SerializeField] private Type name = value;`

접근 제한자 허용:
```
public serialize name: Type = value
private serialize name: Type = value
```

어노테이션:
```
@header("Section")
@range(0, 100)
@tooltip("Description")
serialize name: Type = value
```

### 일반 필드
```
private name: Type          // non-null, 초기화 필수
private name: Type?         // nullable
var name: Type = value      // mutable
val name: Type = value      // immutable
```

### 컴포넌트 룩업 (component 전용)
```
require name: Type      // Awake에서 GetComponent, null이면 에러+disable
optional name: Type     // Awake에서 GetComponent, nullable
child name: Type        // GetComponentInChildren + null 체크
parent name: Type       // GetComponentInParent + null 체크
```

## 5. 함수

```
func name(param: Type): ReturnType {
    body
}

// expression body
func name(): ReturnType = expression

// void 반환 (Unit)
func name() {
    body
}

// 접근 제한자
private func name() { }
protected func name() { }
```

## 6. Lifecycle 블록

```
awake { }
start { }
update { }
fixedUpdate { }
lateUpdate { }
onEnable { }
onDisable { }
onDestroy { }

// 매개변수 있는 lifecycle
onTriggerEnter(other: Collider) { }
onTriggerExit(other: Collider) { }
onTriggerStay(other: Collider) { }
onCollisionEnter(collision: Collision) { }
onCollisionExit(collision: Collision) { }
onCollisionStay(collision: Collision) { }
```

규칙:
- component 내부에서만 사용 가능
- 각 lifecycle 블록은 컴포넌트당 최대 1회
- require/optional 해결 후 awake 블록 실행

## 7. 제어문

### if (괄호 없음)
```
if condition {
    body
} else if condition {
    body
} else {
    body
}

// 표현식
val x = if a > b { a } else { b }
```

### when
```
when subject {
    Pattern => body
    Pattern => body
    else => body
}

// 조건 기반 (subject 없음)
when {
    condition => body
    else => body
}
```
when 화살표: `=>`

### for (괄호 없음)
```
for item in collection { body }
for i in 0 until n { body }
for i in 0..n { body }          // inclusive
for i in n downTo 0 { body }
```

### while (괄호 없음)
```
while condition { body }
```

`break`, `continue` 지원.

## 8. Null Safety

```
Type    // non-null
Type?   // nullable

x?.member       // safe call
x ?: default    // null coalescing (Elvis operator)
x!!             // non-null assertion

// early return
val x = expr ?: return
```

smart cast: null 체크 후 non-null로 타입 축소 (val 로컬만).

## 9. Component Lookup Sugar

함수형:
| Sugar | C# |
|---|---|
| `get<T>()` | `GetComponent<T>()` |
| `require<T>()` | `GetComponent<T>()` + null 에러 |
| `find<T>()` | `FindFirstObjectByType<T>()` |
| `child<T>()` | `GetComponentInChildren<T>()` |
| `parent<T>()` | `GetComponentInParent<T>()` |

## 10. Coroutine

```
coroutine name(params) {
    body with wait statements
}
```

### wait 형식
| Moon | C# |
|---|---|
| `wait 1.0s` | `yield return new WaitForSeconds(1.0f)` |
| `wait nextFrame` | `yield return null` |
| `wait fixedFrame` | `yield return new WaitForFixedUpdate()` |
| `wait until condition` | `yield return new WaitUntil(() => condition)` |
| `wait while condition` | `yield return new WaitWhile(() => condition)` |

### 실행/중지
```
start coroutineName()
stop coroutineName()
stopAll()
```

## 11. Event/Listener Sugar

```
listen event.name {
    body
}

listen event.name { param ->
    body using param
}
```
→ `event.name.AddListener((...) => { body });`

## 12. Intrinsic (이스케이프 해치)

```
// 문장 블록 — raw C# 삽입
intrinsic {
    C# code here
}

// 표현식 — T 타입 반환
val x = intrinsic<T> {
    C# expression
}

// 함수 전체
intrinsic func name(params): ReturnType {
    C# body
}

// 코루틴 전체
intrinsic coroutine name(params) {
    C# body
}
```

규칙:
- 내부 코드는 Moon가 파싱하지 않음 (중괄호 매칭만)
- 타입 체크, null safety 미적용
- 컴파일러 경고 발생

## 13. Input Sugar

| Moon | C# |
|---|---|
| `input.axis("name")` | `Input.GetAxis("name")` |
| `input.getKey(KeyCode.X)` | `Input.GetKey(KeyCode.X)` |
| `input.getKeyDown(KeyCode.X)` | `Input.GetKeyDown(KeyCode.X)` |
| `input.getButton("name")` | `Input.GetButton("name")` |
| `input.getButtonDown("name")` | `Input.GetButtonDown("name")` |

## 14. 문자열 보간

```
"text ${expression} more text"
"simple $variable"
```
→ `$"text {expression} more text"`

## 15. v1 명시적 배제

- 제네릭 선언 (호출만 가능)
- 확장 함수 / 연산자 오버로딩
- abstract / sealed 클래스
- 커스텀 프로퍼티 (get/set)
- 구조 분해
- 완전한 람다 (listen/wait에서만 제한적)
- 고차 함수 / async/await
- try-catch-finally
- 모듈/패키지 / 타입 별칭
- 상태 머신 DSL / DOTS/ECS
- do-while
