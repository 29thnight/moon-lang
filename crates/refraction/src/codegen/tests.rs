#[cfg(test)]
mod e2e {
    use crate::lexer::lexer::Lexer;
    use crate::parser::parser::Parser;
    use crate::lowering::lower::lower_file;
    use crate::codegen::emitter::emit;

    /// Full pipeline: source → tokens → AST → IR → C#
    fn compile(input: &str) -> String {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let file = parser.parse_file();
        assert!(parser.errors().is_empty(), "Parse errors: {:?}", parser.errors());
        let ir = lower_file(&file);
        emit(&ir)
    }

    #[test]
    fn test_empty_component() {
        let output = compile("component Foo : MonoBehaviour {}");
        assert!(output.contains("public class Foo : MonoBehaviour"));
        assert!(output.contains("{"));
        assert!(output.contains("}"));
    }

    #[test]
    fn test_component_with_interfaces() {
        let output = compile("component Foo : MonoBehaviour, IFoo, IBar {}");
        assert!(output.contains("public class Foo : MonoBehaviour, IFoo, IBar"));
    }

    #[test]
    fn test_serialize_field() {
        let output = compile("component Foo : MonoBehaviour {\n  serialize speed: Float = 5.0\n}");
        assert!(output.contains("[SerializeField]"));
        assert!(output.contains("private float _speed = 5.0f;"));
        assert!(output.contains("speed => _speed;"));
    }

    #[test]
    fn test_serialize_with_annotation() {
        let output = compile("component Foo : MonoBehaviour {\n  @header(\"Movement\")\n  serialize speed: Float = 5.0\n}");
        assert!(output.contains("[Header(\"Movement\")]"));
        assert!(output.contains("[SerializeField]"));
    }

    #[test]
    fn test_require_generates_awake() {
        let output = compile("component Foo : MonoBehaviour {\n  require rb: Rigidbody\n}");
        assert!(output.contains("private void Awake()"));
        assert!(output.contains("rb = GetComponent<Rigidbody>()"));
        assert!(output.contains("rb == null"));
        assert!(output.contains("Debug.LogError"));
        assert!(output.contains("enabled = false"));
    }

    #[test]
    fn test_optional_no_error_check() {
        let output = compile("component Foo : MonoBehaviour {\n  optional audio: AudioSource\n}");
        assert!(output.contains("audio = GetComponent<AudioSource>()"));
        // Should NOT contain error check for optional
        let awake_section = output.split("void Awake()").nth(1).unwrap_or("");
        assert!(!awake_section.contains("Debug.LogError"));
    }

    #[test]
    fn test_lifecycle_update() {
        let output = compile("component Foo : MonoBehaviour {\n  update {\n    move()\n  }\n}");
        assert!(output.contains("private void Update()"));
        assert!(output.contains("move();"));
    }

    #[test]
    fn test_lifecycle_on_trigger() {
        let output = compile("component Foo : MonoBehaviour {\n  onTriggerEnter(other: Collider) {\n    print(other)\n  }\n}");
        assert!(output.contains("private void OnTriggerEnter(Collider other)"));
        assert!(output.contains("Debug.Log(other)"));
    }

    #[test]
    fn test_func_block_body() {
        let output = compile("component Foo : MonoBehaviour {\n  func jump() {\n    print(\"jump\")\n  }\n}");
        assert!(output.contains("public void jump()"));
        assert!(output.contains("Debug.Log(\"jump\")"));
    }

    #[test]
    fn test_func_expr_body() {
        let output = compile("component Foo : MonoBehaviour {\n  func isDead(): Bool = hp <= 0\n}");
        assert!(output.contains("public bool isDead()"));
        assert!(output.contains("return hp <= 0;"));
    }

    #[test]
    fn test_private_func() {
        let output = compile("component Foo : MonoBehaviour {\n  private func helper() {\n  }\n}");
        assert!(output.contains("private void helper()"));
    }

    #[test]
    fn test_coroutine() {
        let output = compile("component Foo : MonoBehaviour {\n  coroutine blink() {\n    wait 0.2s\n  }\n}");
        assert!(output.contains("private System.Collections.IEnumerator blink()"));
        assert!(output.contains("yield return new WaitForSeconds(0.2f)"));
    }

