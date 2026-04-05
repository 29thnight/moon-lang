# PrSM Roslyn 사이드카 설계안

## 목적

PrSM은 Unity 전용 스크립트 언어에 가깝고, 생성된 C#도 Unity 프로젝트 안에서만 의미가 있다. 이 전제를 받아들이면 Unity API 정보를 Rust 내부의 수작업 카탈로그로 유지하는 것보다, 별도 .NET 사이드카가 Roslyn과 Unity 참조 어셈블리를 통해 실제 C# 심볼 정보를 제공하도록 바꾸는 편이 더 정확하다.

이 문서는 다음 구조를 정의한다.

- Rust LSP는 PrSM 자체 의미 분석의 권위자다.
- 별도 .NET 사이드카는 Unity/C# 외부 심볼 계층의 권위자다.
- 두 프로세스는 에디터와 분리된 프로세스 간 프로토콜로 통신한다.

## 왜 VS Code 중계가 아니라 사이드카인가

VS Code 확장 내부에서 C# 확장 기능을 중계하는 방식은 빠르게 붙이기에는 좋지만, 다음 문제가 있다.

- VS Code API에 종속된다.
- 다른 에디터나 테스트 환경에서 동일한 동작을 재현하기 어렵다.
- Rust LSP가 직접 기능을 소유하지 못한다.
- C# 확장의 상태와 워크스페이스 로딩 타이밍에 강하게 묶인다.

별도 사이드카는 구현량은 더 크지만 구조는 더 명확하다.

- Rust LSP가 직접 질의한다.
- 에디터 독립적이다.
- 통합 테스트가 쉽다.
- Unity API/생성 C# 조회를 한 군데로 모을 수 있다.

## 책임 분리

### Rust LSP가 계속 담당하는 것

- `.prsm` 파싱
- 프로젝트 인덱스 구축
- Typed HIR 기반 정의/참조/이름변경
- PrSM 고유 진단
- 편집 중 미저장 버퍼 상태 해석
- PrSM 문법 sugar와 lowering 전 의미 정보

### Roslyn 사이드카가 담당하는 것

- Unity 참조 어셈블리 로드
- 생성된 `.cs` 파일 로드
- C# 타입, 멤버, 이벤트, 프로퍼티 심볼 조회
- XML documentation 또는 메타데이터 기반 hover 정보 조회
- Unity API completion 후보 계산
- 생성된 C# 기준 definition/implementation 조회

## 대체 범위

전면 대체의 대상은 Unity/C# 외부 심볼 레이어다.

- 대체 대상:
  - Rust의 수작업 Unity 카탈로그
  - Unity 타입/멤버 hover 설명
  - Unity 타입/멤버 completion 카탈로그
- 대체 비대상:
  - PrSM 자체 semantic model
  - PrSM rename/reference의 원본 기준 span 계산
  - PrSM 문법 오류와 타입 오류

즉, `GameObject.SetActive`, `Transform.position`, `Button.onClick` 같은 정보는 사이드카가 권위자가 되고, `listen`, lifecycle sugar, 미저장 PrSM 편집 상태 같은 것은 Rust가 계속 권위자다.

## 프로세스 구조

```text
VS Code extension
  -> prism lsp (Rust)
       -> prism-roslyn-sidecar (.NET)
            -> Roslyn Workspace / MetadataReference / XML docs
            -> Unity generated .cs
            -> UnityEngine*.dll / Unity UI dll / package assemblies
```

사이드카는 독립 실행 파일 또는 `dotnet`으로 구동되는 작은 서비스로 둔다.

권장 이름:

- `prism-roslyn-sidecar`

권장 실행 방식:

- 기본: stdio JSON-RPC
- 대안: named pipe

초기 구현은 stdio가 가장 단순하다. Rust LSP가 이미 stdio 기반 패턴을 쓰고 있고, 프로세스 수명 관리도 쉽다.

## 프로젝트 입력

사이드카는 다음 입력을 받아야 한다.

