# New Input System sugar와 Unity 구조 불일치 분석

작성일: 2026-04-10

## 요약

현재 PrSM의 New Input System sugar는 다음 형태를 기준으로 설계되어 있다.

```prsm
input.action("Jump").pressed
input.player("Gameplay").action("Fire").pressed
```

현재 lowering은 이를 대략 다음 C#으로 변환한다.

```csharp
_prsmInput.actions["Jump"].WasPressedThisFrame()
_prsmInput.actions["Gameplay/Fire"].WasPressedThisFrame()
```

Unity Input System API 기준으로 보면 `WasPressedThisFrame()`, `WasReleasedThisFrame()`, `IsPressed()`, `ReadValue<T>()`, 그리고 `InputActionAsset["Map/Action"]` 조회 자체는 존재한다. 문제는 PrSM sugar가 Unity의 `PlayerInput`, action map, generated C# wrapper 구조를 서로 섞어서 설명하고 있다는 점이다.

## 잘못된 부분

### 1. `input.player("MapName")`는 실제 Unity 개념과 이름이 맞지 않음

문서와 구현은 `input.player("Gameplay").action("Fire")`에서 `"Gameplay"`를 action map 이름처럼 사용한다.

```rust
format!("{}/{}", map_name, action_name)
```

관련 위치:

- `crates/refraction/src/lowering/lower.rs`: `extract_input_player_name`, `try_lower_new_input_member`
- `docs/en/input-system.md`: "Player Form (Multiplayer)"
- `docs/ko/input-system.md`: "Player 형식 (멀티플레이어)"

하지만 Unity의 `PlayerInput`에서 "player"는 action map selector가 아니라 플레이어별 입력 인스턴스, 즉 `PlayerInput` 컴포넌트와 `InputUser`에 가까운 개념이다. Action map은 `currentActionMap`, `defaultActionMap`, `SwitchCurrentActionMap(string)`, 또는 `InputActionAsset.FindActionMap(...)` 쪽 개념이다.

따라서 현재 sugar 이름은 의미가 틀렸다. 실제 의도가 map 선택이면 아래처럼 이름을 바꾸는 편이 맞다.

```prsm
input.map("Gameplay").action("Fire").pressed
```

또는 Unity의 asset lookup 형식을 그대로 노출하려면 아래처럼 단순화할 수 있다.

```prsm
input.action("Gameplay/Fire").pressed
```

### 2. `input.action("Jump")`가 "default action map" 조회라는 설명이 정확하지 않음

`docs/en/input-system.md`는 기본형을 "default action map에서 action을 조회한다"고 설명한다. 하지만 실제 lowering은 `_prsmInput.actions["Jump"]`를 만든다.

Unity의 `InputActionAsset` string indexer와 `FindAction(string)`은 `"Map/Action"` 형태면 특정 map 안에서 찾지만, 단순 이름 `"Jump"`이면 모든 action map을 순서대로 검색해서 첫 번째 일치 action을 반환한다. 즉 현재 lowering은 "default map" 또는 "current map" 조회가 아니다.

이 차이는 여러 action map에 같은 action 이름이 있을 때 바로 문제가 된다.

예:

```text
Gameplay/Submit
UI/Submit
```

`input.action("Submit").pressed`가 현재 map의 `Submit`을 읽는다는 기대와 달리, asset 순서와 enabled 상태에 따라 다른 action을 반환할 수 있다.

수정 방향:

- 현재 action map을 기준으로 하려면 `_prsmInput.currentActionMap["Jump"]` 또는 null-safe helper를 사용한다.
- 특정 map을 기준으로 하려면 `input.map("Gameplay").action("Jump")`처럼 명시한다.
- asset 전체 검색이 의도라면 문서에서 "default action map"이라는 표현을 제거한다.

### 3. `PlayerInput` 인프라 자동 생성 설명이 Unity 런타임 전제를 빠뜨림

컴파일러는 input sugar를 감지하면 다음을 주입한다.

```csharp
private PlayerInput _prsmInput;
_prsmInput = GetComponent<PlayerInput>();
```

관련 위치:

- `crates/refraction/src/lowering/lower.rs`: `uses_input_system` 처리
- `docs/en/input-system.md`: "Generated Infrastructure"

하지만 Unity에서 이 코드가 실제로 동작하려면 GameObject에 `PlayerInput` 컴포넌트가 붙어 있어야 하고, `actions` asset과 default map 설정도 되어 있어야 한다. 현재 문서는 "직접 선언하지 않아도 된다"는 점만 말하고, 필요한 Unity-side 설정을 충분히 명시하지 않는다.

또한 compiler는 `[RequireComponent(typeof(PlayerInput))]`를 추가하지 않고, null guard도 만들지 않는다. 그래서 컴포넌트가 없거나 `actions`가 비어 있으면 런타임 NRE 또는 action lookup 예외가 날 수 있다.

### 4. package detection은 있지만 진단에 사용되지 않음

`project_graph.rs`는 `Packages/manifest.json`에서 `com.unity.inputsystem`을 감지한다.

