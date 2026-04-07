---
title: 개요
parent: 시작하기 (KO)
nav_order: 1
---

# 개요

PrSM(**P**ragmatic **R**educed **S**yntax for **M**etascript)은 `.prsm` 소스 파일을 Unity용 생성 C#으로 컴파일하는 언어 도구체인입니다. 컴파일 과정에서는 `.prsmmap.json` 사이드카도 함께 생성되어 진단과 탐색을 원본 소스로 다시 연결합니다.

저장소는 네 가지 축으로 구성됩니다.

- `crates/refraction`: 컴파일러 코어와 `prism` CLI
- `unity-package`: Unity Editor 통합과 생성 코드 워크플로 지원
- `vscode-prsm`: 구문 강조, 진단, 탐색, 스니펫, 생성 코드 소스맵 도우미
- `samples`: 로컬 검증 및 회귀용 예제 스크립트

PrSM은 범용 언어를 목표로 하지 않습니다. 핵심 목적은 Unity 게임플레이 코드를 더 짧게 쓰게 하면서 null-safety, 라이프사이클 문법, 컴포넌트 룩업, 코루틴, 읽기 쉬운 생성 C#을 제공하는 것입니다.

## 언어 버전

| 버전 | 도구 릴리스 | 주요 기능 |
|------|------------|----------|
| [PrSM 1](spec/lang-1.md) | Prism v0.x | 핵심 component 모델, 라이프사이클 블록, `serialize`/`require`, `listen`, 코루틴 |
| [PrSM 2](spec/lang-2.md) | Prism v0.x | 패턴 바인딩, listen 수명, 구조 분해, Input System sugar, 제네릭 추론 |
| [PrSM 3](spec/lang-3.md) | Prism v1.0.0 | 인터페이스, 제네릭, `singleton`, `pool`, SOLID 경고, 옵티마이저 |
| [PrSM 4](spec/lang-4.md) | Prism v2.0.0 | 30개 기능: try/catch, 람다, 프로퍼티, struct, async/await, state machine, command, bind, 옵티마이저 v4, Burst 분석, Rust/Elm 스타일 에러, 리팩토링 도구, 디버거 소스 맵 |

각 버전은 완전히 하위 호환됩니다 — 이전 버전의 프로그램은 변경 없이 컴파일됩니다. 업그레이드 절차는 [버전별 마이그레이션](migration-v1-to-v2.md)을 참조하세요.
