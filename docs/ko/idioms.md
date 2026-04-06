---
title: Idioms & Patterns
parent: 언어 가이드
grand_parent: 한국어 문서
nav_order: 14
---

# 관용구 & 패턴

이 페이지는 PrSM 개발에서 권장되는 패턴과 흔한 안티패턴을 모았습니다. 각 섹션에는 짧은 코드 예제가 포함되어 있습니다.

## 컴포넌트 설계

컴포넌트는 하나의 책임에 집중하세요. 런타임에 반드시 존재해야 하는 의존성에는 `require`를, 있을 수도 없을 수도 있는 의존성에는 `optional`을 사용합니다.

```prsm
component DamageReceiver : MonoBehaviour {
    require collider: Collider
    optional animator: Animator

    serialize maxHp: Int = 100
    var hp: Int = maxHp

    func takeDamage(amount: Int) {
        hp = Math.Max(0, hp - amount)
        animator?.SetTrigger("Hit")
        if hp <= 0 {
            die()
        }
    }
}
```

이동, 입력, 체력, UI, 오디오를 하나의 선언에서 처리하는 "만능 컴포넌트"는 피하세요. 별도의 컴포넌트로 분리하고 이벤트나 공유 `asset` 데이터를 통해 통신합니다.

## 이벤트 구독 패턴

### 권장 — `until disable`로 자동 정리

v2 컴포넌트에서 가장 안전한 패턴입니다. 컴포넌트가 비활성화되면 리스너가 자동으로 제거되어, 파괴된 오브젝트에서 오래된 콜백이 호출되는 것을 방지합니다.

```prsm
component ShopUI : MonoBehaviour {
    require buyButton: Button
    require sellButton: Button

    listen buyButton.onClick until disable {
        purchaseSelectedItem()
    }

    listen sellButton.onClick until disable {
        sellSelectedItem()
    }
}
```

### 임시 리스너 — 수동 수명

제한된 시간 동안만 활성화해야 하는 리스너는 토큰을 캡처하고 완료 시 unlisten합니다.

```prsm
component Tutorial : MonoBehaviour {
    require skipButton: Button

    var skipToken: ListenToken? = null

    func startTutorial() {
        skipToken = listen skipButton.onClick manual {
            endTutorial()
        }
    }

    func endTutorial() {
        if skipToken != null {
            unlisten skipToken!!
            skipToken = null
        }
    }
}
```

### 안티패턴 — v2 컴포넌트에서 등록 전용

v2에서 `auto-unlisten`이 활성화된 경우, 수명 수정자 없는 `listen`은 기본적으로 `until disable`로 동작합니다. 의도적으로 등록 전용 시맨틱이 필요하다면 `manual`을 명시하고 이유를 문서화하세요.

## 코루틴 패턴

### 시간 기반 동작

타이머를 수동으로 추적하는 대신 `wait`에 지속 시간을 사용합니다.

```prsm
coroutine flashDamage() {
    spriteRenderer.color = Color.red
    wait 0.15s
    spriteRenderer.color = Color.white
}
```

### `wait until`을 사용한 폴링

수동 `Update` 기반 검사를 `wait until`로 대체하면 의도가 더 명확해집니다.

```prsm
coroutine waitForDoorOpen() {
    wait until door.isOpen
    playOpenAnimation()
}
```

### 다단계 시퀀스

`wait` 구문을 연결하여 스크립트 시퀀스를 만듭니다. 각 단계가 위에서 아래로 읽힙니다.

```prsm
coroutine introSequence() {
    fadeIn(title)
    wait 2.0s
    fadeOut(title)
    wait 0.5s
    fadeIn(subtitle)
    wait 3.0s
    fadeOut(subtitle)
    loadGameplay()
}
```

### 안티패턴 — wait 없는 무한 루프

yield 없이 루프를 도는 코루틴은 Unity를 멈추게 합니다. 루프 본문 안에 항상 `wait`를 포함하세요.

```prsm
// 나쁜 예 — Unity 멈춤
coroutine badLoop() {
    while true {
        checkSomething()
    }
}

// 좋은 예 — 매 프레임 yield
coroutine goodLoop() {
    while true {
        checkSomething()
        wait nextFrame
    }
}
```