```rust
capabilities.input_system_package = dependencies.contains_key("com.unity.inputsystem");
```

하지만 analyzer의 E070은 `.prsmproject`의 `input-system` feature flag만 확인한다. 즉 문서가 말하는 "Input System 패키지가 설치되어 있어야 한다"는 조건은 실제 compile error로 이어지지 않는다. 현재 build report에 `unity_input_system` 값이 실릴 뿐이다.

수정 방향:

- feature flag가 켜져 있는데 Unity package가 없으면 별도 error 또는 warning을 낸다.
- 단일 파일 compile처럼 Unity project context가 없을 수 있는 경로에서는 "패키지 확인 불가"와 "확인됨"을 구분한다.

### 5. 자동 `using UnityEngine.InputSystem` 주입이 없음

현재 `lower_usings`는 원본 `using`과 `System.Collections`만 유지/추가한다. 그런데 input sugar는 숨은 `PlayerInput` 필드를 만든다. 원본 `.prsm`에 `using UnityEngine.InputSystem`이 없다면 생성 C#의 `PlayerInput` 타입이 해석되지 않을 수 있다.

관련 위치:

- `crates/refraction/src/lowering/lower.rs`: `lower_usings`
- `crates/refraction/src/lsp.rs`: LSP organize imports는 `input.action` 사용 시 `UnityEngine.InputSystem`을 used namespace로 보존하지만, compiler lowering이 새 using을 추가하지는 않음

수정 방향:

- input sugar 사용 시 `UnityEngine.InputSystem` using을 자동 추가한다.
- 또는 숨은 필드를 `UnityEngine.InputSystem.PlayerInput`처럼 fully qualified type으로 생성한다.

### 6. `.github/skills/prsm-grammar`의 input 예시는 현재 구현 및 Unity New Input System과 불일치

`.github/skills/prsm-grammar/references/06-expressions.md`에는 다음 예시가 있다.

```prsm
val jumped = input.action("Jump").pressed
```

```csharp
var jumped = Input.GetButtonDown("Jump");
```

이는 New Input System이 아니라 Legacy `UnityEngine.Input` API로 낮추는 설명이다. 현재 compiler 구현도 `input.action("Jump").pressed`를 `Input.GetButtonDown`으로 낮추지 않는다.

또한 같은 파일의 New Input System 예시는 다음과 같다.

```prsm
val move = input.player("Player").action("Move").readValue<Vector2>()
```

하지만 현재 compiler는 `.readValue<T>()` 형태를 New Input System sugar로 구현하지 않고, `.vector2` 또는 `.scalar` 접근자를 구현한다. 게다가 `"Player"`가 player 이름인지 action map 이름인지 모호하다. 실제 Unity generated C# wrapper 관점에서는 `controls.gameplay.move.ReadValue<Vector2>()`처럼 wrapper 인스턴스와 action map property를 쓰는 구조가 별도이다.

## 현재 맞는 부분

다음 accessor와 Unity API 매핑은 큰 방향에서 맞다.

| PrSM accessor | Unity InputAction API |
|---|---|
| `.pressed` | `WasPressedThisFrame()` |
| `.released` | `WasReleasedThisFrame()` |
| `.held` | `IsPressed()` |
| `.vector2` | `ReadValue<UnityEngine.Vector2>()` |
| `.scalar` | `ReadValue<float>()` |

또한 `_prsmInput.actions["Gameplay/Fire"]` 형태는 `InputActionAsset`의 `"map/action"` 조회 규칙에 맞다. 다만 이것은 "player form"이 아니라 "asset에서 map/action 경로로 action을 찾는 form"으로 설명해야 한다.

## 권장 정리 방향

1. 문서와 구현에서 `input.player("Map")`를 폐기하거나 `input.map("Map")`으로 이름을 바꾼다.
2. `input.action("Name")`의 의미를 하나로 고정한다.
   - current/default map 기반이면 `currentActionMap`을 사용한다.
   - asset 전체 lookup이면 문서를 그렇게 고친다.
3. generated C# wrapper 지원 여부를 분리한다.
   - 현재 v2 sugar: `PlayerInput.actions` 기반 polling sugar
   - 미래 DSL: `.inputactions` asset 파싱 및 generated wrapper/typed action DSL
4. `UnityEngine.InputSystem` using 또는 fully qualified type을 compiler가 보장한다.
5. `com.unity.inputsystem` package 존재 여부를 feature gate와 함께 진단한다.
6. `.github/skills/prsm-grammar/references/06-expressions.md`의 input 섹션을 현재 compiler와 docs 기준으로 교체한다.

## 참고한 Unity API

- Unity Input System 1.17 `PlayerInput`: `actions`, `currentActionMap`, `defaultActionMap`, `SwitchCurrentActionMap`
- Unity Input System 1.17 `InputActionAsset`: string indexer와 `FindAction(string)`의 `"map/action"` 조회 규칙
- Unity Input System 1.17 `InputAction`: `WasPressedThisFrame()`, `WasReleasedThisFrame()`, `IsPressed()`, `ReadValue<TValue>()`
