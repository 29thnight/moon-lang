# PrSM v2 확장 설계

## 1. 목표

v1이 "작성 가능하고 Unity에 붙는 최소 제품"이라면, v2의 목표는 PrSM을 중간 규모 Unity 프로젝트와 팀 단위 워크플로에 견딜 수 있는 도구 체계로 확장하는 것이다.

v2의 핵심 목표는 다음 네 가지다.

1. 상태와 데이터를 더 자연스럽게 다룰 수 있는 언어 기능 추가
2. Unity 의존 sugar의 수명과 의미론을 명시적으로 고정
3. 프로젝트 규모 편집 경험을 위한 LSP/인덱스/증분 빌드 기반 마련
4. 런타임 오류, 콘솔 로그, 디버깅 흐름을 원본 `.prsm` 기준으로 추적 가능하게 만들기

## 2. 비목표

v2에서도 다음 항목은 범위 밖으로 둔다.

- 범용 프로그래밍 언어로의 확장
- 커스텀 VM/런타임 도입
- 매크로/메타프로그래밍
- Kotlin 전면 호환
- IL2CPP 친화성을 해치는 reflection 중심 추상화

## 3. 설계 원칙

### Unity-first
- 모든 확장은 Unity 게임플레이 코드 작성 비용을 줄이는 방향이어야 한다.
- Unity 패키지 의존 기능은 감지 가능하고 진단 가능해야 한다.

### AOT-safe
- lowering 결과는 읽을 수 있는 C#이어야 하고, 런타임 코드 생성 없이 동작해야 한다.

### Opt-in breaking change
- v1과 의미가 달라지는 기능은 프로젝트 버전 또는 feature flag로 게이트한다.

### Shared compiler foundation
- CLI, Unity 패키지, VS Code 확장은 동일한 프로젝트 모델과 심볼/진단 인덱스를 공유해야 한다.

## 4. v2 확장 축

## 4.1 언어 버전과 feature gate

v2부터 `.prsmproject`에 언어 버전과 기능 플래그를 명시한다.

```toml
[language]
version = "2.0"
features = ["pattern-bindings", "auto-unlisten", "input-system"]
```

목적은 세 가지다.

- v1과 의미가 달라지는 기능을 안전하게 도입한다.
- Unity 패키지 의존 기능을 명시적으로 활성화한다.
- VS Code/Unity/CLI가 동일한 규칙으로 진단을 낼 수 있게 한다.

기본 정책:

- `version`이 없으면 v1 의미론 유지
- `features`는 선택 기능만 활성화
- 컴파일러는 미지원 버전/기능에 대해 명시적 에러를 낸다

## 4.2 패턴 매칭과 구조 분해

v1의 `when`은 분기 자체는 가능하지만 상태 payload와 데이터 구조를 다루는 표현력이 부족하다. v2에서는 패턴 바인딩을 추가한다.

예시:

```prsm
when state {
    EnemyState.Idle => idle()
    EnemyState.Chase(target) => moveTo(target)
    EnemyState.Stunned(duration) if duration > 0.0 => wait(duration)
}

val PlayerStats(hp, speed) = stats

for EnemySpawn(position, delay) in wave.spawns {
    spawnAt(position, delay)
}
```

지원 범위:

- 파라미터 enum 엔트리 패턴
- data class/일반 class 생성자형 구조 분해
- `when` 가드(`if`) 패턴
- `val`, `for`, `when`에서의 바인딩

의미론:

- enum payload 패턴은 엔트리 비교 후 payload accessor를 임시 변수에 바인딩한다
- data class 구조 분해는 공개 필드/프로퍼티 기준으로 lowering한다
- `when` 완전성 검사는 기존 enum exhaustiveness 규칙을 확장해 유지한다

의도적으로 제외하는 항목:

- OR 패턴
- 리스트/맵 패턴
- 임의 깊이의 중첩 패턴 최적화

이 범위를 넘기면 AST 직접 lowering보다 typed intermediate layer가 필요해지므로, v2에서는 이를 위한 HIR 도입을 같이 진행한다.

## 4.3 listen 수명 모델

v1의 `listen`은 등록만 수행한다. v2에서는 수명 정책을 문법으로 명시한다.

예시:

```prsm
listen button.onClick until disable {
    fire()
}

listen spawner.spawned until destroy {
    handleSpawn(it)
}

val token = listen timer.finished manual {
    reset()
}

unlisten token
```

설계:

