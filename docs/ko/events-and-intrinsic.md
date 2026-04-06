---
title: Events & Intrinsic
parent: 언어 가이드
grand_parent: 한국어 문서
nav_order: 11
---

# Events & Intrinsic

## `listen` — 이벤트 연결

`listen` 키워드는 코드 블록을 Unity `UnityEvent` 또는 `UnityAction` 필드에 연결합니다. 생성된 `Awake()` 안에 `AddListener(...)` 호출로 lowering되며, 컴포넌트 룩업이 모두 완료된 이후에 추가됩니다.

### 기본 사용법

```prsm
listen startButton.onClick {
    SceneManager.LoadScene("Game")
}

listen slider.onValueChanged {
    updateVolume(slider.value)
}
```

### 파라미터 수신

`UnityEvent<T>` 콜백은 이벤트 페이로드를 자동으로 받습니다.

```prsm
listen healthBar.onValueChanged { val newValue ->
    if newValue <= 0.0 {
        triggerDeath()
    }
}
```

### listen 수명 정책 (PrSM 2 부터)

v2는 리스너 정리를 자동 관리하는 명시적 수명 정책을 도입합니다. `component` 선언 내부에서만 유효합니다 (외부 사용 시 에러 E083).

#### `until disable` — OnDisable에서 자동 정리

```prsm
listen button.onClick until disable {
    fire()
}
```

생성 C#:

```csharp
private System.Action _prsm_h0;

void Start() {
    _prsm_h0 = () => { fire(); };
    button.onClick.AddListener(_prsm_h0);
}

private void __prsm_cleanup_disable() {
    button.onClick.RemoveListener(_prsm_h0);
    _prsm_h0 = null;
}

void OnDisable() {
    __prsm_cleanup_disable();
}
```

#### `until destroy` — OnDestroy에서 자동 정리

```prsm
listen spawner.onSpawn until destroy {
    count += 1
}
```

`until disable`과 동일한 패턴이지만 `OnDestroy`에서 정리가 실행됩니다.

#### `manual` — 토큰을 이용한 명시적 제어

```prsm
val token = listen timer.onFinished manual {
    reset()
}

// 나중에:
unlisten token
```

`unlisten`은 토큰을 backing 필드로 해석하여 `RemoveListener` + null 할당을 생성합니다. 컴포넌트의 모든 메서드(라이프사이클 및 사용자 정의 함수)에서 사용 가능합니다.

#### 기본 동작

| 컨텍스트 | 기본값 |
|---------|--------|
| 수정자 없음 (`listen event { }`) | 등록만 — 자동 정리 없음 (v1, v2 동일) |
| `until disable` | OnDisable에서 자동 정리 |
| `until destroy` | OnDestroy에서 자동 정리 |
| `manual` | `unlisten`을 통한 명시적 제어 |

수명 수정자는 명시적으로 작성해야 합니다. v1과 v2 사이에 암묵적 기본값 변경은 없으며, 수정자가 없으면 항상 등록만 수행합니다.

#### 복수 리스너

여러 listen 문은 별도의 핸들러 필드(`_prsm_h0`, `_prsm_h1` 등)를 생성하며 독립적으로 정리됩니다.

---

## `intrinsic` — raw C# escape hatch

PrSM 문법이 커버하지 못하는 Unity API나 패턴이 있을 때, `intrinsic`은 컴포넌트 구조를 벗어나지 않고 raw C#을 삽입할 수 있는 탈출구입니다. `intrinsic` 안의 코드는 PrSM 시맨틱 검사의 대상이 아니며, C# 컴파일러에 의해서만 검증됩니다.

### 문장 블록 intrinsic

메서드 바디 안에 raw C# 문장을 인라인으로 삽입합니다.

```prsm
update {
    intrinsic {
        var ray = Camera.main.ScreenPointToRay(Input.mousePosition);
        if (Physics.Raycast(ray, out var hit, 100f)) {
            Debug.DrawLine(ray.origin, hit.point, Color.red);
        }
    }
}
```

### Intrinsic 함수

함수 전체 바디를 raw C#으로 선언합니다.

```prsm
intrinsic func getMouseWorldPos(): Vector3 {
    """
    var ray = Camera.main.ScreenPointToRay(Input.mousePosition);
    Physics.Raycast(ray, out RaycastHit hit, 100f);
    return hit.point;
    """
}
```

### Intrinsic 코루틴

raw C# `yield` 문장을 포함하는 코루틴을 선언합니다.

```prsm
intrinsic coroutine fadeOut(): IEnumerator {
    """
    float t = 1f;
    while (t > 0f) {
        t -= Time.deltaTime;
        canvasGroup.alpha = t;
        yield return null;
    }
    """
}
```

`intrinsic`은 드물게 사용하는 탈출구입니다. PrSM 문법을 우회하는 기본 수단으로 사용하지 마세요.
