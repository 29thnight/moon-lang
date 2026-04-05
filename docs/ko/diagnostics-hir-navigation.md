---
title: Diagnostics, HIR, and Navigation
parent: 고급 주제
grand_parent: 한국어 문서
nav_order: 2
---

# Diagnostics, HIR, and Navigation

PrSM은 단순 코드 생성 외에도 계층화된 분석 파이프라인을 이미 제공합니다.

현재 제공되는 축은 다음과 같습니다.

- 파싱 및 시맨틱 분석 기반 diagnostics
- 시작/끝 위치를 모두 포함한 JSON diagnostics 출력
- 정의와 참조를 담는 Typed HIR 출력
- definition lookup 과 프로젝트 전역 reference lookup
- 문법 수준의 project symbol / type-reference index 조회

## Diagnostics

기계가 읽기 쉬운 diagnostics 는 `prism check --json` 또는 `prism build --json` 으로 얻을 수 있습니다.

각 diagnostics 항목에는 다음 정보가 들어 있습니다.

- 안정적인 코드와 severity
- 메시지 텍스트
- 파일 경로
- `line` / `col`
- `end_line` / `end_col`

이 끝 위치 정보는 VS Code 확장과 Unity 측 도구가 정확한 하이라이트 및 역매핑을 할 때 그대로 사용합니다.

## Typed HIR

`prism hir . --json` 은 파일 단위 Typed HIR 데이터를 출력합니다.

여기에는 다음이 포함됩니다.

- kind, qualified name, type, mutability, exact span 을 가진 definition
- kind, exact span, resolved definition id 를 가진 reference
- `build` 경로에서 함께 계산되는 프로젝트 단위 HIR 통계

Typed HIR 은 definition 과 references 같은 의미 기반 내비게이션 명령의 핵심 기반입니다.

## Definition 과 References

내비게이션 CLI 는 위치 기반으로 동작합니다.

```powershell
cargo run -p refraction --bin prism -- definition . --json --file samples\PlayerController.prsm --line 10 --col 5
cargo run -p refraction --bin prism -- references . --json --file samples\PlayerController.prsm --line 10 --col 5
```

`definition` 은 해당 위치의 심볼이 가리키는 선언을 찾습니다.

`references` 는 먼저 소유 definition 을 확인한 다음, 같은 qualified symbol 에 대한 프로젝트 전역 참조 집합을 돌려줍니다.

## Project Index

`prism index` 는 Typed HIR 의 가벼운 문법 수준 동반 계층입니다.

지원하는 조회 방식은 다음과 같습니다.

- 전체 프로젝트 symbol 목록
- 정확한 `--symbol` 필터
- 정확한 `--qualified-name` 필터
- `--file`, `--line`, `--col` 기반 위치 조회

위치 조회는 두 종류의 결과를 함께 줄 수 있습니다.

- 선언/멤버 위치를 가리키는 `symbol_at`
- 필드 타입, 파라미터 타입, 선언 헤더 타입 이름 같은 위치를 가리키는 `reference_at`

덕분에 시맨틱 lookup 을 바로 쓰지 않는 경우에도 index 만으로 유의미한 탐색이 가능합니다.

## 에디터 연동

VS Code 확장은 이 계층들을 함께 사용합니다.

- diagnostics 는 `prism check --json` 에서 옴
- go-to-definition 은 우선 `prism definition` 을 사용
- hover 와 일부 내비게이션은 `prism index` 를 fallback 으로 사용
- references 와 rename 은 Typed HIR 기반으로 동작
- document / workspace symbol 은 캐시된 index 결과로 구성됨

즉 전용 LSP 서버는 아직 미래 작업이지만, 현재 CLI 와 확장만으로도 분석 기반 편집 워크플로는 이미 성립합니다.