- `until disable`: `OnDisable`에서 자동 해제
- `until destroy`: `OnDestroy`에서 자동 해제
- `manual`: 구독 토큰 반환, 사용자가 `unlisten` 호출

기본값:

- `language.version = "2.0"`인 component 내부에서 `listen`은 기본적으로 `until disable`
- v1 프로젝트는 기존처럼 등록만 수행

lowering 전략:

- 컴파일러가 숨김 구독 필드 또는 토큰 목록을 생성
- 이미 존재하는 `onDisable`/`onDestroy` 블록과 충돌하지 않도록 정리 메서드를 별도 합성 후 lifecycle에서 호출
- UnityEvent와 C# event/delegate 패턴을 모두 다루되, 지원 불가 형식은 명시적으로 진단

## 4.4 New Input System sugar

v1은 Legacy Input만 지원한다. v2에서는 Unity New Input System을 선택적으로 지원한다.

예시:

```prsm
if input.action("Jump").pressed {
    jump()
}

val move = input.action("Move").vector2
val look = input.player("Gameplay").action("Look").vector2
```

범위:

- `InputAction`, `InputActionReference`, `PlayerInput` 기반 sugar
- 버튼, 스칼라, `Vector2` 입력 읽기
- 프레임 상태 (`pressed`, `released`, `held`, `wasPressedThisFrame`) 제공

프로젝트 모델 요구사항:

- Unity `Packages/manifest.json` 또는 `.prsmproject` capability에서 Input System 패키지 존재를 감지
- 패키지가 없는데 sugar를 사용하면 컴파일 에러

비범위:

- Input Actions asset 내부 이름 정적 검증
- generated strongly-typed wrapper 자동 생성

즉, v2의 목표는 "패키지 존재를 이해하는 언어"까지이고, 에셋 스키마 수준의 정적 검증은 이후 단계로 둔다.

## 4.5 제네릭 호출 타입 추론

v1에서는 `get<Rigidbody>()`처럼 타입 인자를 항상 적어야 한다. v2는 기대 타입 기반의 제한적 추론을 도입한다.

예시:

```prsm
val rb: Rigidbody = get()
val health: Health? = child()
val points: List<Int> = loadJson()
```

규칙:

- 좌변 타입, 함수 인자 타입, 반환 기대 타입에서만 추론
- 단일 해가 나올 때만 허용
- 모호하면 명시적 타입 인자 요구

의도적으로 제외:

- 전역 Hindley-Milner 스타일 추론
- 람다와 오버로드가 겹친 복합 추론
- 제네릭 선언 자체의 대규모 확장

즉, v2의 추론은 "보일러플레이트 제거" 수준으로만 제한한다.

## 4.6 프로젝트 그래프와 증분 캐시

v2 기능 대부분은 파일 단위 처리만으로는 한계가 있다. 따라서 컴파일러 내부에 명시적 프로젝트 그래프와 증분 캐시를 도입한다.

새 내부 구성 요소:

- `ProjectGraph`: 소스 파일, import, Unity capability, language version 해석
- `FileSummary`: 선언, 참조, 진단, 생성 산출물 요약
- `SymbolIndex`: 정의/참조/hover/LSP용 심볼 테이블
- `SourceMapIndex`: `.prsm`와 generated C# 위치 매핑

캐시 정책:

- 작업 디렉터리 아래 `.prsm/cache` 사용
- 파일 해시 기반 invalidation
- CLI, Unity, VS Code가 동일 캐시 포맷 공유

이 계층이 먼저 있어야 LSP와 source mapping이 반복 구현 없이 붙는다.

## 4.7 Typed HIR 도입

현재 구조는 AST -> semantic -> lowering 흐름이 단순해서 v1에는 적합하지만, v2 기능은 해석된 식/패턴/호출 정보를 재사용해야 한다. 따라서 v2부터 typed HIR를 도입한다.

HIR가 보유해야 하는 정보:

- 해석된 심볼 핸들
- 확정된 식 타입
- 제네릭 치환 결과
- 패턴 바인딩 정보
- `listen` 수명 정책
- source span과 generated source mapping anchor

역할 분리:

- Parser는 순수 구문 AST 생성
- Semantic은 AST를 검사하면서 HIR 생성
- Lowering은 AST가 아니라 HIR를 입력으로 사용

효과:

- 패턴 매칭 lowering 단순화
- LSP hover/completion/rename 데이터 재사용
- 디버거/source map 생성 지점 일원화

