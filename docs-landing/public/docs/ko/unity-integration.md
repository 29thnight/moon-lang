---
title: Unity Integration
parent: 도구
grand_parent: 한국어 문서
nav_order: 2
---

# Unity Integration

Unity 패키지는 이미 `.prsm` 파일을 위한 실용적인 import/build 워크플로를 구현하고 있습니다.

현재 담당하는 역할:

- Unity 내부에서 PrSM 소스 import 및 compile
- 생성된 C# 출력 처리
- compiler resolution helper
- project settings 연동
- source-map 기반 진단 remapping
- 템플릿과 editor menu action

즉, 이것은 단순한 설계 문서가 아니라 저장소 안에 실제 코드와 smoke 검증이 있는 구현 영역입니다.