    #[test]
    fn test_wait_forms() {
        let output = compile("component Foo : MonoBehaviour {\n  coroutine test() {\n    wait 1.0s\n    wait nextFrame\n    wait fixedFrame\n    wait until ready\n  }\n}");
        assert!(output.contains("yield return new WaitForSeconds(1.0f)"));
        assert!(output.contains("yield return null"));
        assert!(output.contains("yield return new WaitForFixedUpdate()"));
        assert!(output.contains("yield return new WaitUntil(() => ready)"));
    }

    #[test]
    fn test_start_coroutine() {
        let output = compile("component Foo : MonoBehaviour {\n  func go() {\n    start blink()\n  }\n  coroutine blink() {\n    wait 1.0s\n  }\n}");
        assert!(output.contains("StartCoroutine(blink())"));
    }

    #[test]
    fn test_listen_without_lambda_params() {
        let output = compile("component Foo : MonoBehaviour {\n  serialize button: Button\n  start {\n    listen button.onClick {\n      play()\n    }\n  }\n}");
        assert!(output.contains("button.onClick.AddListener(() =>"));
        assert!(output.contains("play();"));
    }

    #[test]
    fn test_listen_with_lambda_param() {
        let output = compile("component Foo : MonoBehaviour {\n  serialize slider: Slider\n  start {\n    listen slider.onValueChanged {\n      value => setVolume(value)\n    }\n  }\n}");
        assert!(output.contains("slider.onValueChanged.AddListener((value) =>"));
        assert!(output.contains("setVolume(value);"));
    }

    #[test]
    fn test_intrinsic_block_statement() {
        let output = compile("component Foo : MonoBehaviour {\n  func f() {\n    intrinsic {\n      Debug.Log(\"raw\");\n    }\n  }\n}");
        assert!(output.contains("Debug.Log(\"raw\");"));
    }

    #[test]
    fn test_intrinsic_function_member() {
        let output = compile("component Foo : MonoBehaviour {\n  intrinsic func nativeLog(message: String) {\n    Debug.Log(message);\n  }\n}");
        assert!(output.contains("public void nativeLog(string message)"));
        assert!(output.contains("Debug.Log(message);"));
    }

    #[test]
    fn test_intrinsic_coroutine_member() {
        let output = compile("component Foo : MonoBehaviour {\n  intrinsic coroutine waitNative() {\n    yield return null;\n  }\n}\n");
        assert!(output.contains("private System.Collections.IEnumerator waitNative()"));
        assert!(output.contains("yield return null;"));
    }

    #[test]
    fn test_if_else() {
        let output = compile("component Foo : MonoBehaviour {\n  func f() {\n    if hp <= 0 {\n      die()\n    } else {\n      run()\n    }\n  }\n}");
        assert!(output.contains("if (hp <= 0)"));
        assert!(output.contains("die();"));
        assert!(output.contains("else"));
        assert!(output.contains("run();"));
    }

    #[test]
    fn test_if_expression() {
        // Simple if/else with single-expression branches is lowered to a ternary — no __prsm_expr needed.
        let output = compile("component Foo : MonoBehaviour {\n  func score(): Int = if hp <= 0 { 0 } else { 100 }\n}");
        assert!(!output.contains("__prsm_expr"), "simple ternary should not emit __prsm_expr helper");
        assert!(output.contains("return (hp <= 0 ? 0 : 100);"));
    }

    #[test]
    fn test_if_expression_block_emits_helper() {
        // Multi-statement block expression requires __prsm_expr helper.
        let output = compile("component Foo : MonoBehaviour {\n  func score(): Int = if hp <= 0 {\n    val x = 1\n    x\n  } else { 100 }\n}");
        assert!(output.contains("private static T __prsm_expr<T>(System.Func<T> thunk)"));
        assert!(output.contains("__prsm_expr(() =>"));
    }

    #[test]
    fn test_when_expression() {
        let output = compile("component Foo : MonoBehaviour {\n  func score(): Int = when state {\n    EnemyState.Idle => 0\n    else => 100\n  }\n}");
        assert!(output.contains("return state switch"));
        assert!(output.contains("EnemyState.Idle => 0"));
        assert!(output.contains("_ => 100"));
    }

    #[test]
    fn test_for_range() {
        let output = compile("component Foo : MonoBehaviour {\n  func f() {\n    for i in 0 until 10 {\n      print(i)\n    }\n  }\n}");
        assert!(output.contains("for (int i = 0; i < 10; i++)"));
    }

