---
title: Declarations & Fields
parent: 언어 가이드
grand_parent: 한국어 문서
nav_order: 5
---

# Declarations & Fields

## 최상위 선언

현재 PrSM이 지원하는 선언은 다음과 같습니다.

- `component`
- `asset`
- `class`
- `data class`
- `enum`
- `attribute`

각 파일에는 정확히 하나의 최상위 선언만 들어갑니다.

## 직렬화 필드

```prsm
@header("Movement")
serialize speed: Float = 5.0
```

이 형태는 Unity 직렬화를 유지하면서도 속성형 API를 제공하는 C#으로 내려갑니다.

## 일반 필드

- `val` 은 불변 필드/로컬
- `var` 는 가변 필드/로컬
- `public`, `private`, `protected` 가시성 지원

## 컴포넌트 룩업 필드

아래 형태는 `component` 에서만 지원합니다.

- `require name: Type`
- `optional name: Type`
- `child name: Type`
- `parent name: Type`

이들은 lowering 단계에서 생성되는 `Awake()` 로직으로 조립됩니다.
