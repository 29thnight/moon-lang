---
title: Input System
parent: 언어 가이드
grand_parent: 한국어 문서
nav_order: 9
---

# Input System

(PrSM 2 부터)

PrSM v2는 Unity의 **New Input System** 패키지를 위한 구문 설탕(syntactic sugar)을 도입하여,
`PlayerInput`이나 `InputAction` 보일러플레이트 코드를 직접 작성하지 않고도
간결하고 선언적인 방식으로 액션 상태를 조회할 수 있게 해줍니다.

## 기능 게이트

입력 구문 설탕은 명시적 기능 플래그 뒤에 위치합니다. `.prsmproject` 파일에
다음을 추가하세요:

```toml
features = ["input-system"]
```

또한 Unity 프로젝트에 **Input System** 패키지(`com.unity.inputsystem`)가
설치되어 있어야 합니다. 기능 플래그가 없는 상태에서 `input.*` 표현식을
사용하면 **error E070**이 발생합니다.

```
error[E070]: input-system sugar requires `features = ["input-system"]` in .prsmproject
  --> src/player_controller.prsm:12:5
   |
12 |     val move = input.action("Move").vector2
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

## Asset wiring

외부 `.inputactions` asset을 sugar에 연결하려면, 직렬화된
`InputActionAsset` 필드에 다음 annotation을 붙입니다:

```prsm
@inputActions(defaultMap: "Gameplay")
serialize controls: InputActionAsset
```

이 annotation은 compiler 전용이며 C# attribute로 내려가지 않습니다. 대신
compiler는 이 필드를 `PlayerInput.actions`의 소스로 사용하고,
`defaultMap`이 있으면 `PlayerInput.defaultActionMap`도 함께 설정합니다.

생성되는 wiring 예시:

```csharp
[UnityEngine.RequireComponent(typeof(UnityEngine.InputSystem.PlayerInput))]
private UnityEngine.InputSystem.PlayerInput _prsmInput;

void Awake()
{
    _prsmInput = GetComponent<UnityEngine.InputSystem.PlayerInput>();
    _prsmInput.actions = controls;
    _prsmInput.defaultActionMap = "Gameplay";
}
```

이 방식으로 Inspector에서 asset reference를 유지하면서, 생성된 component가
항상 `PlayerInput` 의존성을 가지도록 만들 수 있습니다.

## 기본 형식

기본 형식은 기본 액션 맵에서 명명된 액션을 조회합니다:

```prsm
input.action("ActionName").state
```

`state`를 아래에 설명된 다섯 가지 상태 접근자 중 하나로 교체하세요.

### 예제

```prsm
func update() {
    if input.action("Jump").pressed {
        jump()
    }

    val move = input.action("Move").vector2
    transform.translate(move.x, 0, move.y)
}
```

## Player 형식 (멀티플레이어)

각 플레이어가 고유한 액션 맵을 가지는 멀티플레이어 시나리오에서는
Player 형식을 사용하세요. 맵 이름과 액션 이름을 모두 지정합니다:

```prsm
input.player("MapName").action("ActionName").state
```

Player 형식은 `"Map/Action"` 형식의 조회 키를 생성하므로,
컴파일러는 `input.player("Gameplay").action("Fire").pressed`를
`_prsmInput.actions["Gameplay/Fire"]`에 대한 조회로 변환합니다.

### 예제

```prsm
func update() {
    if input.player("Gameplay").action("Fire").pressed {
        fireWeapon()
    }

    val look = input.player("Gameplay").action("Look").vector2
    rotateCameraBy(look)
}
```

## 상태 접근자

PrSM은 다섯 가지 상태 접근자를 제공합니다. 각 접근자는 확인된
`InputAction`의 특정 C# 호출에 매핑됩니다.

### `.pressed`

액션이 처음 눌린 프레임에서 `true`를 반환합니다.

| PrSM | 생성된 C# |
|---|---|
| `input.action("Jump").pressed` | `_prsmInput.actions["Jump"].WasPressedThisFrame()` |

### `.released`

액션이 해제된 프레임에서 `true`를 반환합니다.

| PrSM | 생성된 C# |
|---|---|
| `input.action("Jump").released` | `_prsmInput.actions["Jump"].WasReleasedThisFrame()` |

### `.held`

액션이 계속 눌려 있는 매 프레임마다 `true`를 반환합니다.

| PrSM | 생성된 C# |
|---|---|
| `input.action("Crouch").held` | `_prsmInput.actions["Crouch"].IsPressed()` |

### `.vector2`

현재 값을 `Vector2`로 읽습니다. 이동이나 시점 스틱에 주로 사용됩니다.

| PrSM | 생성된 C# |
|---|---|
| `input.action("Move").vector2` | `_prsmInput.actions["Move"].ReadValue<UnityEngine.Vector2>()` |

### `.scalar`

현재 값을 `float`로 읽습니다. 트리거나 1D 축에 유용합니다.

| PrSM | 생성된 C# |
|---|---|
| `input.action("Throttle").scalar` | `_prsmInput.actions["Throttle"].ReadValue<float>()` |

## 생성되는 인프라

컴파일러가 클래스에서 입력 구문 설탕을 감지하면, 직접 선언할 필요 없이
자동으로 지원 인프라를 생성합니다:

1. 클래스의 private 필드:

   ```csharp
   private UnityEngine.InputSystem.PlayerInput _prsmInput;
   ```

2. 클래스 수준 dependency marker:

   ```csharp
   [UnityEngine.RequireComponent(typeof(UnityEngine.InputSystem.PlayerInput))]
   ```

3. `Awake` 내부의 초기화 (이미 `Awake`가 있는 경우 병합):

   ```csharp
   void Awake()
   {
       _prsmInput = GetComponent<UnityEngine.InputSystem.PlayerInput>();
   }
   ```

클래스에 이미 PrSM의 `awake()` 함수가 있는 경우, 컴파일러는 사용자 구문
앞에 `GetComponent` 호출을 생성된 `Awake` 본문에 병합합니다.

## Error E070

기능 게이트를 활성화하지 않고 입력 구문 설탕을 사용하면, 컴파일러는
명확한 해결 방법과 함께 **E070**을 출력합니다:

```
error[E070]: input-system sugar requires `features = ["input-system"]` in .prsmproject
```

이를 해결하려면 프로젝트 파일에 `features = ["input-system"]`을 추가하고
Unity Input System 패키지가 설치되어 있는지 확인하세요.

## 레거시 Input 구문 설탕과의 비교

PrSM v1은 Unity의 레거시 `UnityEngine.Input` API를 위한 구문 설탕을
제공했습니다:

| 레거시 구문 설탕 (PrSM 1) | New Input System 구문 설탕 (PrSM 2 부터) |
|---|---|
| `input.getKey(KeyCode.Space)` | `input.action("Jump").pressed` |
| `input.axis("Horizontal")` | `input.action("Move").vector2.x` |
| `input.getMouseButton(0)` | `input.action("Fire").pressed` |

레거시 형식은 여전히 컴파일되며 `Input.GetKey`, `Input.GetAxis` 등의
호출을 생성합니다. 그러나 `input-system` 기능을 활성화한 경우, 같은 파일에서
레거시 구문 설탕과 새 입력 구문 설탕을 함께 사용하면 컴파일러가
액션 기반 API로의 마이그레이션을 권장하는 **경고**를 출력합니다.

두 스타일은 마이그레이션 기간 동안 같은 프로젝트에서 공존할 수 있지만,
단일 클래스 내에서 혼합하여 사용할 수는 없습니다.
