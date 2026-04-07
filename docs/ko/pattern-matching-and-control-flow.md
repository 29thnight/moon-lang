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

### OR 패턴 (PrSM 4 부터)

`when` 분기에서 쉼표로 구분된 여러 패턴은 개별 패턴 중 하나라도 매치되면 매치됩니다. OR 그룹의 모든 분기는 같은 변수 집합을 바인딩해야 합니다 (또는 어느 것도 바인딩하지 않아야 합니다).

```prsm
when direction {
    Direction.Up, Direction.Down    => handleVertical()
    Direction.Left, Direction.Right => handleHorizontal()
}
```

생성 C#:

```csharp
switch (direction) {
    case Direction.Up:
    case Direction.Down:
        handleVertical();
        break;
    case Direction.Left:
    case Direction.Right:
        handleHorizontal();
        break;
}
```

다른 변수를 바인딩하는 OR 패턴 분기는 E130을 발생시킵니다.

### 범위 패턴 (PrSM 4 부터)

`when` 분기 안의 `in low..high`는 닫힌 범위 `[low, high]` 내의 값을 매치합니다. 정수형과 부동소수점 타입만 지원됩니다.

```prsm
when score {
    in 90..100 => "A"
    in 80..89  => "B"
    in 70..79  => "C"
    else       => "F"
}
```

`low > high`인 범위는 E131을 발생시킵니다. 겹치는 범위 패턴은 W023을 발생시킵니다.

### `when`의 스마트 캐스트 (PrSM 4 부터)

`is` 분기가 매치된 후 분기 본문 내에서 주체는 검사된 타입으로 좁혀집니다:

```prsm
when target {
    is Enemy => target.takeDamage(10)
    is Ally  => target.heal(5)
}
```

## `try` / `catch` / `finally` (PrSM 4 부터)

예외가 일급 시민이 됩니다. `throw`에서 `new` 키워드는 생략됩니다. `try`는 정확히 하나의 `catch` 절이 있을 때 표현식으로 사용할 수 있습니다.

```prsm
try {
    val data = File.readAllText(path)
} catch (e: FileNotFoundException) {
    warn(e.message)
} catch (e: Exception) {
    error(e.message)
} finally {
    cleanup()
}

throw ArgumentException("Invalid value")

val result = try { parseInt(str) } catch (e: Exception) { -1 }
```

생성 C#:

```csharp
try
{
    var data = File.ReadAllText(path);
}
catch (FileNotFoundException e) { Debug.LogWarning(e.Message); }
catch (Exception e) { Debug.LogError(e.Message); }
finally { Cleanup(); }

throw new ArgumentException("Invalid value");
```

상위 절에서 이미 잡힌 타입의 `catch` 절은 E100을 발생시킵니다. 비-Exception 표현식의 `throw`는 E101을 발생시킵니다. 빈 `catch` 블록은 W020을 발생시킵니다.

## `use` (IDisposable) (PrSM 4 부터)

`use`는 `IDisposable` 리소스의 자동 해제를 보장합니다. 블록 형식은 블록 종료 시점에, 선언 형식은 둘러싼 스코프 종료 시점에 해제합니다.

```prsm
use stream = FileStream(path, FileMode.Open) {
    val data = stream.readToEnd()
}

use val conn = DbConnection(connString)
// 스코프 종료 시 conn 자동 해제
```

C# `using` 문 (블록 형식) 또는 `using` 선언 (`use val`)으로 변환됩니다. IDisposable을 구현하지 않는 타입에 `use`를 사용하면 E119가 발생합니다.

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
