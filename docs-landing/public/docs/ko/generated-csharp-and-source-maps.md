---
title: Generated C# & Source Maps
parent: 도구
grand_parent: 한국어 문서
nav_order: 4
---

# Generated C# & Source Maps

PrSM은 원본을 읽기 쉬운 생성 C# 으로 내리고, 역매핑을 위한 source-map sidecar 도 함께 만듭니다.

산출물:

- source: `.prsm`
- generated code: `.cs`
- sidecar source map: `.prsmmap.json`

이 소스맵은 현재 컴파일러 워크플로, VS Code 확장, Unity 통합에서 원본 진단 및 탐색을 되돌리는 데 이미 사용되고 있습니다.
