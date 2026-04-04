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
    fn test_if_else() {
        let output = compile("component Foo : MonoBehaviour {\n  func f() {\n    if hp <= 0 {\n      die()\n    } else {\n      run()\n    }\n  }\n}");
        assert!(output.contains("if (hp <= 0)"));
        assert!(output.contains("die();"));
        assert!(output.contains("else"));
        assert!(output.contains("run();"));
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
    fn test_asset_declaration() {
        let output = compile("asset WeaponData : ScriptableObject {\n  serialize damage: Int = 10\n}");
        assert!(output.contains("[CreateAssetMenu"));
        assert!(output.contains("public class WeaponData : ScriptableObject"));
        assert!(output.contains("[SerializeField]"));
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
    fn test_data_class() {
        let output = compile("data class DamageInfo(\n  val amount: Int,\n  val crit: Bool\n)");
        assert!(output.contains("[System.Serializable]"));
        assert!(output.contains("public class DamageInfo"));
        assert!(output.contains("public int amount"));
        assert!(output.contains("public bool crit"));
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
}
