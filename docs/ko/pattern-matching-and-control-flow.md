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

의미 분석 단계는 가능한 경우 완전성 검사를 수행합니다 (누락된 variant에 대해 경고 W003).

### 패턴 바인딩 (PrSM 2 부터)

Enum payload 바인딩은 파라미터화된 enum 엔트리에서 데이터를 추출합니다:

```prsm
enum EnemyState(val target: String) {
    Idle(""),
    Chase("player"),
    Stunned("player")
}

when state {
    EnemyState.Idle => idle()
    EnemyState.Chase(target) => moveTo(target)
    EnemyState.Stunned(duration) if duration > 0.0 => wait(duration)
}
```

생성 C#은 튜플 스타일 접근을 사용합니다:

```csharp
case EnemyState.Chase _prsm_m8_5:
    var target = _prsm_m8_5.Item1;
    moveTo(target);
    break;
```

**규칙:**
- 바인딩 수는 enum 파라미터 수와 일치해야 합니다 (불일치 시 에러 E082)
- variant 이름은 enum에 존재해야 합니다 (알 수 없는 variant 시 에러 E081)
- 빈 바인딩 `EnemyState.Idle`은 추출 없이 매칭합니다

### when 가드 (PrSM 2 부터)

가드는 패턴 뒤에 조건을 추가합니다:

```prsm
when state {
    EnemyState.Stunned(duration) if duration > 0.0 => wait(duration)
    EnemyState.Stunned(duration) => recover()
}
```

가드 표현식은 패턴 매칭 후 평가됩니다. C# 출력에서 `&&` 조건으로 생성됩니다.

### `val` 구조 분해 (PrSM 2 부터)

Data class 인스턴스를 개별 변수로 구조 분해할 수 있습니다:

```prsm
data class PlayerStats(hp: Int, speed: Float)

val PlayerStats(hp, speed) = getStats()
```

생성 C#:

```csharp
var _prsm_d = getStats();
var hp = _prsm_d.hp;
var speed = _prsm_d.speed;
```

**규칙:**
- 바인딩 수는 data class 필드 수와 일치해야 합니다 (에러 E082)
- 바인딩 이름은 로컬 변수 이름으로 사용됩니다

### `for` 구조 분해 (PrSM 2 부터)

동일한 구조 분해 문법이 `for` 루프에서도 동작합니다:

```prsm
for Spawn(pos, delay) in wave.spawns {
    spawnAt(pos, delay)
}
```

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
