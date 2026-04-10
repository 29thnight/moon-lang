---
name: prsm-bug-hunting
description: 'Bug reproduction and localization workflow for PrSM. Use when reproducing failures, narrowing regressions, following stack traces, identifying suspect files, creating minimal repros, or deciding the smallest command that proves the bug. Trigger on repro, isolate, regression hunt, flaky failure, stack trace, broken command, or minimal reproduction requests.'
argument-hint: 'Describe the symptom, target file, failing command, or regression you want reproduced'
---

# PrSM Bug Hunting

버그를 실제로 재현하고 가장 작은 원인 표면으로 좁힌다.

## 사용할 때
- 사용자가 `왜 깨졌는지 찾아줘`, `재현해줘`, `어디서 망가졌는지 좁혀줘` 같은 요청을 할 때
- 스택 트레이스나 실패 명령만 있고 원인 위치가 모호할 때
- 전체 검증을 돌리기 전에 좁은 재현 경로를 만들고 싶을 때

## 절차

1. 증상을 가장 작은 표면으로 변환한다.
   - 파일 하나인지
   - 명령 하나인지
   - 특정 편집기 기능인지
   - Unity 외부 프로젝트 재현인지
2. 가장 좁은 명령부터 선택한다.
   - compiler: `cargo test`, `cargo run -p refraction --bin prism -- ...`
   - extension: `cd vscode-prsm; npm test`
   - packaging: `cd vscode-prsm; npm run package`, `npm run verify`, `npm run verify:install`
   - unity smoke: `powershell -ExecutionPolicy Bypass -File .\run-blazetest-smoke.ps1 -ProjectPath ...`
3. 재현 입력을 최소화한다.
   - 단일 sample 파일
   - 현재 실패하는 `.prsm` 파일
   - stack trace 한 줄
   - 단일 VS Code command or provider flow
4. 실행 결과를 기록한다.
   - command
   - cwd
   - observed behavior
   - expected behavior
5. 의심 파일과 계약을 좁힌다.
6. 수정 요청이 있으면 최소 표면만 바꾸고 focused verification을 다시 실행한다.

## 흔한 핫스팟

[common hotspots](./references/hotspots.md)을 먼저 확인한다.

## 출력 규약

- 기본 출력 파일: `_workspace/issues/<issue-key>/02_repro.md`
- 최소 포함 항목:
  - exact command
  - repro input
  - pass or fail
  - suspect files
  - hypothesis
  - next verification

## 금지 패턴

- 재현 없이 바로 대규모 수정
- 증거 없이 `아마 이 파일` 식 결론
- 초기 조사 단계에서 곧바로 `run-verification.ps1`