## 4.8 LSP 서버

v2의 편집기 경험은 현재 확장의 shell-out 중심 모델을 넘어서야 한다. 설계는 `prism lsp` 서브커맨드 또는 별도 `prsmls` 바이너리 두 가지 중 하나로 수렴한다.

권장 방향:

- 배포 복잡도를 줄이기 위해 `prism lsp` 우선
- VS Code 확장은 thin client로 유지

1차 지원 범위:

- completion
- hover
- go-to-definition
- references
- rename
- document/workspace symbols
- code action (명시적 타입 인자 추가, 미사용 import 정리 등)

구현 원칙:

- LSP는 `ProjectGraph + SymbolIndex + HIR`를 그대로 사용
- 진단은 기존 `check --json` 경로와 동일 규칙을 사용하되, incremental snapshot 기반으로 빠르게 응답

## 4.9 source mapping과 디버깅

v1의 `#line`은 기반은 갖췄지만, 실제 디버깅 경험은 더 정교한 매핑이 필요하다. v2는 source mapping을 독립 산출물로 승격한다.

산출물:

- generated C# 내부의 정교한 `#line` 블록
- 파일별 또는 프로젝트별 sidecar source map JSON

활용처:

- Unity Console 예외/스택트레이스 remap
- VS Code generated C# peek와 원본 소스 점프
- 향후 debugger integration

원칙:

- 매핑 단위는 멤버/식/블록 수준
- 사람이 읽는 generated C#를 훼손하지 않아야 함
- Unity 패키지와 VS Code 확장이 동일 포맷을 읽어야 함

## 4.10 테스트 전략 확장

v2 기능은 툴 체인 전반을 건드리므로 테스트를 계층화해야 한다.

필수 레이어:

- Rust unit/integration: parser, semantic, lowering, source map, HIR
- golden tests: 패턴 매칭, listener lowering, input sugar
- VS Code integration: LSP hover/definition/rename
- Unity package EditMode: source map 소비, log remap, compiler resolver
- Unity PlayMode/BlazeTest: 실제 listen 해제, Input System 동작, 런타임 예외 매핑

## 5. 릴리스 단계

### 단계 1: 기반 계층
- 언어 버전/feature gate
- ProjectGraph + cache
- Typed HIR
- source map 포맷 초안

### 단계 2: 편집기 기반
- `prism lsp` 구현
- VS Code 확장 thin client 전환
- go-to-definition/hover/rename 우선 제공

### 단계 3: 언어 확장
- 패턴 바인딩
- 제네릭 호출 추론
- `listen` 수명 정책

### 단계 4: Unity 확장
- New Input System sugar
- Unity Console/stacktrace remap 고도화
- Unity 통합 테스트 확대

이 순서를 고정하는 이유는, v2 언어 기능 대부분이 결국 프로젝트 그래프와 typed semantic layer를 요구하기 때문이다. 순서를 거꾸로 가면 구현은 빠를 수 있어도 재사용성이 무너진다.

## 6. 호환성 정책

- v1 프로젝트는 아무 설정이 없으면 기존 의미론 유지
- breaking change는 `language.version = "2.0"` 또는 개별 feature flag가 있을 때만 적용
- generated C#의 public shape는 가능한 한 안정적으로 유지
- 각 기능은 CLI/Unity/VS Code에서 동일한 오류 코드 체계를 사용

## 7. 권장 우선순위

실제 구현 착수 순서는 다음이 가장 안전하다.

1. ProjectGraph와 Typed HIR
2. source map 포맷과 cache
3. `prism lsp`
4. `listen` 수명 정책
5. 제한적 제네릭 추론
6. 패턴 바인딩
7. New Input System sugar

이 순서면 언어 기능이 늘어날수록 도구 품질도 같이 올라가고, Unity/VS Code/CLI가 서로 다른 규칙으로 분기하는 문제를 피할 수 있다.

## 8. v2 종료 조건

다음 조건을 만족하면 v2의 첫 릴리스를 고려할 수 있다.

- 최소 한 개 Unity 샘플 프로젝트가 `language.version = "2.0"`으로 동작
- LSP에서 definition/hover/rename이 실사용 가능한 수준으로 동작
- Unity Console 예외와 컴파일 진단이 `.prsm` 기준으로 안정적으로 역매핑됨
- `listen` 수명 정책과 Input System sugar가 BlazeTest에서 검증됨
- 새 기능이 모두 feature gate 또는 language version 아래에서 문서화됨