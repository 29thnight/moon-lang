---
title: Pattern Matching & Control Flow
parent: 언어 가이드
grand_parent: 한국어 문서
nav_order: 7
---

# Pattern Matching & Control Flow

## `when`

현재 PrSM의 패턴 매칭은 `when` 중심으로 구현되어 있습니다.

```prsm
when state {
    EnemyState.Idle => idle()
    EnemyState.Chase => chase()
    else => attack()
}
```

조건식 기반 `when` 도 지원합니다.

```prsm
when {
    hp <= 0 => die()
    else => run()
}
```

의미 분석 단계는 가능한 경우 완전성 검사를 수행합니다.

## `if`, `for`, `while`

PrSM 제어문은 괄호 없이 중괄호 기반으로 작성합니다.

```prsm
if hp <= 0 {
    die()
} else {
    run()
}

for i in 0 until count {
    tick(i)
}

while alive {
    updateState()
}
```

`if`, `when` 표현식을 지원하며, `break`, `continue` 도 구현되어 있습니다.