    #[test]
    fn test_for_each() {
        let output = compile("component Foo : MonoBehaviour {\n  func f() {\n    for enemy in enemies {\n      attack(enemy)\n    }\n  }\n}");
        assert!(output.contains("foreach (var enemy in enemies)"));
    }

    #[test]
    fn test_while_loop() {
        let output = compile("component Foo : MonoBehaviour {\n  func f() {\n    while alive {\n      tick()\n    }\n  }\n}");
        assert!(output.contains("while (alive)"));
    }

    #[test]
    fn test_vec3_sugar() {
        let output = compile("component Foo : MonoBehaviour {\n  func f() {\n    val v = vec3(1, 2, 3)\n  }\n}");
        assert!(output.contains("new Vector3(1, 2, 3)"));
    }

    #[test]
    fn test_safe_call() {
        let output = compile("component Foo : MonoBehaviour {\n  optional anim: Animator\n  func f() {\n    anim?.play(\"Run\")\n  }\n}");
        assert!(output.contains("anim != null"));
        assert!(output.contains("anim.Play(\"Run\")"));
    }

    #[test]
    fn test_elvis_operator() {
        let output = compile("component Foo : MonoBehaviour {\n  func f() {\n    val name = playerName ?: \"Unknown\"\n  }\n}");
        assert!(output.contains("playerName ?? \"Unknown\""));
    }

    #[test]
    fn test_simple_string_interpolation() {
        let output = compile("component Foo : MonoBehaviour {\n  func f() {\n    print(\"hello $name\")\n  }\n}");
        assert!(output.contains("Debug.Log($\"hello {name}\")"));
    }

    #[test]
    fn test_asset_declaration() {
        let output = compile("asset WeaponData : ScriptableObject {\n  serialize damage: Int = 10\n}");
        assert!(output.contains("[CreateAssetMenu"));
        assert!(output.contains("public class WeaponData : ScriptableObject"));
        assert!(output.contains("[SerializeField]"));
    }

    #[test]
    fn test_class_with_interfaces() {
        let output = compile("class Helper : BaseHelper, IDisposable, IComparable {
}");
        assert!(output.contains("public class Helper : BaseHelper, IDisposable, IComparable"));
    }

    #[test]
    fn test_enum_declaration() {
        let output = compile("enum EnemyState {\n  Idle,\n  Chase,\n  Attack\n}");
        assert!(output.contains("public enum EnemyState"));
        assert!(output.contains("Idle,"));
        assert!(output.contains("Chase,"));
        assert!(output.contains("Attack,"));
    }

    #[test]
    fn test_parameterized_enum_declaration() {
        let output = compile("enum Weapon(val damage: Int, val range: Float) {\n  Sword(10, 1.5),\n  Bow(7, 8.0)\n}");
        assert!(output.contains("public enum Weapon"));
        assert!(output.contains("public static class WeaponExtensions"));
        assert!(output.contains("public static int Damage(this Weapon value)"));
        assert!(output.contains("public static float Range(this Weapon value)"));
        assert!(output.contains("case Weapon.Sword:"));
        assert!(output.contains("return 10;"));
        assert!(output.contains("return 1.5f;"));
        assert!(output.contains("case Weapon.Bow:"));
        assert!(output.contains("return 8.0f;"));
    }

