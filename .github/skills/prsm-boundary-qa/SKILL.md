---
name: prsm-boundary-qa
description: 'Cross-component QA workflow for PrSM boundaries. Use when a bug may live between compiler output and VS Code behavior, between source-map sidecars and Unity remapping, between packaged binaries and workspace builds, or between repo assumptions and external BlazeTest projects. Trigger on boundary mismatch, contract drift, source-map bug, hover mismatch, stack-trace remap failure, packaging drift, or cross-component regression.'
argument-hint: 'Describe the producer, consumer, and failing behavior you want compared'
---

# PrSM Boundary QA

이 스킬은 한 컴포넌트가 만든 결과를 다른 컴포넌트가 잘못 읽는 문제를 찾는 데 쓴다.

## 핵심 원칙

- 항상 생산자와 소비자를 같이 읽는다.
- 존재 여부만 보지 말고 shape, path, span, filename, project-root 가정을 같이 비교한다.
- 단일 컴포넌트 테스트 통과를 경계면 정상의 근거로 쓰지 않는다.

## 주요 경계

### Compiler -> VS Code extension
- diagnostic span
- definition/reference/index JSON
- generated output path
- chosen prism binary

### Compiler -> source-map sidecar -> VS Code and Unity
- `.prsmmap.json` existence
- anchor span and nested segment precision
- original `.prsm` path and generated `.cs` path

### VS Code extension -> packaged artifact
- workspace dev binary vs bundled `bin/prism.exe`
- package output vs local build output

### Unity package -> runtime stack trace remap
- generated `.cs` frame parsing
- remapped `Assets/...prsm(line,col)` location
- one-shot cache or stale sidecar behavior

### Repo -> BlazeTest external project
- active document project root
- external project path assumptions
- stale generated output or compatibility fallback usage

## 절차

1. 실패 동작의 생산자와 소비자를 이름으로 적는다.
2. 각 쪽의 근거 파일과 함수 또는 스크립트를 찾는다.
3. 다음 체크리스트를 따라 비교한다.
4. mismatch가 있으면 어느 쪽이 계약을 깨는지 좁힌다.
5. 결과를 `_workspace/issues/<issue-key>/03_boundary.md`에 남긴다.

## 체크리스트

[boundary checklist](./assets/boundary-checklist.md)을 사용한다.

## 출력 규약

- boundary name
- producer path
- consumer path
- expected contract
- observed contract
- mismatch or confirmed match
- next action