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
    fn test_optional_nullable_lookup_strips_nullable_generic_argument() {
        let output = compile("component Foo : MonoBehaviour {\n  optional cam: Camera?\n}");
        assert!(output.contains("private Camera? _cam;"));
        assert!(output.contains("public Camera? cam => _cam;"));
        assert!(output.contains("_cam = GetComponent<Camera>()"));
        assert!(!output.contains("GetComponent<Camera?>()"));
    }

    #[test]
    fn test_lifecycle_update() {
        let output = compile("component Foo : MonoBehaviour {\n  update {\n    move()\n  }\n}");
        assert!(output.contains("private void Update()"));
        assert!(output.contains("Move();"));
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
        assert!(output.contains("public void Jump()"));
        assert!(output.contains("Debug.Log(\"jump\")"));
    }

    #[test]
    fn test_func_expr_body() {
        let output = compile("component Foo : MonoBehaviour {\n  func isDead(): Bool = hp <= 0\n}");
        assert!(output.contains("public bool IsDead()"));
        assert!(output.contains("return hp <= 0;"));
    }

    #[test]
    fn test_private_func() {
        let output = compile("component Foo : MonoBehaviour {\n  private func helper() {\n  }\n}");
        assert!(output.contains("private void Helper()"));
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
    fn test_wait_millisecond_duration_literal() {
        let output = compile("component Foo : MonoBehaviour {\n  coroutine test() {\n    wait 500ms\n  }\n}");
        assert!(output.contains("yield return new WaitForSeconds(0.5f)"));
    }

    #[test]
    fn test_start_coroutine() {
        let output = compile("component Foo : MonoBehaviour {\n  func go() {\n    start blink()\n  }\n  coroutine blink() {\n    wait 1.0s\n  }\n}");
        assert!(output.contains("StartCoroutine(Blink())"));
    }

    #[test]
    fn test_listen_without_lambda_params() {
        let output = compile("component Foo : MonoBehaviour {\n  serialize button: Button\n  start {\n    listen button.onClick {\n      play()\n    }\n  }\n}");
        assert!(output.contains("button.onClick.AddListener(() =>"));
        assert!(output.contains("Play();"));
    }

    #[test]
    fn test_listen_with_lambda_param() {
        let output = compile("component Foo : MonoBehaviour {\n  serialize slider: Slider\n  start {\n    listen slider.onValueChanged {\n      value => setVolume(value)\n    }\n  }\n}");
        assert!(output.contains("slider.onValueChanged.AddListener((value) =>"));
        assert!(output.contains("SetVolume(value);"));
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
        assert!(output.contains("Die();"));
        assert!(output.contains("else"));
        assert!(output.contains("Run();"));
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
        assert!(output.contains("public void Jump()"));
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
        assert!(output.contains("StartCoroutine(HitInvincible())"));
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

    // ── v2 input system map form test ───────────────────────────

    #[test]
    fn test_input_map_action_vector2() {
        let src = r#"component Foo : MonoBehaviour {
  update {
    val look = input.map("Gameplay").action("Look").vector2
  }
}"#;
        let output = compile(src);
        assert!(!output.contains("PlayerInput"), "should NOT inject PlayerInput for basic input sugar");
        assert!(output.contains(r#"InputSystem.actions?.FindAction("Gameplay/Look")?.ReadValue<UnityEngine.Vector2>() ?? default)"#),
            "should generate null-safe FindAction with map/action lookup key");
    }

    #[test]
    fn test_input_action_no_player_input() {
        let src = r#"component Foo : MonoBehaviour {
  update {
    if input.action("Jump").pressed { jump() }
  }
}"#;
        let output = compile(src);
        assert!(!output.contains("PlayerInput"), "should NOT inject PlayerInput for basic input.action()");
        assert!(!output.contains("_prsmInput"), "should NOT generate _prsmInput field");
        assert!(!output.contains("RequireComponent"), "should NOT add RequireComponent");
        assert!(output.contains(r#"InputSystem.actions?.FindAction("Jump")?.WasPressedThisFrame() ?? false)"#),
            "should generate null-safe InputSystem.actions.FindAction call");
    }

    // ── T2: listen multiple subscriptions & ordering ──────────────

    #[test]
    fn test_input_actions_annotation_wires_player_input_asset() {
        let src = r#"component Foo : MonoBehaviour {
  @inputActions(defaultMap: "Gameplay")
  serialize controls: InputActionAsset

  update {
    if input.action("Jump").pressed {
      jump()
    }
  }

  func jump(): Unit {}
}"#;
        let output = compile(src);
        assert!(
            output.contains("[UnityEngine.RequireComponent(typeof(UnityEngine.InputSystem.PlayerInput))]"),
            "should add RequireComponent for PlayerInput when @inputActions present: {}",
            output
        );
        assert!(
            output.contains("using UnityEngine.InputSystem;"),
            "should add UnityEngine.InputSystem using for InputActionAsset field: {}",
            output
        );
        assert!(
            output.contains("_prsmInput.actions = controls;"),
            "should wire annotated asset into PlayerInput.actions: {}",
            output
        );
        assert!(
            output.contains("_prsmInput.defaultActionMap = \"Gameplay\";"),
            "should wire defaultActionMap from annotation: {}",
            output
        );
        assert!(
            !output.contains("[InputActions"),
            "compiler-only annotation must not lower to a C# attribute: {}",
            output
        );
        assert!(
            output.contains("InputSystem.actions?.FindAction("),
            "input sugar should use null-safe InputSystem.actions.FindAction: {}",
            output
        );
    }

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
        assert!(output.contains(r#"InputSystem.actions?.FindAction("Jump")?.WasReleasedThisFrame() ?? false)"#),
            "should generate null-safe FindAction + WasReleasedThisFrame");
    }

    #[test]
    fn test_input_held() {
        let src = r#"component Foo : MonoBehaviour {
  update {
    if input.action("Sprint").held { sprint() }
  }
}"#;
        let output = compile(src);
        assert!(output.contains(r#"InputSystem.actions?.FindAction("Sprint")?.IsPressed() ?? false)"#),
            "should generate null-safe FindAction + IsPressed");
    }

    #[test]
    fn test_input_scalar() {
        let src = r#"component Foo : MonoBehaviour {
  update {
    val aim = input.action("Aim").scalar
  }
}"#;
        let output = compile(src);
        assert!(output.contains(r#"InputSystem.actions?.FindAction("Aim")?.ReadValue<float>() ?? 0f)"#),
            "should generate null-safe FindAction + ReadValue<float>");
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
        assert!(output.contains("void Add(T item)"), "should generate method with generic param type");
    }

    #[test]
    fn test_generic_func() {
        let src = r#"class Utils {
  func findAll<T>(): List<T> where T : Component { }
}"#;
        let output = compile(src);
        assert!(output.contains("List<T> FindAll<T>()"), "should generate generic method signature");
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

    // ── v3 optimizer: single-binding destructure inline ───────────

    #[test]
    fn test_single_binding_destructure_inlined() {
        let src = "component Foo : MonoBehaviour {\n  func f() {\n    val Stats(hp) = getStats()\n  }\n}";
        let output = compile(src);
        assert!(output.contains("GetStats().hp"), "single binding should inline without temp variable");
        assert!(!output.contains("_prsm_d"), "should NOT have temp variable for single binding");
    }

    // ── v4 Phase 1 tests ────────────────────────────────────────────

    #[test]
    fn test_try_catch_finally() {
        let src = r#"component Foo : MonoBehaviour {
  func f() {
    try {
      val x = 1
    } catch (e: Exception) {
      log(e)
    } finally {
      cleanup()
    }
  }
}"#;
        let output = compile(src);
        assert!(output.contains("try"), "should contain try");
        assert!(output.contains("catch (Exception e)"), "should contain catch with type");
        assert!(output.contains("finally"), "should contain finally");
    }

    #[test]
    fn test_throw_statement() {
        let src = r#"component Foo : MonoBehaviour {
  func f() {
    throw ArgumentException("bad")
  }
}"#;
        let output = compile(src);
        assert!(output.contains("throw new ArgumentException"), "throw should add 'new' keyword");
    }

    // Issue #1: rethrow inside a catch clause must NOT receive a `new`
    // prefix. The lowered output `throw new e;` would be invalid C#.
    #[test]
    fn test_throw_statement_rethrow() {
        let src = r#"using System
component Foo : MonoBehaviour {
  func go() {
    try {
      risky()
    } catch (e: Exception) {
      throw e
    }
  }
  func risky() { throw Exception("boom") }
}"#;
        let output = compile(src);
        assert!(
            output.contains("throw e;"),
            "rethrow should forward variable verbatim: {}",
            output
        );
        assert!(
            !output.contains("throw new e"),
            "rethrow must not be wrapped with `new` (invalid C#): {}",
            output
        );
        // The constructor `throw Exception("boom")` in the helper function
        // should still receive the `new` prefix in the same compile.
        assert!(
            output.contains("throw new Exception(\"boom\")"),
            "constructor throw should still receive `new` prefix: {}",
            output
        );
    }

    #[test]
    fn test_static_func() {
        let src = r#"class MathHelper {
  static func lerp(a: Float, b: Float, t: Float): Float = a + (b - a) * t
}"#;
        let output = compile(src);
        assert!(output.contains("static"), "should contain static modifier");
        assert!(output.contains("float"), "should contain float return type");
    }

    #[test]
    fn test_static_val_field() {
        let src = r#"class MathHelper {
  static val PI: Float = 3.14
}"#;
        let output = compile(src);
        assert!(output.contains("static"), "should contain static");
        assert!(output.contains("readonly"), "static val should be readonly");
    }

    #[test]
    fn test_const_field() {
        let src = r#"class Config {
  const MAX: Int = 100
}"#;
        let output = compile(src);
        assert!(output.contains("const"), "should contain const modifier");
        assert!(output.contains("int"), "should map Int to int");
        assert!(output.contains("100"), "should contain value");
    }

    #[test]
    fn test_raw_string_literal() {
        let src = r#"component Foo : MonoBehaviour {
  func f() {
    val json = """
hello world
"""
  }
}"#;
        let output = compile(src);
        assert!(output.contains("hello world"), "raw string content should be preserved");
    }

    #[test]
    fn test_in_operator_range() {
        let src = r#"component Foo : MonoBehaviour {
  func f() {
    val x = 5
    if x in 1..10 {
      log("in range")
    }
  }
}"#;
        let output = compile(src);
        assert!(output.contains(">=") && output.contains("<="), "in range should lower to >= and <=");
    }

    #[test]
    fn test_in_operator_collection() {
        let src = r#"component Foo : MonoBehaviour {
  func f() {
    val name = "Alice"
    if name in names {
      log("found")
    }
  }
}"#;
        let output = compile(src);
        assert!(output.contains(".Contains("), "in collection should lower to .Contains()");
    }

    #[test]
    fn test_or_pattern_in_when() {
        let src = r#"component Foo : MonoBehaviour {
  func f() {
    val x = 1
    when x {
      1, 2 => log("one or two")
      else => log("other")
    }
  }
}"#;
        let output = compile(src);
        assert!(output.contains("case 1:"), "should contain case 1:");
        assert!(output.contains("case 2:"), "should contain case 2:");
    }

    #[test]
    fn test_range_pattern_in_when() {
        let src = r#"component Foo : MonoBehaviour {
  func f() {
    val score = 85
    when score {
      in 90..100 => log("A")
      in 80..89 => log("B")
      else => log("F")
    }
  }
}"#;
        let output = compile(src);
        assert!(output.contains(">=") && output.contains("<="), "range pattern should use >= and <=");
    }

    #[test]
    fn test_null_coalesce_assign() {
        let src = r#"component Foo : MonoBehaviour {
  func f() {
    var x: Int? = null
    x ?:= 42
  }
}"#;
        let output = compile(src);
        assert!(output.contains("??="), "?:= should lower to ??=");
    }

    // ── Language 4 Phase 2: Type System Extensions ──────────────

    #[test]
    fn test_safe_cast() {
        let src = r#"component Foo : MonoBehaviour {
  func test(obj: Object) {
    val enemy = obj as Enemy?
  }
}"#;
        let output = compile(src);
        assert!(output.contains("obj as Enemy"), "safe cast should lower to C# 'as'");
    }

    #[test]
    fn test_force_cast() {
        let src = r#"component Foo : MonoBehaviour {
  func test(obj: Object) {
    val boss = obj as! Boss
  }
}"#;
        let output = compile(src);
        assert!(output.contains("((Boss)obj)"), "force cast should lower to C# explicit cast");
    }

    #[test]
    fn test_conversion_method_to_float() {
        let src = r#"component Foo : MonoBehaviour {
  func test() {
    val x = 42.toFloat()
  }
}"#;
        let output = compile(src);
        assert!(output.contains("((float)42)"), "toFloat() should lower to (float) cast");
    }

    #[test]
    fn test_conversion_method_to_string() {
        let src = r#"component Foo : MonoBehaviour {
  func test() {
    val x = 100.toString()
  }
}"#;
        let output = compile(src);
        assert!(output.contains("100.ToString()"), "toString() should lower to .ToString()");
    }

    #[test]
    fn test_abstract_class() {
        let src = r#"abstract class Weapon {
  abstract func attack()
  open func reload() { }
}"#;
        let output = compile(src);
        assert!(output.contains("public abstract class Weapon"), "abstract class modifier");
        assert!(output.contains("public abstract void Attack()"), "abstract func signature");
        assert!(output.contains("public virtual void Reload()"), "open func becomes virtual");
    }

    #[test]
    fn test_sealed_class() {
        let src = "sealed class Shape { }";
        let output = compile(src);
        assert!(output.contains("public sealed class Shape"), "sealed class modifier");
    }

    #[test]
    fn test_override_func() {
        let src = r#"class Sword : Weapon {
  override func attack() { }
}"#;
        let output = compile(src);
        assert!(output.contains("public override void Attack()"), "override func");
    }

    #[test]
    fn test_struct_basic() {
        let src = "struct DamageInfo(amount: Int, type: DamageType)";
        let output = compile(src);
        assert!(output.contains("public struct DamageInfo"), "struct declaration");
        assert!(output.contains("public int amount;"), "struct field");
        assert!(output.contains("public DamageType type;"), "struct field type");
        assert!(output.contains("this.amount = amount;"), "struct constructor body");
        assert!(output.contains("this.type = type;"), "struct constructor body");
    }

    #[test]
    fn test_struct_with_body() {
        let src = r#"struct Color32(r: Byte, g: Byte, b: Byte, a: Byte) {
  static val white: Color32 = Color32(255, 255, 255, 255)
}"#;
        let output = compile(src);
        assert!(output.contains("public struct Color32"), "struct declaration");
        assert!(output.contains("public byte r;"), "struct field");
        assert!(output.contains("public static readonly"), "static val in struct");
    }

    #[test]
    fn test_tuple_expression() {
        let src = r#"component Foo : MonoBehaviour {
  func test() {
    val pair = (42, "hello")
  }
}"#;
        let output = compile(src);
        assert!(output.contains("(42, \"hello\")"), "tuple expression");
    }

    #[test]
    fn test_tuple_type_in_func_return() {
        let src = r#"class Foo {
  func getResult(): (Int, String) = (42, "answer")
}"#;
        let output = compile(src);
        assert!(output.contains("(int, string)"), "tuple return type");
        assert!(output.contains("(42, \"answer\")"), "tuple expression in return");
    }

    // ── v4 Phase 4 — event, use, collection literals, DIM ─────────

    #[test]
    fn test_event_member_basic() {
        let src = r#"component Boss : MonoBehaviour {
  event onDamaged: (Int) => Unit
}"#;
        let output = compile(src);
        assert!(
            output.contains("public event System.Action<int> onDamaged;"),
            "event declaration: {}",
            output
        );
    }

    #[test]
    fn test_event_member_no_args() {
        let src = r#"component Boss : MonoBehaviour {
  event onDeath: () => Unit
}"#;
        let output = compile(src);
        assert!(
            output.contains("public event System.Action onDeath;"),
            "event with no args: {}",
            output
        );
    }

    #[test]
    fn test_event_member_invocation() {
        let src = r#"component Boss : MonoBehaviour {
  event onHealthChanged: (Int) => Unit
  func takeDamage(amount: Int) {
    onHealthChanged?.invoke(amount)
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("public event System.Action<int> onHealthChanged;"),
            "event field: {}",
            output
        );
        // Safe-call lowering for `?.invoke(...)` uses the Unity-safe null
        // check pattern: `if (event != null) event.Invoke(...)`.
        assert!(
            output.contains("onHealthChanged.Invoke(amount)"),
            "event invocation: {}",
            output
        );
        assert!(
            output.contains("onHealthChanged != null"),
            "event invocation null check: {}",
            output
        );
    }

    #[test]
    fn test_event_member_subscription() {
        let src = r#"component Boss : MonoBehaviour {
  private event onHit: (Int) => Unit
  func register(handler: (Int) => Unit) {
    onHit += handler
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("private event System.Action<int> onHit;"),
            "private event: {}",
            output
        );
        assert!(
            output.contains("onHit += handler"),
            "event += subscription: {}",
            output
        );
    }

    // ── use expression ───────────────────────────────────────────

    #[test]
    fn test_use_declaration_form() {
        let src = r#"component Foo : MonoBehaviour {
  func read() {
    use val stream = openFile()
    log("done")
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("using var stream = OpenFile()"),
            "use declaration form: {}",
            output
        );
    }

    #[test]
    fn test_use_block_form() {
        let src = r#"component Foo : MonoBehaviour {
  func read() {
    use stream = openFile() {
      log("inside use")
    }
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("using (var stream = OpenFile())"),
            "use block form opens using statement: {}",
            output
        );
        assert!(
            output.contains("Debug.Log(\"inside use\")"),
            "use block body: {}",
            output
        );
    }

    #[test]
    fn test_use_with_explicit_type() {
        let src = r#"component Foo : MonoBehaviour {
  func read() {
    use val s: FileStream = openStream()
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("using FileStream s = OpenStream()"),
            "use with explicit type: {}",
            output
        );
    }

    // ── Collection literals ──────────────────────────────────────

    #[test]
    fn test_list_literal_inferred() {
        let src = r#"component Foo : MonoBehaviour {
  func test() {
    val numbers = [1, 2, 3]
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("new System.Collections.Generic.List<int> { 1, 2, 3 }"),
            "list literal with inferred int element type: {}",
            output
        );
    }

    #[test]
    fn test_list_literal_with_type_annotation() {
        let src = r#"component Foo : MonoBehaviour {
  func test() {
    val xs: List<Int> = []
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("new System.Collections.Generic.List<int>()"),
            "empty list with explicit annotation: {}",
            output
        );
    }

    #[test]
    fn test_list_literal_strings() {
        let src = r#"component Foo : MonoBehaviour {
  func test() {
    val names = ["Alice", "Bob"]
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("new System.Collections.Generic.List<string> { \"Alice\", \"Bob\" }"),
            "list literal of strings: {}",
            output
        );
    }

    #[test]
    fn test_map_literal_inferred() {
        let src = r#"component Foo : MonoBehaviour {
  func test() {
    val lookup = {"hp": 100, "mp": 50}
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("new System.Collections.Generic.Dictionary<string, int>"),
            "map literal types: {}",
            output
        );
        assert!(
            output.contains("[\"hp\"] = 100"),
            "map literal first entry: {}",
            output
        );
        assert!(
            output.contains("[\"mp\"] = 50"),
            "map literal second entry: {}",
            output
        );
    }

    #[test]
    fn test_empty_list_without_type_errors() {
        // E107 is reported by the semantic analyzer; here we just verify the
        // compiler does not crash and emits some output for a literal that
        // does have a type annotation downstream.
        let src = r#"component Foo : MonoBehaviour {
  func test() {
    val xs: List<String> = []
  }
}"#;
        let output = compile(src);
        assert!(output.contains("new System.Collections.Generic.List<string>"));
    }

    // ── Default interface methods ────────────────────────────────

    #[test]
    fn test_default_interface_method() {
        let src = r#"interface IMovable {
  func move() {
    log("default move")
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("public interface IMovable"),
            "interface decl: {}",
            output
        );
        assert!(
            output.contains("void move()"),
            "method header: {}",
            output
        );
        assert!(
            output.contains("Debug.Log(\"default move\")"),
            "default body: {}",
            output
        );
    }

    #[test]
    fn test_interface_signature_only_no_default() {
        let src = r#"interface IDamageable {
  func takeDamage(amount: Int)
}"#;
        let output = compile(src);
        assert!(
            output.contains("void takeDamage(int amount);"),
            "abstract interface method: {}",
            output
        );
    }

    #[test]
    fn test_interface_mixed_default_and_signature() {
        let src = r#"interface IThing {
  func hello()
  func greet(): String {
    return "hello"
  }
}"#;
        let output = compile(src);
        assert!(output.contains("void hello();"), "abstract method: {}", output);
        assert!(
            output.contains("string greet()"),
            "default method header: {}",
            output
        );
        assert!(
            output.contains("return \"hello\""),
            "default body: {}",
            output
        );
    }

    // ── Phase 5: async / state machine / command / bind ──────────

    #[test]
    fn test_async_func_lowers_to_unitask() {
        let src = r#"component Loader : MonoBehaviour {
  async func loadProfile(): String {
    val payload = await fetch("/api/profile")
    return payload
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("async Cysharp.Threading.Tasks.UniTask<string> LoadProfile()"),
            "expected async UniTask<string>: {}",
            output
        );
        assert!(
            output.contains("await Fetch(\"/api/profile\")"),
            "expected await call: {}",
            output
        );
    }

    #[test]
    fn test_async_void_lowers_to_unitask() {
        let src = r#"component Loader : MonoBehaviour {
  async func ping() {
    await delay(1)
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("async Cysharp.Threading.Tasks.UniTask Ping()"),
            "expected async UniTask (void): {}",
            output
        );
    }

    #[test]
    fn test_state_machine_generates_enum_and_dispatcher() {
        let src = r#"component AI : MonoBehaviour {
  state machine ai {
    state Idle {
      enter { log("idle") }
      on go => Run
    }
    state Run {
      exit { log("leaving run") }
      on stopRun => Idle
    }
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("private enum AiState"),
            "expected enum AiState: {}",
            output
        );
        assert!(
            output.contains("Idle,") && output.contains("Run,"),
            "expected enum members: {}",
            output
        );
        assert!(
            output.contains("public void TransitionAi(string eventName)"),
            "expected transition method: {}",
            output
        );
        assert!(
            output.contains("(AiState.Idle, \"go\") => AiState.Run"),
            "expected switch arm: {}",
            output
        );
    }

    #[test]
    fn test_state_machine_initial_state() {
        let src = r#"component AI : MonoBehaviour {
  state machine ai {
    state Idle { on go => Run }
    state Run { on stopRun => Idle }
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("private AiState _ai = AiState.Idle"),
            "expected initial state assignment: {}",
            output
        );
    }

    #[test]
    fn test_command_lowers_to_class_and_helper() {
        let src = r#"component Unit : MonoBehaviour {
  command moveTo(target: Vector3) {
    log("moving")
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("public class MoveToCommand : ICommand"),
            "expected nested command class: {}",
            output
        );
        assert!(
            output.contains("public void Execute()"),
            "expected Execute method: {}",
            output
        );
        assert!(
            output.contains("new MoveToCommand(this, target).Execute()"),
            "expected helper invocation: {}",
            output
        );
    }

    #[test]
    fn test_command_with_undo_generates_undo_method() {
        let src = r#"component Unit : MonoBehaviour {
  command damage(amount: Int) {
    log("hurt")
  } undo {
    log("heal")
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("public void Undo()"),
            "expected Undo method: {}",
            output
        );
    }

    #[test]
    fn test_bind_property_lowers_to_inpc() {
        let src = r#"component HUD : MonoBehaviour {
  bind hp: Int = 100
}"#;
        let output = compile(src);
        assert!(
            output.contains("System.ComponentModel.INotifyPropertyChanged"),
            "component implements INPC: {}",
            output
        );
        assert!(
            output.contains("private int _hp = 100"),
            "backing field: {}",
            output
        );
        assert!(
            output.contains("public int hp"),
            "property header: {}",
            output
        );
        assert!(
            output.contains("OnPropertyChanged(nameof(hp))"),
            "INPC notification: {}",
            output
        );
        assert!(
            output.contains("public event System.ComponentModel.PropertyChangedEventHandler PropertyChanged"),
            "PropertyChanged event: {}",
            output
        );
    }

    #[test]
    fn test_bind_to_statement_assigns_initial_value() {
        let src = r#"component HUD : MonoBehaviour {
  bind hp: Int = 100
  serialize hpLabel: TextMeshProUGUI
  awake {
    bind hp to hpLabel.text
  }
}"#;
        let output = compile(src);
        // Issue #44: bind of a non-string source to a stringy target
        // (`*.text`) must convert via `.ToString()`. Before the fix
        // this emitted `hpLabel.text = this.hp` which produced CS0029.
        assert!(
            output.contains("hpLabel.text = this.hp.ToString()"),
            "expected initial sync with ToString conversion: {}",
            output
        );
        // The push-target lambda must also wrap with `.ToString()`.
        assert!(
            output.contains("hpLabel.text = __v.ToString()"),
            "expected push-target lambda with ToString: {}",
            output
        );
    }

    // Issue #44 (cont): same lowering must hold for Float / numeric
    // sources, not just Int. Verifies the type-aware ToString
    // insertion isn't tied to a specific source type.
    #[test]
    fn test_bind_float_to_text_inserts_to_string() {
        let src = r#"component HUD : MonoBehaviour {
  bind speed: Float = 1.5
  serialize speedLabel: TextMeshProUGUI
  awake {
    bind speed to speedLabel.text
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("speedLabel.text = this.speed.ToString()"),
            "expected float bind with ToString: {}",
            output
        );
        assert!(
            output.contains("speedLabel.text = __v.ToString()"),
            "expected float push lambda with ToString: {}",
            output
        );
    }

    // Language 5, Sprint 3: `val ref` declares a `ref readonly` local
    // initialized with `ref expr`.
    #[test]
    fn test_val_ref_lowers_to_ref_readonly_local() {
        let src = r#"component Probe : MonoBehaviour {
  func go() {
    val ref pos: Vector3 = ref transform.position
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("ref readonly Vector3 pos = ref transform.position"),
            "expected ref readonly local: {}",
            output
        );
    }

    // `var ref name = ref expr` lowers to a mutable `ref` local.
    #[test]
    fn test_var_ref_lowers_to_ref_local() {
        let src = r#"component Probe : MonoBehaviour {
  func go() {
    var ref pos: Vector3 = ref transform.position
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("ref Vector3 pos = ref transform.position"),
            "expected ref local: {}",
            output
        );
    }

    // ── Language 5 (deferred): stackalloc / ref struct / Span slice ──

    // `stackalloc[Int](256)` lowers to C# `stackalloc int[256]`.
    #[test]
    fn test_stackalloc_lowers_to_csharp_stackalloc() {
        let src = "component Probe : MonoBehaviour {\n  func go() {\n    val buf = stackalloc[Int](256)\n  }\n}";
        let output = compile(src);
        assert!(
            output.contains("stackalloc int[256]"),
            "expected stackalloc int[256] in output: {}",
            output
        );
    }

    // `ref struct Slice<T>(...)` lowers with the `ref` modifier.
    #[test]
    fn test_ref_struct_lowers_with_ref_modifier() {
        let src = "ref struct Slice(begin: Int, length: Int)";
        let output = compile(src);
        assert!(
            output.contains("public ref struct Slice"),
            "expected public ref struct Slice: {}",
            output
        );
    }

    // Issue #14 (follow-up to #4): a parameter name that collides with
    // a PrSM keyword (`start`) must also be referenceable from the
    // function body. The lang-5 spec example uses `start + length`
    // inside `func sum()`, where `start` previously failed to parse as
    // an expression because it lexes as the `Start` token.
    #[test]
    fn test_ref_struct_keyword_field_referenced_in_body() {
        let src = "ref struct Slice(start: Int, length: Int) {\n  func sum(): Int = start + length\n}";
        let output = compile(src);
        assert!(
            output.contains("public ref struct Slice"),
            "expected ref struct lowering: {}",
            output
        );
        assert!(
            output.contains("return start + length;"),
            "expected `start + length` body to compile and lower verbatim: {}",
            output
        );
    }

    // Issue #4: parameter names that collide with PrSM keywords
    // (`start`, `length`, `class`, etc.) must be accepted in declaration
    // position. The lang-5 spec example for `ref struct` uses `start` as
    // a field name; the parser previously rejected it as a `Start` token.
    #[test]
    fn test_ref_struct_keyword_param_name() {
        let src = "ref struct Slice(start: Int, length: Int) {\n  func describe(): String = \"slice\"\n}";
        let output = compile(src);
        assert!(
            output.contains("public ref struct Slice"),
            "expected public ref struct Slice: {}",
            output
        );
        assert!(
            output.contains("public int start;"),
            "expected `start` field declaration: {}",
            output
        );
        assert!(
            output.contains("public int length;"),
            "expected `length` field declaration: {}",
            output
        );
    }

    // Issue #10: a `when` expression used in `return` position must
    // emit each line of the lowered switch expression with the proper
    // indent (8 spaces inside a function body), not dedented to column
    // 0 on the second line onwards.
    #[test]
    fn test_when_expression_in_return_indent_propagation() {
        let src = "component Probe : MonoBehaviour {\n  func describe(value: Int): String {\n    return when value {\n      > 80 => \"high\"\n      > 30 => \"mid\"\n      else => \"low\"\n    }\n  }\n}";
        let output = compile(src);
        // After the `return value switch` line, the opening `{`, the
        // case arms, and the closing `};` should all be indented at the
        // function-body depth (8 spaces). The previous behavior dedented
        // every line after the first to column 0.
        assert!(
            output.contains("value switch\n        {"),
            "expected `{{` to be padded after `value switch`: {}",
            output
        );
        assert!(
            output.contains("\n        };"),
            "expected closing `}};` at function-body indent: {}",
            output
        );
        assert!(
            !output.contains("value switch\n{"),
            "lowered output dedents `{{` to column 0 after `value switch`: {}",
            output
        );
    }

    // Issue #32: a `data class` body block parses operator overloads
    // and the lowering emits each operator as a static C# method that
    // references the synthesized fields via `left.x` / `left.y`.
    #[test]
    fn test_data_class_body_operator_overload_lowers() {
        let src = "data class Vec2i(x: Int, y: Int) {\n  operator plus(other: Vec2i): Vec2i = Vec2i(x + other.x, y + other.y)\n}";
        let output = compile(src);
        assert!(
            output.contains("public class Vec2i"),
            "expected Vec2i data class to lower: {}",
            output
        );
        assert!(
            output.contains("public static Vec2i operator +(Vec2i left, Vec2i other)"),
            "expected operator + method: {}",
            output
        );
        assert!(
            output.contains("new Vec2i(left.x + other.x, left.y + other.y)"),
            "expected `left.x + other.x` field rewrite (not `x + other.left.x`): {}",
            output
        );
    }

    // Issue #24: command sugar nested ICommand class rewrites bare
    // owner-member references to `_owner.name` (fields, properties,
    // methods, lookup fields, and Unity-component built-ins like
    // `transform` / `gameObject`). Without this fix the generated
    // CanExecute / Execute / Undo bodies referenced undefined
    // identifiers.
    #[test]
    fn test_command_sugar_owner_member_rewrite() {
        let src = "component UnitController : MonoBehaviour {\n  var prevPos: Vector3 = Vector3.zero\n  var isAlive: Bool = true\n  var isStunned: Bool = false\n  command moveUnit(target: Vector3) {\n    prevPos = transform.position\n    transform.position = target\n  } undo {\n    transform.position = prevPos\n  } canExecute = isAlive && !isStunned\n}";
        let output = compile(src);
        assert!(
            output.contains("CanExecute() => _owner.isAlive && !(_owner.isStunned)"),
            "expected `_owner.isAlive` / `_owner.isStunned` rewrite in CanExecute: {}",
            output
        );
        assert!(
            output.contains("_owner.prevPos = _owner.transform.position"),
            "expected `_owner.prevPos = _owner.transform.position` in Execute: {}",
            output
        );
        assert!(
            output.contains("_owner.transform.position = _owner.prevPos"),
            "expected `_owner.transform.position = _owner.prevPos` in Undo: {}",
            output
        );
    }

    // Issue #26: a `typealias Name = Target` declaration emits a C#
    // file-scoped using alias directive (`using Name = Target;`).
    #[test]
    fn test_typealias_emits_using_alias_directive() {
        let src = "typealias Position = Vector3";
        let output = compile(src);
        assert!(
            output.contains("using Position = Vector3;"),
            "expected `using Position = Vector3;` directive: {}",
            output
        );
    }

    // Issue #29: a nested data class lowers with the correct indent
    // (one level inside the parent class) and the parent's collection
    // field uses the nested type rather than `object` (covered by
    // #25). The previous lowering left the original 4-space indent on
    // every line of the nested class, producing a doubled indent in
    // the final output.
    #[test]
    fn test_nested_data_class_indent() {
        let src = "component Inventory : MonoBehaviour {\n  data class Slot(itemId: Int, count: Int)\n  var slots: List<Slot> = []\n}";
        let output = compile(src);
        assert!(
            output.contains("List<Slot> _slots = new System.Collections.Generic.List<Slot>()"),
            "expected `List<Slot>` substitution: {}",
            output
        );
        // The nested class header should be indented one level inside
        // the parent class body (4 spaces in front of `public class`).
        assert!(
            output.contains("\n    public class Slot"),
            "expected nested class indented at parent body depth: {}",
            output
        );
        assert!(
            !output.contains("\n        public class Slot"),
            "lowered output must not double-indent the nested class header: {}",
            output
        );
    }

    // Issue #28: positional pattern with binding variables and a guard
    // (`Point(x, y) if x == y`) lowers to a C# switch expression with
    // the captures and the `when` clause preserved. The previous
    // lowering dropped both — emitting `Point _ => "diagonal"`.
    #[test]
    fn test_positional_pattern_with_binding_and_guard() {
        let src = "component Probe : MonoBehaviour {\n  func describe(p: Point): String {\n    return when p {\n      Point(0, 0) => \"origin\"\n      Point(x, y) if x == y => \"diagonal\"\n      else => \"elsewhere\"\n    }\n  }\n}";
        let output = compile(src);
        assert!(
            output.contains("Point(var x, var y) when x == y => \"diagonal\""),
            "expected `Point(var x, var y) when x == y` arm: {}",
            output
        );
        assert!(
            !output.contains("Point _ when"),
            "lowered output must not drop the binding into `Point _`: {}",
            output
        );
    }

    // Issue #25: a generic class field initializer `var items: List<T> = []`
    // lowers to `new List<T>()` (not `new List<object>()`). The previous
    // lowering dropped the type annotation and fell back to `object`
    // for the element type, silently producing wrong runtime behavior.
    #[test]
    fn test_generic_field_init_preserves_type_parameter() {
        let src = "class Registry<T> where T : MonoBehaviour {\n  var items: List<T> = []\n}";
        let output = compile(src);
        assert!(
            output.contains("new System.Collections.Generic.List<T>()"),
            "expected `new List<T>()` (not `<object>`): {}",
            output
        );
        assert!(
            !output.contains("new System.Collections.Generic.List<object>()"),
            "lowered output must not fall back to List<object>: {}",
            output
        );
    }

    // Issue #22: `bind X to widget.text` (the canonical lang-4 MVVM
    // pattern) compiles without a false-positive E144 type mismatch.
    // The previous semantic check rejected the case because it treated
    // the `text` member name as a type literal.
    #[test]
    fn test_bind_to_member_access_no_false_positive_e144() {
        let src = "component PlayerHUD : MonoBehaviour {\n  bind playerName: String = \"Hero\"\n  serialize nameLabel: TextMeshProUGUI\n  awake {\n    bind playerName to nameLabel.text\n  }\n}";
        let output = compile(src);
        assert!(
            output.contains("public class PlayerHUD"),
            "expected PlayerHUD component to lower: {}",
            output
        );
        assert!(
            output.contains("nameLabel.text = this.playerName"),
            "string bind should remain a direct assignment: {}",
            output
        );
        assert!(
            !output.contains("nameLabel.text = this.playerName.ToString()"),
            "string bind must not add a redundant ToString conversion: {}",
            output
        );
    }

    // Issue #21: a map literal assigned to a `Map<String, Int>`-typed
    // variable passes the type check, even though the analyzer reports
    // the literal as `External("map")` instead of a full generic type.
    #[test]
    fn test_map_literal_assignment_to_typed_variable() {
        let src = "component Probe : MonoBehaviour {\n  func go() {\n    val lookup: Map<String, Int> = {\"hp\": 100, \"mp\": 50}\n  }\n}";
        let output = compile(src);
        assert!(
            output.contains("Dictionary<string, int> lookup"),
            "expected Dictionary<string, int> lookup assignment: {}",
            output
        );
    }

    // Issue #20: a lambda literal assigned to a variable annotated
    // with a function type passes the type check (the analyzer trusts
    // the explicit annotation rather than producing a function type
    // from the lambda body).
    #[test]
    fn test_lambda_assignment_to_typed_variable() {
        let src = "component Probe : MonoBehaviour {\n  func go() {\n    val callback: (Int) => Unit = { x => log(\"$x\") }\n    callback(42)\n    val add: (Int, Int) => Int = { a, b => a + b }\n    log(\"${add(3, 4)}\")\n  }\n}";
        let output = compile(src);
        assert!(
            output.contains("Action<int> callback"),
            "expected Action<int> callback assignment: {}",
            output
        );
        assert!(
            output.contains("Func<int, int, int> add"),
            "expected Func<int, int, int> add assignment: {}",
            output
        );
    }

    // Issue #31: a safe cast `as Type?` preserves the nullable suffix
    // in the lowered C# output, opting into nullable reference types.
    #[test]
    fn test_safe_cast_preserves_nullable_suffix() {
        let src = "component Probe : MonoBehaviour {\n  func handle(c: Collider) {\n    val box = c as BoxCollider?\n  }\n}";
        let output = compile(src);
        assert!(
            output.contains("c as BoxCollider?"),
            "expected nullable suffix preserved in `as` cast: {}",
            output
        );
    }

    #[test]
    fn test_if_is_smart_cast_lowers_with_pattern_alias() {
        let src = "component Probe : MonoBehaviour {\n  func handle(c: Collider) {\n    if c is BoxCollider {\n      log(\"size: ${c.size}\")\n    }\n  }\n}";
        let output = compile(src);
        assert!(
            output.contains("if (c is BoxCollider _bc_c)"),
            "expected smart-cast pattern variable in if-condition: {}",
            output
        );
        assert!(
            output.contains("Debug.Log($\"size: {_bc_c.size}\")"),
            "expected smart-cast alias usage inside if-body: {}",
            output
        );
        assert!(
            !output.contains("Debug.Log($\"size: {c.size}\")"),
            "lowered output must not keep the wide receiver inside the narrowed block: {}",
            output
        );
    }

    // Issue #30: PrSM `e.message` member access on a System.Exception
    // lowers to PascalCase `e.Message`. Other Exception members
    // (`stackTrace`, `innerException`, `helpLink`, `targetSite`) get
    // the same treatment.
    #[test]
    fn test_exception_message_lowers_to_pascalcase() {
        let src = "using System\ncomponent Probe : MonoBehaviour {\n  func go() {\n    try { risky() } catch (e: Exception) { log(e.message) }\n  }\n  func risky() { throw Exception(\"boom\") }\n}";
        let output = compile(src);
        assert!(
            output.contains("e.Message"),
            "expected `e.Message` (PascalCase): {}",
            output
        );
        assert!(
            !output.contains("e.message"),
            "lowered output must not contain camelCase `e.message`: {}",
            output
        );
    }

    #[test]
    fn test_non_exception_message_member_stays_camelcase() {
        let src = "component Probe : MonoBehaviour {\n  data class LogState(message: String)\n  val logger: LogState = LogState(\"hi\")\n  func go() {\n    log(logger.message)\n  }\n}";
        let output = compile(src);
        assert!(
            output.contains("logger.message"),
            "expected non-exception member to remain camelCase: {}",
            output
        );
        assert!(
            !output.contains("logger.Message"),
            "non-exception member sugar must not rewrite to PascalCase: {}",
            output
        );
    }

    // Issue #27: a multi-line PrSM string (typically from a raw string
    // literal `"""..."""`) lowers to a C# verbatim string `@"..."`.
    // The verbatim form preserves newlines without `\n` sequences and
    // escapes embedded `"` as `""`. PrSM raw strings preserve special
    // characters without processing escapes (per the lang-4 spec).
    #[test]
    fn test_raw_string_lowers_to_verbatim_string() {
        // Use a multi-line literal that contains a real newline; the
        // lexer scans the body verbatim and the lowering wraps it in
        // the C# verbatim form.
        let src = "component Probe : MonoBehaviour {\n  func go() {\n    val text = \"\"\"\nfirst line\nsecond line\n\"\"\"\n    log(text)\n  }\n}";
        let output = compile(src);
        assert!(
            output.contains("@\""),
            "expected verbatim string `@\"...\"` for raw string lowering: {}",
            output
        );
        assert!(
            output.contains("first line"),
            "expected raw string body in lowered output: {}",
            output
        );
        assert!(
            output.contains("second line"),
            "expected raw string body second line in lowered output: {}",
            output
        );
    }

    // Issue #23: `operator get` / `operator set` indexer declarations
    // must NOT trigger the reserved-name E101 check that applies to
    // free-standing `func get()`. The two have different lowerings: a
    // free `get` collides with the `GetComponent<T>()` sugar, but the
    // operator form lowers to a C# `this[...]` indexer.
    #[test]
    fn test_operator_get_set_indexer_not_rejected() {
        let src = "class Inventory {\n  var items: List<Int> = []\n  operator get(index: Int): Int = items[index]\n  operator set(index: Int, value: Int) { items[index] = value }\n}";
        let output = compile(src);
        assert!(
            output.contains("public class Inventory"),
            "expected class Inventory to lower: {}",
            output
        );
        assert!(
            output.contains("this[int index]"),
            "expected `this[int index]` indexer member: {}",
            output
        );
    }

    // Issue #17: tuple destructure `val (a, b) = expr` lowers to a
    // C# tuple deconstruction `var (a, b) = expr;`. The lang-4 spec
    // example for tuples and the v5 discard destructure both depend
    // on this form.
    #[test]
    fn test_tuple_destructure_lowers_to_var_paren() {
        let src = "component Probe : MonoBehaviour {\n  func getResult(): (Int, String) = (42, \"answer\")\n  func go() {\n    val (num, name) = getResult()\n    log(\"$num $name\")\n  }\n}";
        let output = compile(src);
        assert!(
            output.contains("var (num, name) = GetResult()"),
            "expected `var (num, name) = GetResult()`: {}",
            output
        );
    }

    // Issue #17: discard `_` is accepted as a tuple destructure binding.
    #[test]
    fn test_tuple_destructure_with_discard() {
        let src = "component Probe : MonoBehaviour {\n  func getResult(): (Int, String) = (42, \"answer\")\n  func go() {\n    val (_, name) = getResult()\n    log(name)\n  }\n}";
        let output = compile(src);
        assert!(
            output.contains("var (_, name) = GetResult()"),
            "expected `var (_, name) = GetResult()`: {}",
            output
        );
    }

    // Issue #19: an attribute target name that is a PrSM keyword
    // (`@return`, `@type`) must parse. The previous parser used
    // `expect_ident` and rejected the `return` keyword token, which
    // caused the next member declaration to be misparsed as a new
    // top-level decl.
    #[test]
    fn test_attribute_target_with_keyword_name() {
        let src = "component Probe : MonoBehaviour {\n  @field(nonSerialized)\n  var cache: Int = 0\n\n  @return(notNull)\n  func getName(): String = \"Player\"\n}";
        let output = compile(src);
        assert!(
            output.contains("public class Probe : MonoBehaviour"),
            "expected component Probe to lower: {}",
            output
        );
        assert!(
            output.contains("public string GetName()"),
            "expected `GetName` method after `@return` annotation: {}",
            output
        );
    }

    // Issue #16: a `var name: Type = init` followed by `get`/`set`
    // accessors must continue to be parsed as a single property member.
    // The previous parser closed the field after the initializer and
    // misparsed the trailing accessor lines as a new top-level decl.
    #[test]
    fn test_property_with_init_and_accessors_parses() {
        let src = "component Player : MonoBehaviour {\n  var maxHp: Int = 100\n  var hp: Int = 100\n    get = _hp\n    set(value) {\n      _hp = Mathf.clamp(value, 0, maxHp)\n    }\n  val isAlive: Bool\n    get = hp > 0\n}";
        let output = compile(src);
        assert!(
            output.contains("public class Player : MonoBehaviour"),
            "expected component Player to lower: {}",
            output
        );
        assert!(
            output.contains("public int maxHp"),
            "expected `maxHp` field/property: {}",
            output
        );
        assert!(
            output.contains("public int hp"),
            "expected `hp` property: {}",
            output
        );
        assert!(
            output.contains("public bool isAlive"),
            "expected `isAlive` computed property: {}",
            output
        );
    }

    // Issue #11: PrSM `length` member access on a collection lowers
    // to PascalCase `Length` so the result is valid against arrays,
    // NativeArray<T>, Span<T>, etc.
    #[test]
    fn test_length_member_lowers_to_pascalcase() {
        let src = "component Bench : MonoBehaviour {\n  func go(arr: Array<Int>) {\n    for i in 0 until arr.length {\n      log(\"$i\")\n    }\n  }\n}";
        let output = compile(src);
        assert!(
            output.contains("arr.Length"),
            "expected `arr.Length` (PascalCase): {}",
            output
        );
        assert!(
            !output.contains("arr.length"),
            "lowered output must not contain camelCase `length`: {}",
            output
        );
    }

    #[test]
    fn test_safe_call_property_preserves_non_sugar_camelcase_member() {
        let src = "component Probe : MonoBehaviour {\n  optional cam: Camera?\n  func go() {\n    val depth = cam?.depth ?: 0.0\n    log(\"$depth\")\n  }\n}";
        let output = compile(src);
        assert!(
            output.contains("cam.depth"),
            "expected safe-call property to preserve Camera.depth: {}",
            output
        );
        assert!(
            !output.contains("cam.Depth"),
            "safe-call property must not rewrite Camera.depth to PascalCase: {}",
            output
        );
    }

    // Issue #7: `yield i` inside a `for` loop must succeed when the
    // for-loop induction variable shares the coroutine's element type.
    // The previous semantic analyzer treated every for-loop variable as
    // `var`, producing a false-positive E148 against `Seq<Int>`.
    #[test]
    fn test_yield_for_loop_induction_variable() {
        let src = "component Cutscene : MonoBehaviour {\n  coroutine countdown(): Seq<Int> {\n    for i in 0 until 5 {\n      yield i\n    }\n    yield 0\n    yield break\n  }\n}";
        let output = compile(src);
        assert!(
            output.contains("yield return i"),
            "expected `yield return i` in lowered coroutine: {}",
            output
        );
        assert!(
            output.contains("IEnumerator<int> countdown"),
            "expected typed iterator return: {}",
            output
        );
    }

    // Issue #6: a named argument whose name collides with a PrSM keyword
    // (`parent`, `child`, `length`, etc.) must be accepted at the call
    // site. Discovered after the lang-5 spec example for default
    // parameters tried `instantiate(prefab, parent: someParent)`.
    #[test]
    fn test_named_argument_with_keyword_name() {
        let src = "component Probe : MonoBehaviour {\n  func instantiate(prefab: GameObject, parent: Transform? = null): GameObject {\n    return GameObject.Instantiate(prefab, parent)\n  }\n  func go(some: GameObject, target: Transform) {\n    instantiate(some, parent: target)\n  }\n}";
        let output = compile(src);
        assert!(
            output.contains("Instantiate(some, parent: target)"),
            "expected named argument call with keyword name: {}",
            output
        );
    }

    // Issue #2: a `serialize var hp: Int { get; set { ... } }`
    // declaration must lower to a backing field carrying
    // `[SerializeField]` (with brackets), and the empty getter must
    // emit `return _hp;` rather than the broken `return ;`.
    #[test]
    fn test_serialize_property_with_custom_setter_emits_brackets_and_backing_return() {
        let src = "component Player : MonoBehaviour {\n  serialize var hp: Int = 100\n    get\n    set { field = Mathf.clamp(value, 0, 200) }\n}";
        let output = compile(src);
        assert!(
            output.contains("[SerializeField]"),
            "expected `[SerializeField]` (with brackets) on backing field: {}",
            output
        );
        assert!(
            !output.contains("\n    SerializeField\n"),
            "lowered output contains bare `SerializeField` token without brackets: {}",
            output
        );
        assert!(
            output.contains("return _hp;"),
            "expected getter to return backing field `_hp`: {}",
            output
        );
        assert!(
            !output.contains("return ;"),
            "lowered output contains broken `return ;` empty getter: {}",
            output
        );
    }

    // Issue #9: a receiver-less PascalCase Call is treated as a
    // constructor invocation and the lowering prepends `new`. Without
    // this fix, every `data class` / `struct` instantiation produced
    // invalid C# (`var info = DamageInfo(10, true);` — no method
    // `DamageInfo`).
    #[test]
    fn test_pascalcase_call_lowers_with_new() {
        let src = "data class DamageInfo(amount: Int, crit: Bool)";
        let _ = compile(src); // Generates the data class itself.

        // Use the data class from a sibling component (separate compile
        // unit since #8 forbids two top-level decls in one file).
        let src = "component Probe : MonoBehaviour {\n  func go() {\n    val info = DamageInfo(10, true)\n  }\n}";
        let output = compile(src);
        assert!(
            output.contains("var info = new DamageInfo(10, true)"),
            "expected `new DamageInfo(10, true)`: {}",
            output
        );
    }

    // Issue #3: `val ref` with an explicit type lowers to a valid
    // `ref readonly T name = ref expr;` statement. The annotation is
    // trusted by the semantic analyzer (no E020 type-mismatch from the
    // ref-of inner expression).
    #[test]
    fn test_val_ref_with_explicit_type_lowers_correctly() {
        let src = "component Probe : MonoBehaviour {\n  func go() {\n    val ref pos: Vector3 = ref transform.position\n  }\n}";
        let output = compile(src);
        assert!(
            output.contains("ref readonly Vector3 pos = ref transform.position"),
            "expected `ref readonly Vector3 pos = ref transform.position`: {}",
            output
        );
        assert!(
            !output.contains("ref readonly var"),
            "lowering must not emit invalid `ref readonly var` form: {}",
            output
        );
    }

    // Issue #3: `val ref` without an explicit type produces E190.
    #[test]
    fn test_val_ref_without_type_emits_e190() {
        let src = "component Probe : MonoBehaviour {\n  func go() {\n    val ref pos = ref transform.position\n  }\n}";
        let mut lexer = Lexer::new(src);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let file = parser.parse_file();
        assert!(parser.errors().is_empty(), "Parse errors: {:?}", parser.errors());
        // Run semantic analysis to surface E190.
        let mut analyzer = crate::semantic::analyzer::Analyzer::new();
        analyzer.analyze_file(&file);
        let errors = analyzer.diag.errors();
        assert!(
            errors.iter().any(|d| d.code == "E190"),
            "expected E190 diagnostic for ref local without explicit type, got: {:?}",
            errors
        );
    }

    // Issue #8: a `.prsm` file containing more than one top-level
    // declaration must produce a hard error (E189). Earlier versions
    // silently dropped the second declaration.
    #[test]
    fn test_multiple_top_level_decls_emits_e189() {
        // The high-level `compile` helper asserts there are zero parser
        // errors, so we drive the lexer + parser directly here to inspect
        // the diagnostic list without panicking.
        let src = "data class PlayerStats(hp: Int)\n\ncomponent Probe : MonoBehaviour {\n  func go() {}\n}";
        let mut lexer = Lexer::new(src);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let _file = parser.parse_file();
        let errors = parser.errors();
        assert!(
            errors.iter().any(|e| e.message.contains("E189")),
            "expected E189 multi-decl diagnostic, got: {:?}",
            errors
        );
    }

    // `arr[1..5]` lowers to a C# range slice. PrSM `..` is inclusive
    // (matching Kotlin), so the lowered upper bound is `(5 + 1)`.
    // `arr until 5` lowers to the half-open form `arr[1..5]`.
    #[test]
    fn test_range_index_access_lowers_to_csharp_range() {
        let src = "component Probe : MonoBehaviour {\n  func go(arr: Array<Int>) {\n    val slice = arr[1..5]\n  }\n}";
        let output = compile(src);
        assert!(
            output.contains("arr[1..(5 + 1)]"),
            "expected inclusive C# range slice: {}",
            output
        );
    }

    // `arr until 5` lowers to the half-open `arr[1..5]` form.
    #[test]
    fn test_until_index_access_lowers_to_half_open_range() {
        let src = "component Probe : MonoBehaviour {\n  func go(arr: Array<Int>) {\n    val slice = arr[1 until 5]\n  }\n}";
        let output = compile(src);
        assert!(
            output.contains("arr[1..5]"),
            "expected half-open C# range slice: {}",
            output
        );
    }

    // ── Language 5 (deferred): positional/property patterns + with ──

    // Positional pattern with sub-patterns lowers to C# 9 case syntax.
    #[test]
    fn test_positional_pattern_with_subpatterns() {
        let src = "component Probe : MonoBehaviour {\n  func describe(p: Point) {\n    when p {\n      Point(0, 0) => print(\"origin\")\n      else => print(\"other\")\n    }\n  }\n}";
        let output = compile(src);
        assert!(
            output.contains("Point(0, 0)"),
            "expected positional pattern: {}",
            output
        );
    }

    // Property pattern lowers to C# 9 `Type { x: …, y: … }` syntax.
    #[test]
    fn test_property_pattern() {
        let src = "component Probe : MonoBehaviour {\n  func describe(p: Point) {\n    when p {\n      Point { x: 0, y: > 0 } => print(\"upper\")\n      else => print(\"other\")\n    }\n  }\n}";
        let output = compile(src);
        assert!(
            output.contains("Point { x: 0, y: > 0 }"),
            "expected property pattern: {}",
            output
        );
    }

    // `receiver with { field = value }` lowers to a C# `with`-expression.
    #[test]
    // Issue #5: a `val name = receiver with { f = v }` declaration is
    // desugared to a sequence of statements (declaration + per-field
    // assignments) so the lowered C# is valid for plain `data class`
    // and Unity struct types. The previous lowering emitted the C#
    // `with` syntax, which only works on records.
    fn test_with_expression_lowers_to_csharp_with() {
        let src = "component Probe : MonoBehaviour {\n  func go(p: Point) {\n    val q = p with { x = 0 }\n  }\n}";
        let output = compile(src);
        assert!(
            output.contains("var q = p;"),
            "expected `var q = p;` as first statement of with desugar: {}",
            output
        );
        assert!(
            output.contains("q.x = 0;"),
            "expected `q.x = 0;` field mutation: {}",
            output
        );
        assert!(
            !output.contains(" with { "),
            "lowered output must not emit C# `with` syntax (invalid on plain class): {}",
            output
        );
    }

    // ── Language 5 (deferred): generalized nested class ──────────

    // A `data class` declared inside a component lowers to a nested
    // C# class in the parent type.
    #[test]
    fn test_nested_data_class_inside_component() {
        let src = "component Inventory : MonoBehaviour {\n  data class Slot(name: String, count: Int)\n}";
        let output = compile(src);
        assert!(
            output.contains("public class Inventory"),
            "expected outer class: {}",
            output
        );
        assert!(
            output.contains("Slot"),
            "expected nested Slot type: {}",
            output
        );
    }

    // A nested `enum` declared inside a class is also emitted in-place.
    #[test]
    fn test_nested_enum_inside_class() {
        let src = "class Order {\n  enum Status { Pending, Shipped, Delivered }\n}";
        let output = compile(src);
        assert!(
            output.contains("public class Order"),
            "expected outer class: {}",
            output
        );
        assert!(
            output.contains("Status"),
            "expected nested enum: {}",
            output
        );
    }

    // ── Language 5, Sprint 6 ──────────────────────────────────────

    // `arr?[0]` lowers to a C# null-conditional indexer.
    #[test]
    fn test_safe_index_access_lowers_to_question_bracket() {
        let src = "component Probe : MonoBehaviour {\n  func go(arr: List<Int>) {\n    val first = arr?[0]\n  }\n}";
        let output = compile(src);
        assert!(
            output.contains("arr?[0]"),
            "expected null-conditional indexer: {}",
            output
        );
    }

    // `throw expr` in expression position composes with the `?:` elvis
    // operator. PrSM uses Kotlin-style `?:` for null-coalescing rather
    // than C# `??`; the lowered C# uses the corresponding `??` form.
    //
    // The lowered C# must include the `new` keyword on the constructed
    // exception — a bare `throw Exception(...)` is invalid C# (CS1525).
    // This mirrors the `Stmt::Throw` lowering, which also prepends `new`.
    #[test]
    fn test_throw_expression_in_elvis() {
        let src = "component Probe : MonoBehaviour {\n  func go(body: GameObject?) {\n    val rb = body ?: throw Exception(\"missing\")\n  }\n}";
        let output = compile(src);
        assert!(
            output.contains("throw new Exception(\"missing\")"),
            "expected `throw new Exception(\"missing\")` in lowered output: {}",
            output
        );
        // Guard against the regression where `new` was missing entirely.
        assert!(
            !output.contains("throw Exception("),
            "lowered output contains bare `throw Exception(` without `new` (invalid C#): {}",
            output
        );
    }

    // Issue #13: throw expression with a variable target must NOT receive
    // a `new` prefix. `throw cached!!` should forward the variable
    // (post-#43, wrapped in a runtime null-check `?? throw` clause).
    #[test]
    fn test_throw_expression_variable() {
        let src = r#"using System
component Probe : MonoBehaviour {
  func go(body: GameObject?, cached: Exception?) {
    val rb = body ?: throw cached!!
  }
}"#;
        let output = compile(src);
        // The variable `cached` must be the thrown value (via the
        // `!!` runtime check), never wrapped with a stray `new`.
        assert!(
            output.contains("cached ??"),
            "throw of a variable should reference `cached` directly: {}",
            output
        );
        assert!(
            !output.contains("throw new cached"),
            "throw of a variable must not be wrapped with `new` (invalid C#): {}",
            output
        );
    }

    #[test]
    fn test_non_null_assert_is_preserved_in_member_access() {
        let src = "component Probe : MonoBehaviour {\n  optional cam: Camera?\n  func go() {\n    val fov = cam!!.fieldOfView\n  }\n}";
        let output = compile(src);
        // Issue #43: `!!` lowers to a runtime null check using
        // `?? throw new System.NullReferenceException(...)`, not the
        // C# null-forgiving operator (which only suppresses warnings).
        assert!(
            output.contains("?? throw new System.NullReferenceException"),
            "expected non-null assertion to lower to runtime throw: {}",
            output
        );
        assert!(
            output.contains("`cam` was null"),
            "expected descriptive null message: {}",
            output
        );
        assert!(
            output.contains(".fieldOfView"),
            "expected member access still emitted: {}",
            output
        );
        assert!(
            !output.contains("var fov = cam.fieldOfView"),
            "lowering must not drop the non-null assertion: {}",
            output
        );
    }

    // ── Language 5, Sprint 5 ──────────────────────────────────────

    // `partial component Player : ...` lowers to `public partial class`.
    #[test]
    fn test_partial_component_lowers_to_partial_class() {
        let src = "partial component Player : MonoBehaviour {\n  func go() {}\n}";
        let output = compile(src);
        assert!(
            output.contains("public partial class Player"),
            "expected partial class lowering: {}",
            output
        );
    }

    // `partial class Foo { ... }` also lowers with `partial`.
    #[test]
    fn test_partial_class_lowers_to_partial_modifier() {
        let src = "partial class Foo {\n  func bar() {}\n}";
        let output = compile(src);
        assert!(
            output.contains("public partial class Foo"),
            "expected partial class modifier: {}",
            output
        );
    }

    // ── Language 5, Sprint 4 ──────────────────────────────────────

    // Relational pattern in a `when` switch lowers to C# 9 `case > N:`.
    #[test]
    fn test_relational_pattern_in_when() {
        let src = "component Health : MonoBehaviour {\n  func grade(hp: Int) {\n    when hp {\n      > 80 => print(\"Healthy\")\n      > 30 => print(\"Hurt\")\n      else => print(\"Dying\")\n    }\n  }\n}";
        let output = compile(src);
        assert!(
            output.contains("> 80"),
            "expected > 80 case-pattern: {}",
            output
        );
        assert!(
            output.contains("> 30"),
            "expected > 30 case-pattern: {}",
            output
        );
    }

    // `not pattern` lowers to C# 9 `not` combinator.
    #[test]
    fn test_not_pattern_in_when() {
        let src = "component Probe : MonoBehaviour {\n  func describe(x: Int) {\n    when x {\n      not 0 => print(\"non-zero\")\n      else => print(\"zero\")\n    }\n  }\n}";
        let output = compile(src);
        assert!(
            output.contains("not 0"),
            "expected not 0 pattern: {}",
            output
        );
    }

    // `pattern and pattern` lowers to C# 9 `and` combinator.
    #[test]
    fn test_and_pattern_in_when() {
        let src = "component Probe : MonoBehaviour {\n  func describe(x: Int) {\n    when x {\n      > 0 and < 100 => print(\"in range\")\n      else => print(\"out of range\")\n    }\n  }\n}";
        let output = compile(src);
        assert!(
            output.contains("> 0 and < 100"),
            "expected and-combined pattern: {}",
            output
        );
    }

    // `where T : unmanaged` is forwarded as the C# unmanaged constraint.
    #[test]
    fn test_unmanaged_constraint_passes_through() {
        let src = r#"component Buf : MonoBehaviour {
  func sum<T>(values: T): Int where T : unmanaged {
    return 0
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("where T : unmanaged"),
            "expected unmanaged constraint in C# output: {}",
            output
        );
    }

    // Language 5, Sprint 3: a `bind to` site registers a continuous push
    // callback so future setter writes propagate to every target.
    #[test]
    fn test_bind_to_emits_push_targets_list_and_setter_loop() {
        let src = r#"component HUD : MonoBehaviour {
  bind hp: Int = 100
  awake {
    bind hp to label.text
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("_pushTargets_hp"),
            "expected push targets field: {}",
            output
        );
        assert!(
            output.contains("List<System.Action<int>>"),
            "expected typed action list: {}",
            output
        );
        assert!(
            output.contains("label.text = this.hp.ToString()"),
            "expected initial string conversion for text bind target: {}",
            output
        );
        assert!(
            output.contains("_pushTargets_hp.Add(__v => label.text = __v.ToString())"),
            "expected push registration to stringify non-string values for text targets: {}",
            output
        );
        assert!(
            output.contains("foreach (var __t in _pushTargets_hp) __t(value);"),
            "expected setter loop: {}",
            output
        );
    }

    // ── Language 5, Sprint 1 ──────────────────────────────────────

    // Coroutine that uses general `yield expr` and `yield break`.
    #[test]
    fn test_coroutine_yield_general_value() {
        let src = r#"component Cutscene : MonoBehaviour {
  coroutine countdown(): Seq<Int> {
    yield 3
    yield 2
    yield 1
    yield break
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("System.Collections.Generic.IEnumerator<int>"),
            "expected typed enumerator return: {}",
            output
        );
        assert!(output.contains("yield return 3;"), "yield return 3: {}", output);
        assert!(output.contains("yield return 1;"), "yield return 1: {}", output);
        assert!(output.contains("yield break;"), "yield break: {}", output);
    }

    // `[field: SerializeField]` lowering for an auto-property with the
    // `serialize` modifier.
    #[test]
    fn test_serialize_auto_property_field_target() {
        let src = "component Player : MonoBehaviour {\n  serialize var hp: Int get set\n}";
        let output = compile(src);
        assert!(
            output.contains("[field: SerializeField]"),
            "expected [field: SerializeField] attribute: {}",
            output
        );
        assert!(
            output.contains("public int hp { get; set; }"),
            "expected auto-property declaration: {}",
            output
        );
    }

    // `#if editor` block lowers to `#if UNITY_EDITOR ... #endif`.
    #[test]
    fn test_preprocessor_editor_block_emits_unity_editor() {
        let src = r#"component Foo : MonoBehaviour {
  update {
    move()
    #if editor
      drawGizmos()
    #endif
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("#if UNITY_EDITOR"),
            "expected #if UNITY_EDITOR: {}",
            output
        );
        assert!(
            output.contains("DrawGizmos();"),
            "expected guarded body: {}",
            output
        );
        assert!(output.contains("#endif"), "expected #endif: {}", output);
    }

    // ── Language 5, Sprint 2 ──────────────────────────────────────

    // `out` parameter on a func and `out val name` declaration argument.
    #[test]
    fn test_out_param_and_out_val_call() {
        let src = r#"component Probe : MonoBehaviour {
  func tryParse(input: String, out value: Int): Bool {
    intrinsic { return int.TryParse(input, out value); }
  }
  func go() {
    if tryParse("42", out val parsed) {
      log("got " + parsed)
    }
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("out int value"),
            "expected out parameter declaration: {}",
            output
        );
        assert!(
            output.contains("out var parsed"),
            "expected out var call argument: {}",
            output
        );
    }

    // `vararg` parameter — lowers to `params T[]`.
    #[test]
    fn test_vararg_parameter_lowers_to_params_array() {
        let src = r#"component Logger : MonoBehaviour {
  func log(vararg messages: String) {
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("params string[] messages"),
            "expected params string[] vararg lowering: {}",
            output
        );
    }

    // Default parameter values forward to C# default expressions.
    #[test]
    fn test_default_param_value_lowers_to_csharp() {
        let src = r#"component Spawn : MonoBehaviour {
  func make(prefab: GameObject, count: Int = 3): Int {
    return count
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("int count = 3"),
            "expected default value in lowered signature: {}",
            output
        );
    }

    // Named arguments at the call site (Kotlin `:` form).
    #[test]
    fn test_named_argument_kotlin_colon() {
        let src = r#"component Spawn : MonoBehaviour {
  func make(prefab: Int, count: Int = 1): Int { return count }
  func go() {
    val n = make(prefab: 0, count: 5)
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("count: 5"),
            "expected named argument in lowered call: {}",
            output
        );
    }

    // `@burst` annotation lowers to `[Unity.Burst.BurstCompile]`.
    #[test]
    fn test_burst_annotation_lowers_to_attribute() {
        let src = r#"component Compute : MonoBehaviour {
  @burst
  func process(): Int {
    return 42
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("[Unity.Burst.BurstCompile]"),
            "expected BurstCompile attribute: {}",
            output
        );
    }

    // `nameof(target)` — emits a verbatim C# `nameof(target)` expression.
    #[test]
    fn test_nameof_emits_csharp_nameof() {
        let src = r#"component Player : MonoBehaviour {
  var hp: Int = 100
  func tag(): String = nameof(hp)
}"#;
        let output = compile(src);
        assert!(
            output.contains("nameof(hp)"),
            "expected lowered nameof: {}",
            output
        );
    }

    // `#if ios && !editor` block — boolean operators on PrSM symbols.
    #[test]
    fn test_preprocessor_combined_condition_lowers_operators() {
        let src = r#"component Foo : MonoBehaviour {
  update {
    #if ios && !editor
      handleHaptics()
    #elif android
      handleVibration()
    #endif
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("UNITY_IOS && !UNITY_EDITOR"),
            "expected combined #if condition: {}",
            output
        );
        assert!(
            output.contains("#elif UNITY_ANDROID"),
            "expected #elif android: {}",
            output
        );
    }

    // Issue #48: `wait 500ms` (millisecond duration literal) must
    // parse and lower to `yield return new WaitForSeconds(0.5f);`.
    #[test]
    fn test_wait_500ms_parses_and_lowers() {
        let src = r#"component Probe : MonoBehaviour {
  coroutine sequence(): Seq<Unit> {
    wait 500ms
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("WaitForSeconds(0.5f)"),
            "expected `WaitForSeconds(0.5f)` for `wait 500ms`: {}",
            output
        );
    }

    // Issue #39: `val data = ...` and `var data = ...` must parse
    // even though `data` lexes as the `Data` keyword (used in
    // `data class`). Likewise for any other keyword that may
    // legitimately appear as a variable name in binding position.
    #[test]
    fn test_val_data_keyword_parses_as_identifier() {
        let src = r#"component Probe : MonoBehaviour {
  func go() {
    val data = 42
    log("$data")
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("var data = 42") || output.contains("int data = 42"),
            "expected `data` to lower as a regular variable: {}",
            output
        );
    }

    // Issue #38: a `state machine` block with multiple `state`
    // declarations must parse without an E189 false positive on
    // the second / third state. Before the fix the parser bailed
    // out after the first state and reparsed the rest as new
    // top-level declarations.
    #[test]
    fn test_state_machine_multi_state_parses() {
        let src = r#"component Door : MonoBehaviour {
  state machine doorState {
    state Closed {
      on open => Open
      on lock => Locked
    }
    state Open {
      on close => Closed
    }
    state Locked {
      on unlock => Closed
    }
  }
}"#;
        let output = compile(src);
        assert!(
            !output.contains("E189"),
            "must not raise E189 inside multi-state machine: {}",
            output
        );
        assert!(
            output.contains("Closed") && output.contains("Open") && output.contains("Locked"),
            "expected all three states to be lowered: {}",
            output
        );
    }

    // Issue #37: `class Circle(radius: Float) : Shape` (primary
    // constructor on a class, the lang-4 sealed discriminated union
    // shorthand) must parse and lower to a C# class with an
    // auto-generated constructor that initializes each field.
    #[test]
    fn test_class_primary_ctor_parses_and_lowers() {
        let src = r#"sealed class Shape {
  class Circle(radius: Float) : Shape
  class Rect(width: Float, height: Float) : Shape
}"#;
        let output = compile(src);
        assert!(
            output.contains("public sealed class Shape"),
            "expected sealed base class: {}",
            output
        );
        assert!(
            output.contains("public float radius"),
            "expected Circle.radius property: {}",
            output
        );
        assert!(
            output.contains("public Circle(float radius)"),
            "expected auto-generated Circle ctor: {}",
            output
        );
        assert!(
            output.contains("public Rect(float width, float height)"),
            "expected auto-generated Rect ctor: {}",
            output
        );
    }

    // Issue #36: a named tuple type `(hp: Int, mp: Int)` must
    // parse and lower to a C# named tuple `(int hp, int mp)`.
    // Before the fix the `parse_type` tuple branch rejected
    // `<ident>:` with E100.
    #[test]
    fn test_named_tuple_type_lowers_to_csharp_named_tuple() {
        let src = r#"component Probe : MonoBehaviour {
  func getStats(): (hp: Int, mp: Int) {
    return (100, 50)
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("(int hp, int mp)"),
            "expected named C# tuple return type: {}",
            output
        );
    }

    // Issue #35: enum entries with Rust-style sum-type payload
    // (`Ok(value: Int)`) must parse and lower to a discriminated
    // union (sealed abstract base + sealed variants). Before the
    // fix the parser rejected the `<ident>:` shape with E100.
    #[test]
    fn test_enum_named_payload_parses_and_lowers() {
        let src = r#"enum Result {
  Ok(value: Int),
  Err(message: String)
}"#;
        let output = compile(src);
        assert!(
            output.contains("public abstract class Result"),
            "expected sealed abstract base class: {}",
            output
        );
        assert!(
            output.contains("public sealed class Ok : Result"),
            "expected sealed Ok variant: {}",
            output
        );
        assert!(
            output.contains("public sealed class Err : Result"),
            "expected sealed Err variant: {}",
            output
        );
        assert!(
            output.contains("public int value"),
            "expected `int value` payload field: {}",
            output
        );
        assert!(
            output.contains("public string message"),
            "expected `string message` payload field: {}",
            output
        );
    }

    // Issue #33: the `unlisten` statement must accept the `!!`
    // non-null assertion suffix (documented in docs/en/idioms.md).
    // Before the fix the parser required a bare identifier and
    // rejected `unlisten skipToken!!` with E100.
    #[test]
    fn test_unlisten_with_non_null_assert_parses() {
        let src = r#"component Probe : MonoBehaviour {
  var skipToken: ListenToken? = null
  func go() {
    if skipToken != null {
      unlisten skipToken!!
      skipToken = null
    }
  }
}"#;
        // Must parse + lower without panicking; the unlisten-without-
        // matching-listen path emits a `/* error: */` comment which
        // is sufficient for this test.
        let output = compile(src);
        assert!(
            !output.contains("expected identifier after 'unlisten'"),
            "parser must accept `unlisten <ident>!!`: {}",
            output
        );
    }

    // Issue #33 (cont): `unlisten this.field` should also parse.
    #[test]
    fn test_unlisten_with_this_prefix_parses() {
        let src = r#"component Probe : MonoBehaviour {
  var tk: ListenToken? = null
  func go() {
    unlisten this.tk
  }
}"#;
        let output = compile(src);
        assert!(
            !output.contains("expected identifier after 'unlisten'"),
            "parser must accept `unlisten this.field`: {}",
            output
        );
    }

    // Issue #47: `length`/`count`/`message` sugar is type-gated and
    // must NOT fire for members that happen to share the name on
    // other types. `Camera.depth` is a camelCase Unity field —
    // reading `cam?.depth` must stay `cam.depth`, not `cam.Depth`.
    // Also `cam.enabled` must remain as-is (Camera has `.enabled`).
    #[test]
    fn test_camera_depth_is_not_pascal_cased() {
        let src = r#"component Probe : MonoBehaviour {
  optional cam: Camera?
  func go() {
    val depth = cam?.depth ?: 0.0
    log("$depth")
  }
}"#;
        let output = compile(src);
        // Neither `cam.Depth` nor `cam?.Depth` should appear — only
        // the lowercase form.
        assert!(
            !output.contains(".Depth"),
            "Camera.depth must not be rewritten to `.Depth`: {}",
            output
        );
        assert!(
            output.contains("cam.depth") || output.contains("cam?.depth"),
            "expected `cam.depth` (lowercase) in output: {}",
            output
        );
    }

    // Issue #47 (cont): `length` sugar must still fire for arrays,
    // strings, and span types where C# actually spells the member
    // `Length` — verifies we haven't regressed on the sugar rewrite
    // while fixing the over-application.
    #[test]
    fn test_array_length_sugar_still_fires() {
        let src = r#"component Probe : MonoBehaviour {
  func go() {
    val arr: Array<Int> = [1, 2, 3]
    val n = arr.length
    log("$n")
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("arr.Length"),
            "expected `.Length` sugar for array: {}",
            output
        );
    }

    // Issue #42: `cam?.enabled = false` (safe-call assignment)
    // must lower to a null-guarded `if` statement, not an invalid
    // rvalue-ternary assignment. Before the fix the output was
    // `(cam != null ? cam.Enabled : null) = false;` which C#
    // rejects (cannot assign to a ternary).
    #[test]
    fn test_safe_call_assignment_lowers_to_if_guard() {
        let src = r#"component Probe : MonoBehaviour {
  optional cam: Camera?
  func go() {
    cam?.enabled = false
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("if (cam != null)"),
            "expected `if (cam != null)` guard: {}",
            output
        );
        assert!(
            output.contains("cam.enabled = false"),
            "expected direct `cam.enabled = false` inside the guard: {}",
            output
        );
        assert!(
            !output.contains(") = false"),
            "must not assign to an rvalue ternary: {}",
            output
        );
    }

    // Issue #34: a single-line PrSM string literal containing an
    // escaped double quote must lower to a C# literal that re-escapes
    // the inner `"`. Before the fix the lexer's decoded payload
    // (`escape " quote`) was emitted verbatim, producing invalid C#
    // (`"escape " quote"`).
    #[test]
    fn test_string_literal_escapes_inner_quote() {
        let src = r#"component Probe : MonoBehaviour {
  func go() {
    log("escape \" quote")
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains(r#"\" quote"#),
            "expected re-escaped inner quote: {}",
            output
        );
        assert!(
            !output.contains(r#""escape " quote""#),
            "lowered string must not contain unescaped inner quote: {}",
            output
        );
    }

    // Issue #34 (cont): backslashes must double up; otherwise a
    // Windows path or escape sequence smashes the C# string.
    #[test]
    fn test_string_literal_escapes_backslash() {
        let src = r#"component Probe : MonoBehaviour {
  func go() {
    log("c:\\temp\\file.txt")
  }
}"#;
        let output = compile(src);
        // The PrSM source contains `\\` (two chars) which the lexer
        // decodes to a single backslash byte. The lowering must
        // re-escape it to `\\`.
        assert!(
            output.contains(r#"c:\\temp\\file.txt"#),
            "expected re-escaped backslashes: {}",
            output
        );
    }

    // Issue #34 (cont): an interpolated string literal segment with
    // a literal `{` must double the brace so the C# interpolation
    // parser does not treat it as the start of an expression.
    #[test]
    fn test_string_interp_escapes_braces_and_quotes() {
        let src = r#"component Probe : MonoBehaviour {
  func go(name: String) {
    log("hello {literal} \"$name\"")
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("{{literal}}"),
            "expected doubled `{{{{` for literal brace inside interpolated string: {}",
            output
        );
        assert!(
            output.contains(r#"\""#),
            "expected escaped inner quote inside interpolated string: {}",
            output
        );
    }

    // Issue #40: a `for i in 10 downTo 0` loop must lower to a
    // descending C# `for` (`i >= 0; i--`), not the ascending shape
    // produced by `..` and `until`. Before this fix the parser
    // discarded the direction and the lowering emitted `i <= 0; i++`,
    // which silently never executed the body when start > end.
    #[test]
    fn test_for_loop_downto_lowers_descending() {
        let src = r#"component Probe : MonoBehaviour {
  func go() {
    for i in 10 downTo 0 {
      log("$i")
    }
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("i >= 0"),
            "expected `i >= 0` for descending range: {}",
            output
        );
        assert!(
            output.contains("i--"),
            "expected `i--` for descending range: {}",
            output
        );
        assert!(
            !output.contains("i <= 0"),
            "should NOT emit ascending `i <= 0` for downTo: {}",
            output
        );
    }

    // Issue #41: a `for f in 0.0..1.0 step 0.1` loop must declare
    // the induction variable as `float`, not `int`. Before the fix
    // the lowering hard-coded `int f` which would not compile against
    // the `0.0f`/`0.1f` initial/step values.
    #[test]
    fn test_for_loop_float_range_induction_is_float() {
        let src = r#"component Probe : MonoBehaviour {
  func go() {
    for f in 0.0..1.0 step 0.1 {
      log("$f")
    }
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("float f = 0"),
            "expected `float f` induction variable for float range: {}",
            output
        );
        assert!(
            !output.contains("int f = 0"),
            "must not emit `int f` induction for float range: {}",
            output
        );
    }

    // Issue #41 (cont): an integer range must still lower to `int`.
    #[test]
    fn test_for_loop_int_range_induction_stays_int() {
        let src = r#"component Probe : MonoBehaviour {
  func go() {
    for i in 0 until 10 {
      log("$i")
    }
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("int i = 0"),
            "expected `int i` induction variable for integer range: {}",
            output
        );
    }

    // Issue #40 (cont): explicit `step` on a `downTo` range must
    // subtract the step on each iteration.
    #[test]
    fn test_for_loop_downto_with_step_subtracts() {
        let src = r#"component Probe : MonoBehaviour {
  func go() {
    for i in 100 downTo 0 step 5 {
      log("$i")
    }
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("i -= 5"),
            "expected `i -= 5` for descending range with step: {}",
            output
        );
        assert!(
            output.contains("i >= 0"),
            "expected `i >= 0` for descending range with step: {}",
            output
        );
    }

    // Issue #49: `try { ... } catch (e: T) { ... }` as expression.
    #[test]
    fn test_try_expression_as_value() {
        let src = r#"component Probe : MonoBehaviour {
  func parse() {
    val result = try { parseInt("42") } catch (e: Exception) { -1 }
    log("$result")
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("System.Func<object>"),
            "try expression should lower to an IIFE returning object: {}",
            output
        );
        assert!(
            output.contains("catch (Exception e)"),
            "try expression should preserve catch type: {}",
            output
        );
    }

    // Issue #50: trailing-lambda call `list.filter { it > 10 }`.
    #[test]
    fn test_trailing_lambda_bare_call() {
        let src = r#"component Probe : MonoBehaviour {
  func go(list: List<Int>) {
    list.filter { it > 10 }
  }
}"#;
        let output = compile(src);
        // Method names get PascalCased in the C# emit, so match loosely.
        assert!(
            output.to_lowercase().contains("list.filter"),
            "trailing lambda call should lower to a filter method call: {}",
            output
        );
        assert!(
            output.contains("it > 10"),
            "trailing lambda body should survive: {}",
            output
        );
    }

    // Issue #50 (cont): trailing lambda after positional args.
    #[test]
    fn test_trailing_lambda_with_positional_arg() {
        let src = r#"component Probe : MonoBehaviour {
  func go(xs: List<Int>) {
    xs.fold(0) { acc, x => acc + x }
  }
}"#;
        let output = compile(src);
        assert!(
            output.to_lowercase().contains("xs.fold"),
            "trailing lambda with positional arg should still call fold: {}",
            output
        );
        assert!(
            output.contains("acc + x"),
            "trailing lambda body should survive: {}",
            output
        );
    }

    // Issue #50 (cont): `if expr { body }` must NOT be misread as a
    // trailing-lambda call on `expr`.
    #[test]
    fn test_if_with_method_call_keeps_body_separate() {
        let src = r#"component Probe : MonoBehaviour {
  func go() {
    if isValid() {
      log("ok")
    }
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("if (IsValid())"),
            "if expression body must not be absorbed as trailing lambda: {}",
            output
        );
    }

    // Issue #52: `is Enemy or is Boss` pattern combinator.
    #[test]
    fn test_pattern_or_combinator_parses() {
        let src = r#"component Probe : MonoBehaviour {
  func classify(target: Object) {
    when target {
      is Enemy or is Boss => log("hostile")
      else => log("ok")
    }
  }
}"#;
        // Parse-only: the exact C# shape depends on the switch lowering,
        // but it must compile without errors.
        let _ = compile(src);
    }

    // Issue #53: multi-line call arguments.
    #[test]
    fn test_multiline_call_arguments_parse() {
        let src = r#"component Probe : MonoBehaviour {
  func go() {
    log(
      "a",
      "b"
    )
  }
}"#;
        let output = compile(src);
        // `log` lowers to `Debug.Log`; the two args must survive.
        assert!(
            output.contains("\"a\", \"b\""),
            "multi-line call args should fold back to single-line emit: {}",
            output
        );
    }

    // Issue #54: multi-constraint `where T : A, B` clauses.
    #[test]
    fn test_multi_constraint_where_clause_parses() {
        let src = r#"class Registry<T> where T : MonoBehaviour, IDamageable {
  var items: List<T> = null
}"#;
        let output = compile(src);
        assert!(
            output.contains("where T : MonoBehaviour, IDamageable"),
            "multi-constraint where clause should survive to C#: {}",
            output
        );
    }

    // Issue #55: `@burst(compileSynchronously = true)` named argument.
    #[test]
    fn test_annotation_named_argument_lowers_to_pascal() {
        let src = r#"@burst(compileSynchronously = true)
func calculateForces() {
}"#;
        let output = compile(src);
        assert!(
            output.contains("CompileSynchronously = true"),
            "named annotation arg should PascalCase to C# property: {}",
            output
        );
    }

    // Issue #56: multi-line binary expression continuation.
    #[test]
    fn test_multiline_binary_expression_parses() {
        let src = r#"component Probe : MonoBehaviour {
  func go() {
    val x = 1 +
      2
    log("$x")
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("1 + 2"),
            "multi-line binary expr should fold to `1 + 2`: {}",
            output
        );
    }

    // Issue #57: method chain across newlines via leading-dot continuation.
    #[test]
    fn test_leading_dot_chain_across_newlines() {
        let src = r#"component Probe : MonoBehaviour {
  func go(list: List<Int>) {
    list
      .where({ x => x > 10 })
      .select({ x => x * 2 })
  }
}"#;
        // Parse-only: the actual emission depends on method resolution.
        let _ = compile(src);
    }

    // Issue #58: open-ended range slice `arr[2..]` and `arr[..3]`.
    #[test]
    fn test_open_ended_range_slice_parses() {
        let src = r#"component Probe : MonoBehaviour {
  func go(arr: Array<Int>) {
    val tail = arr[2..]
    val head = arr[..3]
    log("$tail $head")
  }
}"#;
        let output = compile(src);
        // `..` is inclusive in PrSM so `[..3]` becomes `[..(3 + 1)]`.
        assert!(
            output.contains("arr[2..]"),
            "open-ended upper range should lower to C# range slice: {}",
            output
        );
        assert!(
            output.contains("arr[..(3 + 1)]") || output.contains("arr[..3]"),
            "open-ended lower range should lower to C# range slice: {}",
            output
        );
    }

    // Issue #60: member-position `listen` declaration.
    #[test]
    fn test_member_listen_declaration_registers_in_awake() {
        let src = r#"component ShopUI : MonoBehaviour {
  require buyButton: Button

  listen buyButton.onClick until disable {
    purchaseSelectedItem()
  }

  func purchaseSelectedItem() { }
}"#;
        let output = compile(src);
        assert!(
            output.contains("void Awake()"),
            "member-level listen should produce an Awake method: {}",
            output
        );
        assert!(
            output.contains("buyButton.onClick"),
            "listen event should appear in awake body: {}",
            output
        );
    }

    // Issue #61: named tuple literal value `(hp: 100, mp: 50)`.
    #[test]
    fn test_named_tuple_literal_lowers_to_csharp_named_tuple() {
        let src = r#"component Probe : MonoBehaviour {
  func getStats(): (hp: Int, mp: Int) = (hp: 100, mp: 50)
}"#;
        let output = compile(src);
        assert!(
            output.contains("hp: 100"),
            "named tuple literal element should preserve name: {}",
            output
        );
        assert!(
            output.contains("mp: 50"),
            "named tuple literal element should preserve name: {}",
            output
        );
    }

    // Issue #62: `for (k, v) in pairs` tuple destructure.
    #[test]
    fn test_for_tuple_destructure_lowers_to_foreach_with_tuple_pattern() {
        let src = r#"component Probe : MonoBehaviour {
  func go(pairs: List<(Int, String)>) {
    for (k, v) in pairs {
      log("$k: $v")
    }
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("foreach (var (k, v) in pairs)"),
            "for-loop tuple destructure should lower to C# tuple deconstruction: {}",
            output
        );
    }

    // Issue #59: top-level `func` and `const` declarations are wrapped
    // in a synthetic `partial class Globals`.
    #[test]
    fn test_top_level_func_wraps_in_globals() {
        let src = r#"func calculateForces() {
  log("go")
}"#;
        let output = compile(src);
        assert!(
            output.contains("partial class Globals"),
            "top-level func should wrap in Globals class: {}",
            output
        );
        assert!(
            output.contains("CalculateForces"),
            "function body must survive wrapping: {}",
            output
        );
    }

    // Issue #93: extension method visibility must be honored, not
    // hardcoded to `public static`.
    #[test]
    fn test_extension_method_private_visibility_preserved() {
        let src = r#"extend String {
  private func myHelper(): Unit {
    log("internal")
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("private static void myHelper"),
            "extension method must honor private visibility: {}",
            output
        );
    }

    #[test]
    fn test_extension_method_public_visibility_preserved() {
        let src = r#"extend String {
  public func capitalize(): String {
    return this
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("public static"),
            "extension method must honor public visibility: {}",
            output
        );
    }

    // Issue #94: generic extension class names must encode the type
    // parameters so `extend List<Int>` and `extend List<String>` do
    // not collide.
    #[test]
    fn test_generic_extension_class_name_includes_type_args() {
        let src = r#"extend List<Int> {
  func sumAll(): Int {
    return 0
  }
}"#;
        let output = compile(src);
        // The generated class name must NOT be the bare "ListExtensions".
        // It should include the type parameter in some form.
        assert!(
            output.contains("ListOfInt") || output.contains("ListOfint")
                || output.contains("ListOfIntExtensions")
                || output.contains("ListOfintExtensions"),
            "generic extension class name should encode type arg: {}",
            output
        );
    }

    // Issue #95: listen handler field type must match the event's
    // declared function type. For a `(Int) => Unit` event, the
    // backing field should be `System.Action<int>`, not the bare
    // `System.Action`.
    #[test]
    fn test_listen_handler_field_uses_parameterized_delegate() {
        let src = r#"component UI : MonoBehaviour {
  event onDamaged: (Int) => Unit

  awake {
    listen onDamaged until disable { dmg =>
      log(dmg)
    }
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("System.Action<int>"),
            "parameterized event listen should yield System.Action<int> handler field: {}",
            output
        );
    }

    // Issue #95 (cont): zero-argument events continue to use the
    // bare `System.Action` handler field.
    #[test]
    fn test_listen_handler_field_zero_arg_uses_plain_action() {
        let src = r#"component UI : MonoBehaviour {
  event onStart: () => Unit

  awake {
    listen onStart until disable {
      log("begin")
    }
  }
}"#;
        let output = compile(src);
        assert!(
            output.contains("private System.Action _prsm_h"),
            "zero-arg event listen should yield System.Action handler field: {}",
            output
        );
    }
}
