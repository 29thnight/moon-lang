---
title: Generated C# & Source Maps
parent: 도구
grand_parent: 한국어 문서
nav_order: 4
---

# Generated C# & Source Maps

PrSM은 `.prsm` 소스를 읽기 쉬운 C#으로 컴파일하고, 양방향 소스 매핑을 위한 `.prsmmap.json` 사이드카를 생성합니다.

## 컴파일 산출물

| 산출물 | 확장자 | 설명 |
|--------|--------|------|
| 소스 | `.prsm` | PrSM 소스 파일 |
| 생성 코드 | `.cs` | 사람이 읽을 수 있는 C# 출력 |
| 소스맵 | `.prsmmap.json` | 소스↔생성 위치 매핑 |

## Before/After 예시

### 필드와 라이프사이클이 있는 컴포넌트

```prsm
component Player : MonoBehaviour {
    serialize speed: Float = 5.0
    require rb: Rigidbody

    update {
        val h = input.axis("Horizontal")
        rb.velocity = vec3(h, 0, 0) * speed
    }
}
```

```csharp
public class Player : MonoBehaviour
{
    [SerializeField] private float _speed = 5.0f;
    private Rigidbody _rb;

    private void Awake()
    {
        _rb = GetComponent<Rigidbody>();
    }

    private void Update()
    {
        var h = Input.GetAxis("Horizontal");
        _rb.velocity = new Vector3(h, 0, 0) * _speed;
    }
}
```

### listen 수명 정책 (PrSM 2 부터)

```prsm
component UI : MonoBehaviour {
    serialize button: Button

    start {
        listen button.onClick until disable {
            fire()
        }
    }
}
```

```csharp
public class UI : MonoBehaviour
{
    [SerializeField] private Button _button;
    private System.Action _prsm_h0;

    private void Start()
    {
        _prsm_h0 = () => { fire(); };
        _button.onClick.AddListener(_prsm_h0);
    }

    private void __prsm_cleanup_disable()
    {
        _button.onClick.RemoveListener(_prsm_h0);
        _prsm_h0 = null;
    }

    private void OnDisable()
    {
        __prsm_cleanup_disable();
    }
}
```

### Sugar 매핑 요약

| PrSM | 생성 C# |
|------|---------|
| `vec2(x, y)` | `new Vector2(x, y)` |
| `vec3(x, y, z)` | `new Vector3(x, y, z)` |
| `color(r, g, b, a)` | `new Color(r, g, b, a)` |
| `input.axis("H")` | `Input.GetAxis("H")` |
| `get<T>()` | `GetComponent<T>()` |
| `find<T>()` | `FindFirstObjectByType<T>()` |
| `child<T>()` | `GetComponentInChildren<T>()` |
| `parent<T>()` | `GetComponentInParent<T>()` |
| `log(msg)` | `Debug.Log(msg)` |
| `wait 1.5s` | `yield return new WaitForSeconds(1.5f)` |
| `wait nextFrame` | `yield return null` |
| `wait until cond` | `yield return new WaitUntil(() => cond)` |
| `start coroutine()` | `StartCoroutine(coroutine())` |
| `obj?.method()` | `if (obj != null) obj.method()` |
| `a ?: b` | `a ?? b` |

## 소스맵 상세

`.prsmmap.json` 스키마와 디버깅 워크플로는 [소스맵](source-maps.md)을 참조하세요.
