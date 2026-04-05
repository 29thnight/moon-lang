---
title: Data Models & Attributes
parent: 언어 가이드
grand_parent: 한국어 문서
nav_order: 6
---

# Data Models & Attributes

## Data class

PrSM에는 현재 `struct` 키워드가 없습니다. 대신 구현된 데이터 모델 기능은 `data class` 입니다.

```prsm
data class DamageInfo(
    val amount: Int,
    val crit: Bool
)
```

이 선언은 public field, 생성자, `Equals`, `GetHashCode`, `ToString` 을 갖는 직렬화 가능한 C# 클래스로 내려갑니다.

## Enum

```prsm
enum EnemyState {
    Idle,
    Chase,
    Attack
}
```

파라미터가 있는 enum 도 지원하며, payload 접근용 확장 메서드가 함께 생성됩니다.

## Attribute

```prsm
@targets(Method, Property)
attribute Cooldown(
    val duration: Float,
    val resetOnHit: Bool
)
```

이 형태는 생성자와 `AttributeUsage` 메타데이터를 포함한 C# attribute 클래스로 lowering 됩니다.