- workspace root
- `.prsmproject` 경로와 해석된 output dir
- Unity 프로젝트 루트
- generated `.cs` 파일 목록
- Unity 참조 어셈블리 경로 목록
- 필요 시 패키지 어셈블리 경로 목록

참조 어셈블리 후보:

- UnityEngine.CoreModule.dll
- UnityEngine.UI.dll
- Unity.TextMeshPro.dll
- 프로젝트가 참조하는 패키지 DLL

## 통신 계약

Rust와 사이드카는 내부 전용 JSON-RPC 계약을 사용한다.

### 초기화

메서드:

- `sidecar/initialize`
- `sidecar/loadProject`
- `sidecar/shutdown`

`sidecar/loadProject` 요청 예시:

```json
{
  "workspaceRoot": "C:/Project",
  "projectFile": "C:/Project/.prsmproject",
  "unityProjectRoot": "C:/Project",
  "outputDir": "C:/Project/Assets/Generated/PrSM",
  "generatedFiles": [
    "C:/Project/Assets/Generated/PrSM/Player.cs"
  ],
  "metadataReferences": [
    "C:/Program Files/Unity/.../UnityEngine.CoreModule.dll",
    "C:/Program Files/Unity/.../UnityEngine.UI.dll"
  ]
}
```

### 질의 메서드

최소 계약은 다음 정도면 충분하다.

- `unity/completeMembers`
- `unity/getHover`
- `unity/getType`
- `unity/getDefinition`
- `unity/resolveGeneratedSymbol`
- `workspace/reload`
- `health/ping`

예시:

`unity/completeMembers`

```json
{
  "typeName": "Button",
  "prefix": "on",
  "context": {
    "generatedOwnerType": "PlayerUI"
  }
}
```

응답 예시:

```json
{
  "items": [
    {
      "label": "onClick",
      "kind": "event",
      "detail": "UnityEvent Button.onClick",
      "documentation": "UnityEvent that is triggered when the Button is pressed.",
      "signature": "Button.onClick"
    }
  ]
}
```

`unity/getHover`

```json
{
  "typeName": "Transform",
  "memberName": "position"
}
```

응답 예시:

```json
{
  "displayName": "UnityEngine.Transform.position",
  "signature": "public Vector3 position { get; set; }",
  "documentation": "The world space position of the Transform.",
  "assembly": "UnityEngine.CoreModule",
  "source": "metadata"
}
```

## Rust LSP 통합 방식

Rust LSP는 현재 [crates/refraction/src/lsp.rs](../crates/refraction/src/lsp.rs) 와 [crates/refraction/src/lsp_support.rs](../crates/refraction/src/lsp_support.rs) 에서 Unity completion과 hover를 계산한다. 이 중 Unity 외부 API 관련 부분만 사이드카 질의로 바꾼다.

### completion 흐름

1. Rust가 현재 receiver type을 해석한다.
2. receiver가 PrSM 내부 타입이면 기존 HIR/index 경로를 사용한다.
3. receiver가 Unity 또는 외부 C# 타입이면 사이드카의 `unity/completeMembers`를 호출한다.
4. 결과를 LSP completion item으로 변환한다.

### hover 흐름

1. Rust가 현재 위치의 PrSM symbol/reference를 해석한다.
2. PrSM 자체 정의 정보는 Rust가 먼저 만든다.
3. Unity/C# lookup target이 가능하면 `unity/getHover`를 호출한다.
4. Rust가 PrSM hover 섹션과 C# hover 섹션을 합쳐 최종 markdown을 반환한다.

### definition 흐름

1. 원본 PrSM 정의가 있으면 Rust 결과를 우선한다.
2. 외부 Unity/C# 심볼이면 `unity/getDefinition` 또는 `unity/resolveGeneratedSymbol`를 호출한다.
3. metadata-only 심볼이면 문서 링크 또는 decompiled/metadata location 정책을 별도 정의한다.

## generated C#와의 관계

사이드카는 두 종류의 C# 심볼을 다룬다.

### 1. Unity 엔진/패키지 메타데이터 심볼

- `Transform`
- `GameObject`
- `Button`
- `UnityEvent`

