---
title: CLI
parent: 도구
grand_parent: 한국어 문서
nav_order: 1
---

# CLI

현재 `prism` CLI 는 빌드, 분석, 내비게이션, 유틸리티 명령으로 나뉘어 있습니다.

## 빌드와 검증

- `compile <path>` 는 파일 또는 디렉터리의 `.prsm` 소스를 컴파일합니다.
- `check <path>` 는 C# 생성 없이 diagnostics 만 계산합니다.
- `build` 는 현재 작업 디렉터리의 `.prsmproject` 를 찾아 프로젝트 전체를 빌드합니다.

자주 쓰는 옵션:

- `compile` 의 `--output`
- 기계 친화 출력용 `--json`
- `build` 의 `--watch`
- 경고를 줄이는 `compile --no-warnings`

## 분석과 내비게이션

- `hir [path] --json`
- `definition [path] --file ... --line ... --col ... --json`
- `references [path] --file ... --line ... --col ... --json`
- `index [path] --json`

`index` 는 추가로 다음을 지원합니다.

- `--symbol <name>`
- `--qualified-name <name>`
- `--file --line --col` 기반 위치 조회

## 유틸리티 명령

- `init`
- `where`
- `version`

## 대표 사용 예시

```powershell
cargo run -p refraction --bin prism -- compile samples\PlayerController.prsm --output build-output
cargo run -p refraction --bin prism -- check samples\PlayerController.prsm --json
cargo run -p refraction --bin prism -- build --json
cargo run -p refraction --bin prism -- hir . --json
cargo run -p refraction --bin prism -- definition . --json --file samples\PlayerController.prsm --line 10 --col 5
cargo run -p refraction --bin prism -- references . --json --file samples\PlayerController.prsm --line 10 --col 5
cargo run -p refraction --bin prism -- index . --json --symbol PlayerController
cargo run -p refraction --bin prism -- index . --json --file samples\PlayerController.prsm --line 10 --col 5
```

즉 현재 CLI 는 단순 트랜스파일러가 아니라, 의미 분석과 에디터 연동까지 떠받치는 분석 표면 역할을 함께 합니다.
