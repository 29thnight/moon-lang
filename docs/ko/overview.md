---
title: 개요
parent: 시작하기 (KO)
nav_order: 1
---

# 개요

PrSM은 `.prsm` 소스 파일을 Unity용 생성 C#으로 컴파일하는 언어 도구체인입니다. 컴파일 과정에서는 `.prsmmap.json` 사이드카도 함께 생성되어 진단과 탐색을 원본 소스로 다시 연결합니다.

저장소는 네 가지 축으로 구성됩니다.

- `crates/refraction`: 컴파일러 코어와 `prism` CLI
- `unity-package`: Unity Editor 통합과 생성 코드 워크플로 지원
- `vscode-prsm`: 구문 강조, 진단, 탐색, 스니펫, 생성 코드 소스맵 도우미
- `samples`: 로컬 검증 및 회귀용 예제 스크립트

PrSM은 범용 언어를 목표로 하지 않습니다. 핵심 목적은 Unity 게임플레이 코드를 더 짧게 쓰게 하면서 null-safety, 라이프사이클 문법, 컴포넌트 룩업, 코루틴, 읽기 쉬운 생성 C#을 제공하는 것입니다.
