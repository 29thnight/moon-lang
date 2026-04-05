---
title: Project Files & Imports
parent: 언어 가이드
grand_parent: 한국어 문서
nav_order: 12
---

# Project Files & Imports

PrSM은 아직 Zephyr식 독립 모듈 시스템을 제공하지 않습니다. 현재 구현된 범위는 다음과 같습니다.

- `using` 을 통한 네임스페이스 임포트
- `.prsmproject` 기반 프로젝트 탐색
- source include/exclude glob
- compiler output directory 설정

임포트 예시:

```prsm
using UnityEngine
using UnityEngine.UI
using UnityEngine.SceneManagement
```

최소 프로젝트 파일 예시:

```toml
[project]
name = "PrSMDemo"
prsm_version = "0.1.0"

[compiler]
output_dir = "build-output"
```
