# 미결 설계 질문

## Q1: `serialize` 필드의 접근자 lowering
`serialize speed: Float = 5.0` 이 lowering될 때:
- 옵션 A: `[SerializeField] private float speed = 5.0f;` (private, Inspector만 수정)
- 옵션 B: `[SerializeField] public float speed = 5.0f;` (public)
- 옵션 C: backing field + property (읽기 전용 접근)

**현재 결정:** 옵션 A (private + SerializeField). 외부 접근이 필요하면 `public serialize` 명시.

## Q2: `input` sugar의 범위
`input.axis("H")`는 `Input.GetAxis("H")`로 매핑. Input System 패키지 지원은?
- v1에서는 Legacy Input만 지원
- New Input System은 v2에서 별도 sugar 설계

## Q3: `listen` 해제 타이밍
`listen button.onClick { }` 의 AddListener에 대응하는 RemoveListener는?
- 옵션 A: onDisable에서 자동 해제
- 옵션 B: 사용자가 명시적으로 `unlisten` 호출
- 옵션 C: v1에서는 해제 없음 (Unity가 GameObject 파괴 시 정리)

**현재 결정:** 미정. 프로토타입 후 결정.

## Q4: `data class`와 Unity 직렬화
`data class`를 Unity Inspector에서 사용하려면 `[System.Serializable]`이 필요.
- 옵션 A: data class는 항상 Serializable
- 옵션 B: `serialize data class` 로 명시
- 옵션 C: data class는 순수 CLR 타입, 직렬화 원하면 일반 class 사용

**현재 결정:** 옵션 A (항상 Serializable). 게임플레이 payload가 주 용도.

## Q5: 제네릭 호출 시 타입 추론
`get<Rigidbody>()` 는 타입을 명시하지만, 변수에 할당 시 추론 가능한가?
```
val rb: Rigidbody = get()  // 타입 추론으로 <Rigidbody> 생략?
```
**현재 결정:** v1에서는 항상 명시 요구. 추론은 v2에서 고려.

## Q6: intrinsic 블록 내 변수 접근
intrinsic 블록에서 Moon 변수를 참조할 때, lowered C# 이름을 사용해야 함.
`serialize speed: Float` → C#에서 `speed` (private field 이름)
이 매핑을 문서화할 것인가, 아니면 블록 앞에 자동 주석을 생성할 것인가?

**현재 결정:** 생성된 C# 주석으로 필드 매핑 표시.