## null 안전 패턴

### 존재하지 않을 수 있는 컴포넌트에 `optional`과 `?.` 사용

```prsm
component Interactable : MonoBehaviour {
    optional outline: OutlineEffect

    func highlight() {
        outline?.Enable()
    }

    func unhighlight() {
        outline?.Disable()
    }
}
```

### `?:`로 기본값 제공

엘비스 연산자는 값이 null일 수 있을 때 간결한 기본값을 제공합니다.

```prsm
func getDisplayName(): String {
    return player.customName ?: "Unknown Player"
}
```

### `!!` 사용 자제 — `require` 선호

not-null 단언 `!!`은 값이 null이면 런타임에 예외를 던집니다. 컴포넌트에서는 게임플레이 도중이 아닌 초기화 시점에 누락된 참조를 잡을 수 있도록 `require`를 선호하세요.

```prsm
// 위험 — 누락 시 런타임 실패
optional rb: Rigidbody
func move() {
    rb!!.MovePosition(target)   // 누락 시 NullReferenceException
}

// 안전 — Awake에서 명확한 메시지와 함께 즉시 실패
require rb: Rigidbody
func move() {
    rb.MovePosition(target)
}
```

## 데이터 모델링

### 값 타입에 `data class` 사용

데이터를 담는 구조체에는 `data class`를 사용합니다. 컴파일러가 동등성, 해싱, 문자열 표현을 생성합니다.

```prsm
data class DamageInfo {
    amount: Int
    source: GameObject
    damageType: DamageType
}

data class SpawnConfig {
    prefab: GameObject
    position: Vector3
    rotation: Quaternion = Quaternion.identity
}
```

### 유한 상태에 `enum` 사용

단순한 상태 머신에는 일반 enum이 적합합니다.

```prsm
enum EnemyState {
    Idle,
    Chase,
    Attack,
    Flee
}
```

### 데이터를 가진 상태에 매개변수화된 `enum` 사용

상태에 연관 데이터가 있을 때는 매개변수화된 배리언트를 사용합니다.

```prsm
enum AICommand {
    MoveTo(target: Vector3),
    Attack(enemy: GameObject),
    Wait(duration: Float),
    Patrol(waypoints: List<Vector3>)
}
```

## intrinsic 탈출구

`intrinsic`은 PrSM이 아직 지원하지 않는 Unity API에 대해 원시 C#로 직접 작성할 수 있게 합니다. 절제하여 사용하세요.

### 재사용성을 위해 `intrinsic func` 선호

탈출구를 이름 있는 함수로 감싸면 호출 지점이 깔끔하게 유지됩니다.

```prsm
intrinsic func setLayerRecursive(obj: GameObject, layer: Int) {
    obj.layer = layer;
    foreach (Transform child in obj.transform) {
        SetLayerRecursive(child.gameObject, layer);
    }
}
```

### 인라인 `intrinsic {}` 블록은 짧게 유지

인라인 블록을 사용해야 한다면 몇 줄로 제한하세요. 큰 intrinsic 블록은 PrSM을 작성하는 목적을 훼손합니다.

```prsm
func captureScreenshot() {
    val path = "Screenshots/" + System.DateTime.Now.ToString("yyyyMMdd_HHmmss") + ".png"
    intrinsic {
        ScreenCapture.CaptureScreenshot(path);
    }
}
```

### intrinsic을 사용해야 할 때

- PrSM이 래핑하지 않는 Unity API (예: 저수준 렌더링, 네이티브 플러그인)
- 정확한 C# 제어가 필요한 성능 중요 내부 루프
- 특정 C# 패턴이 필요한 서드파티 라이브러리 연동

### intrinsic을 사용하지 말아야 할 때

- 표준 Unity 라이프사이클 — `awake`, `start`, `update`, `onDestroy` 블록 사용
- 이벤트 연결 — `listen` 사용
- 코루틴 — `coroutine`과 `wait` 사용
- 입력 — `on input` 사용 (PrSM 2 부터)

큰 `intrinsic` 블록을 자주 작성하게 된다면, 해당 패턴을 네이티브로 지원할 수 있도록 기능 요청을 제출하는 것을 고려하세요.
