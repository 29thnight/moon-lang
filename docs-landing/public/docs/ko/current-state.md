---
title: Current State
parent: 내부 구조
grand_parent: 한국어 문서
nav_order: 2
---

# Current State

현재 저장소 기준 상태:

- lexer, parser, semantic analysis, lowering, code generation 구현 완료
- `prism` CLI 구현 및 저장소 내부 검증 완료
- Unity package integration 구현 완료
- definition, hover, references, rename, symbol 을 포함한 compiler-backed VS Code 내비게이션 구현 완료
- `.prsmmap.json` 기반 generated C# 역매핑이 VS Code 확장과 Unity package 양쪽에 구현 완료
- 이벤트 리스너 sugar 와 intrinsic escape hatch 경로를 포함한 저장소 내부 lowering 회귀 커버리지 보강
- BlazeTest smoke coverage 와 package-level editor test 존재

아직 미완인 부분:

- 더 넓은 negative test coverage
- 전용 LSP 서버
- 현재 기반을 넘는 더 깊은 debugging / source-map 워크플로
