---
name: prsm-issue-triage
description: 'Issue intake and triage workflow for PrSM. Use when turning a bug report, smoke failure, flaky test, stack trace, or regression note into severity, labels, impacted surfaces, and first investigation steps. Trigger on bug triage, issue grooming, severity, labeling, scope, or missing reproduction details.'
argument-hint: 'Paste the bug report, failure note, or raw symptom to triage'
---

# PrSM Issue Triage

원시 이슈를 조사 가능한 브리프로 정리한다.

## 사용할 때
- 사용자가 현상만 설명하고 아직 조사 단위로 정리되지 않았을 때
- 어떤 서브시스템을 먼저 볼지 결정해야 할 때
- 재현 전에도 심각도와 영향 범위를 정리해야 할 때

## 절차

1. 증상과 기대 동작을 분리한다.
2. 실제 증거를 수집한다.
   - 실패 명령
   - 스택 트레이스
   - 파일 경로
   - 최근 변경 추정
3. 이슈를 저장소 표면에 매핑한다.
   - compiler or CLI
   - vscode-prsm
   - unity-package
   - source map or generated C#
   - packaging or install smoke
   - BlazeTest external project
4. 심각도를 결정한다.
   - `P0`: 작업 전체가 막히거나 데이터 손상 위험
   - `P1`: 핵심 기능 회귀, 우회 어려움
   - `P2`: 부분 기능 실패, 우회 가능
   - `P3`: 저위험 불편 또는 진단 품질 문제
5. 라벨을 붙인다.
   - `area/compiler`
   - `area/vscode`
   - `area/unity`
   - `area/source-map`
   - `area/packaging`
   - `kind/regression`
   - `kind/flaky`
   - `needs-repro`
   - `needs-blazetest`
6. 빠진 증거를 명시한다.
7. [issue brief template](./assets/issue-brief-template.md) 형식으로 정리한다.

## 출력 규약

- 기본 출력 파일: `_workspace/issues/<issue-key>/01_triage.md`
- 재현이 불가능한 경우에도 `missing evidence`를 반드시 채운다.
- 추측은 `hypothesis`로 표시하고 사실과 분리한다.

## 참고

- 표면 분류 기준은 [component map](./references/component-map.md)을 따른다.

## 트리아지 완료 기준

- 심각도가 있다.
- 영향 표면이 있다.
- 첫 번째 조사 단계가 있다.
- 누락 증거가 남아 있다면 무엇을 더 받아야 하는지 적혀 있다.