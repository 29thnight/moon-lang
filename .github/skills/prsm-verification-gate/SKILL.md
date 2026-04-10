---
name: prsm-verification-gate
description: 'Verification and closure workflow for PrSM issue work. Use when deciding what tests to run, how much evidence is enough, whether a fix really closed the bug, or whether residual risk remains. Trigger on verify, validate, regression scope, exit criteria, closure, install smoke, packaging check, or full repository verification decisions.'
argument-hint: 'Describe the fix, suspected regression scope, or verification question'
---

# PrSM Verification Gate

문제의 종료 기준을 정하고, 어떤 검증이 충분한지 판단한다.

## 사용할 때
- 수정 후 무엇을 어디까지 돌려야 할지 정해야 할 때
- 특정 회귀가 한 표면에만 국한되는지 확인해야 할 때
- 전체 검증을 돌릴지 focused verification으로 끝낼지 결정해야 할 때

## 검증 계층

1. Focused local proof
   - 단일 `cargo test` 또는 특정 `prism` CLI 명령
   - `cd vscode-prsm; npm test`
2. Packaging proof
   - `cd vscode-prsm; npm run package`
   - `cd vscode-prsm; npm run verify`
3. Install smoke proof
   - `cd vscode-prsm; npm run verify:install`
4. Cross-surface proof
   - `powershell -ExecutionPolicy Bypass -File .\run-blazetest-smoke.ps1 -ProjectPath C:\Users\idene\BlazeTest`
   - `powershell -ExecutionPolicy Bypass -File .\run-verification.ps1`

## 선택 규칙

- compiler-only fix는 focused local proof부터 시작한다.
- extension UI logic or provider fix는 `npm test`가 기본이다.
- 번들 바이너리, VSIX, 설치 문제는 packaging proof와 install smoke proof까지 올린다.
- Unity remap, external project, source-map chain 문제는 cross-surface proof가 필요할 수 있다.
- 실행하지 않은 계층은 `not run`으로 남기고 이유를 적는다.

## 출력 규약

[verification report template](./assets/verification-report-template.md) 형식을 따른다.

기본 출력 파일은 `_workspace/issues/<issue-key>/04_verify.md`이다.

## 종료 기준

- 재현 증상이 사라졌는가
- 영향 표면에서 필요한 계층 검증이 끝났는가
- 미실행 검증이 있다면 잔여 리스크가 명시됐는가
- 사용자가 다시 재현할 수 있는 절차가 남아 있는가

## 금지 패턴

- `테스트 일부만 했지만 아마 괜찮다` 같은 모호한 종료
- 실패한 검증을 생략으로 기록
- cross-surface 버그를 focused proof만으로 닫기