    #[test]
    fn test_data_class() {
        let output = compile("data class DamageInfo(\n  val amount: Int,\n  val crit: Bool\n)");
        assert!(output.contains("[System.Serializable]"));
        assert!(output.contains("public class DamageInfo"));
        assert!(output.contains("public int amount;"));
        assert!(output.contains("public bool crit;"));
        assert!(output.contains("public DamageInfo(int amount, bool crit)"));
        assert!(output.contains("this.amount = amount;"));
        assert!(output.contains("public override bool Equals(object obj)"));
        assert!(output.contains("public override int GetHashCode()"));
        assert!(output.contains("public override string ToString()"));
        assert!(output.contains(r#"return $"DamageInfo(amount={amount}, crit={crit})";"#));
    }

    #[test]
    fn test_using_statements() {
        let output = compile("using UnityEngine\nusing UnityEngine.UI\ncomponent Foo : MonoBehaviour {}");
        assert!(output.contains("using UnityEngine;"));
        assert!(output.contains("using UnityEngine.UI;"));
    }

    #[test]
    fn test_full_player_controller() {
        let src = r#"using UnityEngine

component PlayerController : MonoBehaviour {
    @header("Movement")
    serialize speed: Float = 5.0
    serialize jumpForce: Float = 8.0

    require rb: Rigidbody
    optional animator: Animator

    update {
        val h = input.axis("Horizontal")
        val v = input.axis("Vertical")
        val move = vec3(h, 0, v)
        rb.velocity = move * speed
    }

    func jump() {
        rb.addForce(vec3(0, jumpForce, 0))
        animator?.play("Jump")
    }
}"#;
        let output = compile(src);
        // Verify key elements
        assert!(output.contains("public class PlayerController : MonoBehaviour"));
        assert!(output.contains("[Header(\"Movement\")]"));
        assert!(output.contains("[SerializeField]"));
        assert!(output.contains("private float _speed = 5.0f;"));
        assert!(output.contains("private void Awake()"));
        assert!(output.contains("_rb = GetComponent<Rigidbody>()"));
        assert!(output.contains("_animator = GetComponent<Animator>()"));
        assert!(output.contains("private void Update()"));
        assert!(output.contains("Input.GetAxis(\"Horizontal\")"));
        assert!(output.contains("new Vector3(h, 0, v)"));
        assert!(output.contains("public void jump()"));
        assert!(output.contains("rb.AddForce"));
        assert!(output.contains("new Vector3(0, jumpForce, 0)"));
        // Safe call: animator?.play("Jump") → if (animator != null) animator.Play("Jump")
        assert!(output.contains("animator != null"));
        assert!(output.contains("animator.Play(\"Jump\")"));
    }

    #[test]
    fn test_full_player_health() {
        let src = r#"using UnityEngine

component PlayerHealth : MonoBehaviour {
    serialize maxHp: Int = 100
    var hp: Int = 100
    var invincible: Bool = false

    func damage(amount: Int) {
        if invincible { return }
        hp -= amount
        start hitInvincible()
        if hp <= 0 {
            die()
        }
    }

    coroutine hitInvincible() {
        invincible = true
        wait 1.0s
        invincible = false
    }

    func die() {
        gameObject.setActive(false)
    }
}"#;
        let output = compile(src);
        assert!(output.contains("public class PlayerHealth : MonoBehaviour"));
        assert!(output.contains("[SerializeField]"));
        assert!(output.contains("private int _maxHp = 100;"));
        assert!(output.contains("private int _hp = 100;"));
        assert!(output.contains("if (invincible)"));
        assert!(output.contains("hp -= amount;"));
        assert!(output.contains("StartCoroutine(hitInvincible())"));
        assert!(output.contains("System.Collections.IEnumerator hitInvincible()"));
        assert!(output.contains("yield return new WaitForSeconds("));
        assert!(output.contains("invincible = true;"));
        assert!(output.contains("invincible = false;"));
    }

    // ── v2 listen lifetime tests ──────────────────────────────────

    #[test]
    fn test_listen_until_disable() {
        let src = "component Foo : MonoBehaviour {\n  serialize button: Button\n  start {\n    listen button.onClick until disable {\n      fire()\n    }\n  }\n}";
        let output = compile(src);
        assert!(output.contains("private System.Action _prsm_h0;"), "should generate handler field");
        assert!(output.contains("_prsm_h0 ="), "should assign lambda to field");
        assert!(output.contains("button.onClick.AddListener(_prsm_h0)"), "should add listener with field");
        assert!(output.contains("__prsm_cleanup_disable"), "should generate cleanup method");
        assert!(output.contains("button.onClick.RemoveListener(_prsm_h0)"), "cleanup should remove listener");
        assert!(output.contains("_prsm_h0 = null"), "cleanup should null the field");
        assert!(output.contains("OnDisable"), "should have OnDisable lifecycle");
    }

    #[test]
    fn test_listen_until_destroy() {
        let src = "component Foo : MonoBehaviour {\n  serialize button: Button\n  start {\n    listen button.onClick until destroy {\n      fire()\n    }\n  }\n}";
        let output = compile(src);
        assert!(output.contains("__prsm_cleanup_destroy"), "should generate destroy cleanup");
        assert!(output.contains("OnDestroy"), "should have OnDestroy lifecycle");
        assert!(output.contains("_prsm_h0 = null"), "cleanup should null the field");
    }

    #[test]
    fn test_listen_manual_and_unlisten() {
        let src = "component Foo : MonoBehaviour {\n  serialize button: Button\n  start {\n    val token = listen button.onClick manual {\n      fire()\n    }\n    unlisten token\n  }\n}";
        let output = compile(src);
        assert!(output.contains("_prsm_h0 ="), "should assign handler");
        assert!(output.contains("button.onClick.AddListener(_prsm_h0)"), "should add listener");
        assert!(output.contains("button.onClick.RemoveListener(_prsm_h0)"), "unlisten should remove listener");
        assert!(output.contains("_prsm_h0 = null"), "unlisten should null the field");
    }

    #[test]
    fn test_unlisten_in_user_func() {
        let src = "component Foo : MonoBehaviour {\n  serialize button: Button\n  start {\n    val token = listen button.onClick manual {\n      fire()\n    }\n  }\n  func cleanup() {\n    unlisten token\n  }\n}";
        let output = compile(src);
        assert!(output.contains("button.onClick.RemoveListener(_prsm_h0)"), "unlisten in func should generate real code");
        assert!(!output.contains("/* unlisten"), "should NOT emit placeholder comment");
    }

    // ── v2 pattern matching tests ─────────────────────────────────

    #[test]
    fn test_when_enum_payload_binding() {
        // compile() handles single declaration — test the enum extension output
        // which generates switch/case for payload access.
        let src = "enum EnemyState(val target: String) {\n  Idle(\"\"),\n  Chase(\"player\")\n}";
        let output = compile(src);
        assert!(output.contains("case EnemyState.Idle"), "should have case arm for Idle");
        assert!(output.contains("case EnemyState.Chase"), "should have case arm for Chase");
        assert!(output.contains("Target(this EnemyState"), "should generate payload accessor");
    }

    #[test]
    fn test_destructure_val() {
        // Single-declaration test: component with inline destructure
        let src = "component Foo : MonoBehaviour {\n  func f() {\n    val Stats(hp, speed) = getStats()\n  }\n}";
        let output = compile(src);
        assert!(output.contains("var _prsm_d = "), "should create temp variable");
        assert!(output.contains("_prsm_d.hp"), "should access hp field");
        assert!(output.contains("_prsm_d.speed"), "should access speed field");
    }

    // ── v2 generic inference test ─────────────────────────────────

    #[test]
    fn test_generic_inference_from_variable_type() {
        let src = "component Foo : MonoBehaviour {\n  func f() {\n    val rb: Rigidbody = get()\n  }\n}";
        let output = compile(src);
        assert!(output.contains("GetComponent<Rigidbody>()"), "should infer Rigidbody generic arg");
    }

    // ── v2 input system player form test ──────────────────────────

    #[test]
    fn test_input_player_action_vector2() {
        let src = r#"component Foo : MonoBehaviour {
  update {
    val look = input.player("Gameplay").action("Look").vector2
  }
}"#;
        let output = compile(src);
        assert!(output.contains("PlayerInput"), "should inject PlayerInput field");
        assert!(output.contains(r#"actions["Gameplay/Look"].ReadValue<UnityEngine.Vector2>()"#),
            "should generate player/action lookup with map prefix");
    }

    // ── T2: listen multiple subscriptions & ordering ──────────────

    #[test]
    fn test_listen_multiple_until_disable() {
        let src = r#"component Foo : MonoBehaviour {
  serialize buttonA: Button
  serialize buttonB: Button
  start {
    listen buttonA.onClick until disable {
      fireA()
    }
    listen buttonB.onClick until disable {
      fireB()
    }
  }
}"#;
        let output = compile(src);
        assert!(output.contains("_prsm_h0"), "should have first handler field");
        assert!(output.contains("_prsm_h1"), "should have second handler field");
        assert!(output.contains("buttonA.onClick.RemoveListener(_prsm_h0)"), "cleanup should remove first");
        assert!(output.contains("buttonB.onClick.RemoveListener(_prsm_h1)"), "cleanup should remove second");
    }

    // ── T3: for loop destructure ──────────────────────────────────

    #[test]
    fn test_for_loop_destructure() {
        let src = r#"component Foo : MonoBehaviour {
  func f() {
    for Spawn(pos, delay) in spawns {
      spawnAt(pos, delay)
    }
  }
}"#;
        let output = compile(src);
        assert!(output.contains("foreach"), "should lower to foreach");
        assert!(output.contains("spawns"), "should iterate over spawns");
    }

