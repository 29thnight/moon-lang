---
title: Generated C# & Source Maps
parent: Tooling
grand_parent: English Docs
nav_order: 4
---

# Generated C# & Source Maps

PrSM compiles `.prsm` source into readable C# and emits `.prsmmap.json` sidecars for bidirectional source mapping.

## Compilation artifacts

| Artifact | Extension | Description |
|----------|-----------|-------------|
| Source | `.prsm` | PrSM source file |
| Generated code | `.cs` | Human-readable C# output |
| Source map | `.prsmmap.json` | Position mapping between source and generated |

## Before/After examples

### Component with fields and lifecycle

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

### Listen with lifetime (PrSM 2 부터)

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

### Coroutine

```prsm
coroutine fadeOut() {
    var alpha = 1.0
    while alpha > 0.0 {
        alpha -= Time.deltaTime
        wait nextFrame
    }
}
```

```csharp
private System.Collections.IEnumerator fadeOut()
{
    var alpha = 1.0f;
    while (alpha > 0.0f)
    {
        alpha -= Time.deltaTime;
        yield return null;
    }
}
```

### Data class

```prsm
data class DamageInfo(amount: Int, crit: Bool)
```

```csharp
[System.Serializable]
public class DamageInfo
{
    public int amount;
    public bool crit;

    public DamageInfo(int amount, bool crit)
    {
        this.amount = amount;
        this.crit = crit;
    }

    public override bool Equals(object obj) { /* value equality */ }
    public override int GetHashCode() { /* hash based on fields */ }
    public override string ToString()
    {
        return $"DamageInfo(amount={amount}, crit={crit})";
    }
}
```

### Parameterized enum

```prsm
enum Weapon(val damage: Int, val range: Float) {
    Sword(10, 1.5),
    Bow(7, 8.0)
}
```

```csharp
public enum Weapon { Sword, Bow }

public static class WeaponExtensions
{
    public static int Damage(this Weapon value)
    {
        switch (value)
        {
            case Weapon.Sword: return 10;
            case Weapon.Bow: return 7;
            default: throw new System.InvalidOperationException();
        }
    }

    public static float Range(this Weapon value) { /* similar switch */ }
}
```

### Sugar mappings summary

| PrSM | Generated C# |
|------|-------------|
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
| `a ?: b` | `a ?? b` (or `a != null ? a : b`) |

## Source map details

See [Source Maps](source-maps.md) for the `.prsmmap.json` schema and debugging workflow.
