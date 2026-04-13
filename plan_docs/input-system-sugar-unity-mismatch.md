# New Input System sugar와 Unity 구조 불일치 분석

작성일: 2026-04-10
수정일: 2026-04-13

## 요약

v3.5.5까지 PrSM의 New Input System sugar는 `PlayerInput` 컴포넌트를 강제 주입하여
Unity의 권장 Actions 워크플로우와 맞지 않았다. v3.6에서 이를 수정하여
`InputSystem.actions.FindAction()`을 사용하도록 변경했다.

## 해결된 이슈

### 1. [해결] `input.player("MapName")` 네이밍 불일치

`input.player()`를 `input.map()`으로 리네이밍했다. Unity에서 "player"는
`PlayerInput` 컴포넌트를 의미하며, 액션 맵 선택과는 다른 개념이다.

변경 전: `input.player("Gameplay").action("Fire").pressed`
변경 후: `input.map("Gameplay").action("Fire").pressed`

### 2. [해결] PlayerInput 강제 주입

`input.action()` 사용만으로 `PlayerInput` 컴포넌트, `[RequireComponent]`,
`_prsmInput` 필드가 자동 주입되었다. Unity 1.19의 권장 워크플로우는
`InputSystem.actions.FindAction()`으로 `PlayerInput` 없이 동작한다.

변경 후: `input.action("Jump").pressed` →
`InputSystem.actions.FindAction("Jump").WasPressedThisFrame()`

`PlayerInput` 인프라는 `@inputActions` 어노테이션이 있을 때만 주입된다.

### 3. [해결] `using UnityEngine.InputSystem` 자동 추가

이미 `file_requires_input_system_namespace()` 함수로 구현되어 있었다.
문서에서 이를 명시하도록 수정했다.

### 4. [해결] E070 chained form 미검출

`input.map("M").action("X")` 체인 형태에서 E070이 발생하지 않던 문제를 수정.
semantic analyzer에서 direct form과 chained form 모두 체크하도록 변경.

### 5. [해결] 문서 불일치

`docs/en/input-system.md`, `docs/ko/input-system.md`,
`.github/skills/prsm-grammar/references/06-expressions.md`의 input 섹션을
현재 구현과 Unity 1.19 API 기준으로 전면 재작성했다.

## 남은 이슈

### 1. package detection 진단 미사용

`project_graph.rs`는 `Packages/manifest.json`에서 `com.unity.inputsystem`을
감지하지만, analyzer의 E070은 `.prsmproject`의 feature flag만 확인한다.
feature flag가 켜져 있는데 Unity 패키지가 없으면 별도 warning이 없다.

수정 방향: feature flag 활성화 + 패키지 미설치 시 warning 또는 info 진단 추가.

### 2. Direct 워크플로우 미지원

`Keyboard.current.spaceKey.isPressed`, `Mouse.current.leftButton.wasPressedThisFrame`
등 Unity New Input System의 Direct 워크플로우 sugar가 없다.
레거시 `input.getKey()` 등의 새 Input System 대응이 존재하지 않는다.

## 참고한 Unity API

- Unity Input System 1.19 `InputSystem.actions`: 프로젝트 전역 `InputActionAsset`
- Unity Input System 1.19 `InputActionAsset.FindAction(string)`: `"map/action"` 또는 단순 이름으로 조회
- Unity Input System 1.19 `InputAction`: `WasPressedThisFrame()`, `WasReleasedThisFrame()`, `IsPressed()`, `ReadValue<TValue>()`