이 경우 소스 파일이 아니라 메타데이터 참조에서 정보를 얻는다.

### 2. PrSM이 생성한 C# 심볼

- `Player.cs`
- `EnemyState.cs`

이 경우 generated `.cs` 자체를 Roslyn 문서로 워크스페이스에 올린다. Rust는 이미 생성된 lookup target과 `.prsmmap.json`을 알고 있으므로, 원본 PrSM 위치와 generated C# 심볼을 연결하는 브로커 역할만 한다.

## 캐시와 무효화

사이드카는 매 요청마다 워크스페이스를 다시 만들면 안 된다. 최소한 다음 캐시가 필요하다.

- metadata references 캐시
- generated document 캐시
- compilation 캐시
- XML documentation provider 캐시

무효화 트리거:

- `.prsmproject` 변경
- generated `.cs` 파일 변경
- Unity 설치 경로 또는 참조 DLL 집합 변경
- 패키지 manifest 변경

Rust LSP는 `workspace/reload`를 보낼 수 있어야 한다.

## 실패 시 동작

Roslyn 사이드카는 강화 계층이지만, Unity 전용 언어라는 전제를 고려하면 기본 경로로 취급할 수 있다. 다만 실패 시 LSP 전체가 깨지면 안 된다.

권장 정책:

- 사이드카 시작 실패:
  - Unity hover/completion을 비활성화하고 경고 로그만 남김
- 프로젝트 로드 실패:
  - PrSM 기본 기능은 유지
- 특정 질의 실패:
  - 해당 요청만 부분 실패 처리

초기 단계에서는 작은 fallback 카탈로그를 남겨도 되지만, 최종 목표는 Rust 수작업 Unity 카탈로그 제거다.

## 단계별 구현 순서

### 1단계: sidecar 뼈대

- `.NET` 실행 파일 생성
- stdio JSON-RPC 루프 구현
- `health/ping`, `sidecar/initialize`, `sidecar/loadProject` 구현

### 2단계: metadata hover

- UnityEngine 참조 어셈블리 로드
- `unity/getHover` 구현
- Rust hover에 C# 섹션 연결

### 3단계: metadata completion

- `unity/completeMembers` 구현
- Rust completion에서 수작업 카탈로그 대신 sidecar 사용

### 4단계: generated C# symbol query

- generated `.cs` 문서 로드
- `unity/resolveGeneratedSymbol`, `unity/getDefinition` 구현
- generated C# definition 연결

### 5단계: 수작업 카탈로그 제거

- [crates/refraction/src/lsp_support.rs](../crates/refraction/src/lsp_support.rs)의 Unity 타입/멤버 상수 제거
- 최소 fallback만 남기거나 완전 제거

## 장점

- Unity API가 실제 참조 어셈블리와 동기화된다.
- 시그니처와 문서 정확도가 높다.
- Rust 내부 수작업 카탈로그를 없앨 수 있다.
- VS Code 확장 API에 묶이지 않는다.
- 장기적으로 다른 에디터 통합도 가능하다.

## 비용과 리스크

- Rust + .NET 이중 런타임 유지 비용
- Unity 참조 DLL 탐색 규칙 정리 필요
- Roslyn 워크스페이스 캐시와 증분 갱신 구현 필요
- 테스트 환경에서 Unity 참조를 어떻게 주입할지 결정 필요

## 최종 판단

이 구조는 Unity에 강하게 결합된 언어라는 전제를 받아들이면서도, PrSM 자체 semantic service의 권위를 잃지 않는 방향이다.

정리하면 다음과 같다.

- Unity API와 generated C# 심볼 정보는 사이드카가 권위자다.
- PrSM 원본 의미 분석과 편집 상태 해석은 Rust LSP가 권위자다.
- Rust의 수작업 Unity 카탈로그는 단계적으로 제거한다.

이 안을 기준으로 구현을 시작하면, 현재의 임시 수작업 카탈로그를 장기 구조로 굳히지 않고 자연스럽게 치환할 수 있다.