---
title: Package Layout
parent: 내부 구조
grand_parent: 한국어 문서
nav_order: 7
---

# Package Layout

현재 저장소의 상위 레이아웃:

- `crates/refraction`: compiler crate 와 `prism` binary
- `unity-package`: Unity package source, editor integration, template, test
- `vscode-prsm`: extension source, grammar, snippet, theme, test
- `samples`: 예제 `.prsm` 스크립트와 generated comparison
- `tests`: invalid / negative source fixture
- `build-output`: sample output 및 smoke artifact
- `plan_docs`: roadmap, spec, architecture, design note
- `docs`: GitHub Pages 지향 문서 트리

이 분리는 실제 제품 형태를 그대로 반영합니다. 즉, 언어 컴파일러, Unity 통합, 에디터 도구가 한 저장소에서 함께 개발됩니다.
