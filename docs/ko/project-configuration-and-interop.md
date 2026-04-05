---
title: Project Configuration & Interop
parent: 고급 주제
grand_parent: 한국어 문서
nav_order: 1
---

# Project Configuration & Interop

프로젝트 단위 설정은 `.prsmproject` 로 관리됩니다.

현재 설정 범위:

- 프로젝트 식별자
- language version 과 feature flag
- compiler path 와 output directory
- source include/exclude pattern

Interop 의 중심은 읽기 쉬운 생성 C# 입니다.

- component 는 일반 Unity component class 로 lowering
- coroutine 은 `IEnumerator` 로 lowering
- asset 는 ScriptableObject 기반 class 로 lowering
- enum payload 는 generated extension method 로 노출

컴파일러 내부에는 과거 명명에 대한 호환 fallback 이 일부 남아 있지만, 활성 포맷은 `.prsmproject` 입니다.
