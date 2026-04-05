---
title: VS Code Extension
parent: 도구
grand_parent: 한국어 문서
nav_order: 3
---

# VS Code Extension

`vscode-prsm` 패키지는 전용 LSP 서버 없이도 컴파일러 기반 편집 워크플로를 제공합니다.

## 현재 기능

- TextMate 구문 강조와 스니펫
- Unity 친화 자동완성과 프로젝트 기반 completion helper
- trusted workspace 에서 열기, 저장, 디바운스된 편집 시점마다 동작하는 `prism check --json` 기반 diagnostics
- 컴파일러 CLI 질의를 이용한 definition, hover, references, rename
- 캐시된 `prism index` 결과를 이용한 document / workspace symbol
- `.prsmproject` 기반 프로젝트 탐색
- `.prsmmap.json` 기반 generated C# 내비게이션
- PrSM 소스에서 generated C# 로, generated C# 에서 다시 원본 PrSM 으로 이동하는 보조 명령
- explorer, graph view, lifecycle 삽입, compile/check 명령

## Trusted Workspace 동작

확장은 어디서나 기본 문법 자산과 편집 보조는 제공하지만, 컴파일러 기반 기능은 trusted workspace 에서만 활성화됩니다.

영향을 받는 항목은 diagnostics, 내비게이션, rename, symbol 조회, compile 계열 명령입니다.

## 내비게이션 모델

이 확장은 의도적으로 compiler-backed 구조를 사용합니다.

- definition 은 우선 `prism definition` 을 사용
- hover 와 일부 fallback 내비게이션은 `prism index` 를 사용
- references 와 rename 은 Typed HIR 기반 `prism references` 를 사용
- generated C# 탐색은 먼저 `.prsmmap.json` sidecar 를 보고, 필요하면 더 느슨한 휴리스틱으로 내려감

전용 LSP 서버는 여전히 미래 작업이지만, 현재 확장만으로도 실제 프로젝트의 핵심 작성 루프는 이미 성립합니다.
