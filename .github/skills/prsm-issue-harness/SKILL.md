---
name: prsm-issue-harness
description: 'Issue management and bug investigation harness for PrSM. Use when triaging issues, hunting bugs, chasing regressions, analyzing smoke failures, parsing stack traces, localizing compiler or VS Code or Unity mismatches, or planning fix verification. Trigger on issue triage, repro, root cause analysis, flaky test, smoke failure, packaging bug, source-map bug, install smoke bug, or "why is this broken" requests.'
argument-hint: 'Describe the issue, failing command, stack trace, or investigation goal'
---

# PrSM Issue Harness

PrSM 저장소의 이슈 관리, 재현, 경계면 분석, 검증을 한 흐름으로 운영하는 작업용 스킬이다.

## 사용할 때
- 버그 리포트를 조사 가능한 형태로 바꿔야 할 때
- 회귀, 스모크 실패, 스택 트레이스, 패키징 실패 원인을 좁혀야 할 때
- 컴파일러, VS Code 확장, Unity 패키지, BlazeTest 외부 프로젝트 중 어느 경계에서 깨졌는지 찾아야 할 때
- 수정 전 조사와 수정 후 검증을 같은 흐름으로 묶어야 할 때

## 에이전트 구성

| 에이전트 | 역할 | 기본 산출물 |
| --- | --- | --- |
| `prsm-issue-triage` | 이슈 접수, 심각도, 범위, 증거 격차 정리 | `01_triage.md` |
| `prsm-bug-hunter` | 재현, 최소 재현 입력, 의심 파일 축소 | `02_repro.md` |
| `prsm-boundary-auditor` | 생산자/소비자 경계 비교 | `03_boundary.md` |
| `prsm-verification-gate` | 검증 범위 결정, 종료 기준 판단 | `04_verify.md` |

## 저장 규약

이슈마다 `_workspace/issues/<issue-key>/` 폴더를 만들고 다음 파일명을 유지한다.

| 파일 | 목적 |
| --- | --- |
| `00_issue.md` | 원본 증상과 요약 |
| `01_triage.md` | 심각도, 라벨, 범위, 증거 공백 |
| `02_repro.md` | 재현 절차, 실제 결과, 의심 모듈 |
| `03_boundary.md` | 경계면 비교 결과 |
| `04_verify.md` | 검증 범위와 실행 결과 |
| `05_summary.md` | 최종 상태 요약 |

초기 문서는 [issue summary template](./assets/issue-summary-template.md)을 기반으로 작성한다.

## 표준 절차

1. 이슈 키를 정한다.
   - 권장 형식: `YYYYMMDD-short-slug`
   - 예: `20260407-hover-stale-binary`
2. `_workspace/issues/<issue-key>/00_issue.md`를 만든다.
3. `prsm-issue-triage`로 다음을 정리한다.
   - 증상
   - 기대 동작
   - 실제 동작
   - 영향 표면
   - 누락된 증거
4. `prsm-bug-hunter`로 재현을 시도한다.
   - 가능한 한 전체 검증 대신 좁은 명령부터 사용한다.
   - 재현 명령, 입력 파일, 관찰 결과를 그대로 기록한다.
5. 증상이 컴포넌트 경계에 걸리면 `prsm-boundary-auditor`를 병행한다.
   - 생성된 `.prsmmap.json`
   - compiler JSON span
   - VS Code의 prism binary 선택
   - Unity remap 경로
   - BlazeTest 프로젝트 루트 가정
6. 수정 요청이 있으면 가장 작은 원인 표면부터 수정한다.
7. `prsm-verification-gate`로 종료 기준을 정리한다.
   - focused check
   - package/verify
   - install smoke
   - BlazeTest smoke
8. `05_summary.md`에 상태를 남긴다.

## 리포지토리 명령 선택

명령 선택 기준은 [repo command matrix](./references/repo-command-matrix.md)를 따른다.

핵심 원칙:
- Rust 컴파일러 증상은 `cargo test` 또는 `cargo run -p refraction --bin prism -- ...` 같은 좁은 명령부터 시작한다.
- VS Code 증상은 `vscode-prsm` 폴더에서 `npm test`를 우선하고, 패키징/설치 문제가 아니면 `npm run package` 이후 단계는 늦춘다.
- Unity/BlazeTest 관련 증상은 외부 프로젝트 경로와 에디터 실행 상태를 먼저 확인한다.
- `run-verification.ps1`는 경계면이 여러 개이거나 종료 직전일 때 사용한다.

## 상태 정의

- `new`: 접수만 됨
- `triaged`: 범위와 증거 공백이 정리됨
- `reproducible`: 재현 가능
- `isolated`: 원인 표면이 좁혀짐
- `fixed-pending-verify`: 수정은 했지만 검증 미완료
- `verified`: 종료 기준 충족
- `blocked`: 추가 정보나 외부 환경이 필요함

## 테스트 시나리오

### 정상 흐름
1. 사용자가 `hover가 이전 바이너리를 보는 것 같다`고 보고한다.
2. `00_issue.md`와 `01_triage.md`에 증상과 영향 표면을 기록한다.
3. `prsm-bug-hunter`가 `vscode-prsm`의 compiler resolution과 개발 빌드 우선순위를 확인한다.
4. `prsm-boundary-auditor`가 확장 번들 바이너리와 워크스페이스 빌드 바이너리 소비 경계를 비교한다.
5. 수정 후 `npm test`와 필요한 경우 `npm run verify`를 실행한다.
6. `04_verify.md`와 `05_summary.md`를 남기고 종료한다.

### 에러 흐름
1. 사용자가 BlazeTest 스모크 실패를 보고하지만 프로젝트가 이미 Unity에서 열려 있다.
2. `prsm-bug-hunter`가 재현을 시도하다 환경 블로커를 확인한다.
3. `01_triage.md`와 `02_repro.md`에 환경 블로커를 기록한다.
4. `prsm-verification-gate`가 `blocked` 상태와 필요한 외부 조치를 명시한다.
5. Unity 실행 해제 전까지 전체 종료로 오판하지 않는다.