    // ── T5: Input System all states ───────────────────────────────

    #[test]
    fn test_input_released() {
        let src = r#"component Foo : MonoBehaviour {
  update {
    if input.action("Jump").released { land() }
  }
}"#;
        let output = compile(src);
        assert!(output.contains("WasReleasedThisFrame()"), "should generate WasReleasedThisFrame");
    }

    #[test]
    fn test_input_held() {
        let src = r#"component Foo : MonoBehaviour {
  update {
    if input.action("Sprint").held { sprint() }
  }
}"#;
        let output = compile(src);
        assert!(output.contains("IsPressed()"), "should generate IsPressed");
    }

    #[test]
    fn test_input_scalar() {
        let src = r#"component Foo : MonoBehaviour {
  update {
    val aim = input.action("Aim").scalar
  }
}"#;
        let output = compile(src);
        assert!(output.contains("ReadValue<float>()"), "should generate ReadValue<float>");
    }

    // ── v3 interface declaration ──────────────────────────────────

    #[test]
    fn test_interface_declaration() {
        let src = r#"interface IDamageable {
  func takeDamage(amount: Int)
  val isAlive: Bool
}"#;
        let output = compile(src);
        assert!(output.contains("public interface IDamageable"), "should generate interface");
        assert!(output.contains("void takeDamage(int amount);"), "should generate method signature");
        assert!(output.contains("bool isAlive { get; }"), "should generate readonly property");
    }

    #[test]
    fn test_interface_with_extends() {
        let src = r#"interface IHealable : IDamageable {
  func heal(amount: Int)
}"#;
        let output = compile(src);
        assert!(output.contains("public interface IHealable : IDamageable"), "should generate extends");
        assert!(output.contains("void heal(int amount);"), "should generate method");
    }

    #[test]
    fn test_generic_class() {
        let src = r#"class Registry<T> where T : Component {
  func add(item: T) { }
}"#;
        let output = compile(src);
        assert!(output.contains("public class Registry<T> where T : Component"), "should generate generic class with where clause");
        assert!(output.contains("void add(T item)"), "should generate method with generic param type");
    }

    #[test]
    fn test_generic_func() {
        let src = r#"class Utils {
  func findAll<T>(): List<T> where T : Component { }
}"#;
        let output = compile(src);
        assert!(output.contains("List<T> findAll<T>()"), "should generate generic method signature");
        assert!(output.contains("where T : Component"), "should generate where clause on method");
    }

    #[test]
    fn test_pool_member() {
        let src = r#"component Spawner : MonoBehaviour {
    serialize prefab: Bullet
    pool bullets: Bullet(capacity = 20, max = 100)
}"#;
        let output = compile(src);
        assert!(output.contains("ObjectPool<Bullet>"), "should generate ObjectPool<Bullet> field type");
        assert!(output.contains("_bullets"), "should generate _bullets backing field");
        assert!(output.contains("defaultCapacity: 20"), "should set defaultCapacity to 20");
        assert!(output.contains("maxSize: 100"), "should set maxSize to 100");
        assert!(output.contains("Instantiate(_prefab)"), "should use Instantiate with matching serialize prefab");
        assert!(output.contains("actionOnGet:"), "should have actionOnGet callback");
        assert!(output.contains("actionOnRelease:"), "should have actionOnRelease callback");
        assert!(output.contains("actionOnDestroy:"), "should have actionOnDestroy callback");
        assert!(output.contains("private void Awake()"), "should generate Awake for pool init");
    }

    #[test]
    fn test_singleton_component() {
        let src = r#"singleton component AudioManager : MonoBehaviour {
  serialize volume: Float = 1.0
}"#;
        let output = compile(src);
        // Singleton _instance field
        assert!(output.contains("private static AudioManager _instance;"), "should generate _instance field");
        // Singleton Instance property with lazy init
        assert!(output.contains("public static AudioManager Instance"), "should generate Instance property");
        assert!(output.contains("FindFirstObjectByType<AudioManager>()"), "should use FindFirstObjectByType in getter");
        assert!(output.contains("go.AddComponent<AudioManager>()"), "should fallback-create in getter");
        // Awake with singleton guard + DontDestroyOnLoad
        assert!(output.contains("_instance = this;"), "should assign _instance = this in Awake");
        assert!(output.contains("DontDestroyOnLoad(gameObject)"), "should call DontDestroyOnLoad in Awake");
        assert!(output.contains("Destroy(gameObject)"), "should destroy duplicate in Awake");
        // User serialize field should still be present
        assert!(output.contains("[SerializeField]"), "should still emit user serialize fields");
        assert!(output.contains("private float _volume = 1.0f;"), "should still emit user field");
    }
}
