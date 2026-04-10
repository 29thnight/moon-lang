# PrSM Issue Command Matrix

이 문서는 조사 단계에서 어떤 명령을 언제 선택할지 정리한다.

## 표면별 시작점

### Compiler and CLI
- 전체 Rust 테스트: `cargo test`
- 단일 흐름 확인: `cargo run -p refraction --bin prism -- check <file-or-project>`
- HIR/definition/references/index 확인:
  - `cargo run -p refraction --bin prism -- hir . --json`
  - `cargo run -p refraction --bin prism -- definition . --json --file <file> --line <n> --col <n>`
  - `cargo run -p refraction --bin prism -- references . --json --file <file> --line <n> --col <n>`
  - `cargo run -p refraction --bin prism -- index . --json --file <file> --line <n> --col <n>`

### VS Code extension
- 테스트 진입점: `cd vscode-prsm; npm test`
- 패키징 확인: `cd vscode-prsm; npm run package`
- 패키지 검증: `cd vscode-prsm; npm run verify`
- 설치 스모크: `cd vscode-prsm; npm run verify:install`

### Unity and BlazeTest
- 외부 프로젝트 스모크: `powershell -ExecutionPolicy Bypass -File .\run-blazetest-smoke.ps1 -ProjectPath C:\Users\idene\BlazeTest`
- 통합 검증: `powershell -ExecutionPolicy Bypass -File .\run-verification.ps1`

## 선택 규칙

- 파일 단위 증상은 가능한 한 CLI 명령과 특정 샘플 파일로 좁힌다.
- 확장 UX 문제는 `npm test`로 먼저 확인하고, 설치/패키징 증상일 때만 `package`, `verify`, `verify:install`로 올린다.
- Unity 증상은 외부 BlazeTest 환경, stale artifact, generated `.cs`, `.prsmmap.json` 경로를 같이 본다.
- 전체 검증은 조사 초반이 아니라 경계면이 여러 개이거나 수정 종료 직전에 사용한다.

## 기록 규칙

- 실행한 명령은 전체 문자열을 그대로 남긴다.
- 명령이 실패하면 종료 코드와 표준 출력 핵심 부분을 함께 기록한다.
- 명령을 생략하면 왜 생략했는지 적는다.