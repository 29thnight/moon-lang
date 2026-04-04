use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("{}_{}", prefix, unique));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn moonc() -> &'static str {
    env!("CARGO_BIN_EXE_moonc")
}

fn write_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

#[test]
fn compile_json_smoke() {
    let root = unique_temp_dir("moonc_compile_smoke");
    let source = root.join("Hello.mn");
    write_file(&source, "component Hello : MonoBehaviour {}");

    let output = Command::new(moonc())
        .args(["compile", source.to_str().unwrap(), "--json"])
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["files"], 1);
    assert_eq!(json["compiled"], 1);
    assert_eq!(json["errors"], 0);
    assert!(root.join("Hello.cs").exists());
    assert!(root.join("Hello.mnmap.json").exists());
    assert_eq!(json["outputs"][0]["source_map"], root.join("Hello.mnmap.json").to_string_lossy().to_string());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn check_json_reports_errors() {
    let root = unique_temp_dir("moonc_check_smoke");
    let source = root.join("Broken.mn");
    write_file(&source, "enum Broken {}");

    let output = Command::new(moonc())
        .args(["check", source.to_str().unwrap(), "--json"])
        .output()
        .unwrap();

    assert!(!output.status.success(), "check should fail for invalid enum");

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["files"], 1);
    assert_eq!(json["errors"], 1);
    assert_eq!(json["diagnostics"][0]["code"], "E050");
    assert!(json["diagnostics"][0]["end_line"].as_u64().is_some());
    assert!(json["diagnostics"][0]["end_col"].as_u64().is_some());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn build_project_json_smoke() {
    let root = unique_temp_dir("moonc_build_smoke");
    write_file(
        &root.join(".mnproject"),
        r#"[project]
name = "SmokeProject"
moon_version = "0.1.0"

[language]
version = "1.0"
features = ["input-system"]

[compiler]
moonc_path = "moonc"
output_dir = "Generated/Moon"

[source]
include = ["Assets/**/*.mn"]
exclude = []
"#,
    );
    write_file(
        &root.join("Packages").join("manifest.json"),
        r#"{
  "dependencies": {
    "com.unity.inputsystem": "1.8.1"
  }
}"#,
    );
    write_file(
        &root.join("Assets").join("BuildTarget.mn"),
        "component BuildTarget : MonoBehaviour {}",
    );

    let output = Command::new(moonc())
        .args(["build", "--json"])
        .current_dir(&root)
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json_start = stdout.find('{').expect("expected JSON output");
    let json: serde_json::Value = serde_json::from_str(&stdout[json_start..]).unwrap();

    assert_eq!(json["project"], "SmokeProject");
    assert_eq!(json["files"], 1);
    assert_eq!(json["compiled"], 1);
    assert_eq!(json["errors"], 0);
    assert_eq!(json["language_version"], "1.0");
    assert_eq!(json["language_features"][0], "input-system");
    assert_eq!(json["hir"]["files_indexed"], 1);
    assert_eq!(json["hir"]["definitions"], 1);
    assert_eq!(json["hir"]["references"], 0);
    assert_eq!(json["index"]["files_indexed"], 1);
    assert_eq!(json["index"]["top_level_symbols"], 1);
    assert_eq!(json["index"]["member_symbols"], 0);
    assert_eq!(json["index"]["total_symbols"], 1);
    assert_eq!(json["unity_capabilities"]["input_system"], true);
    assert!(json["cache_dir"].as_str().is_some());
    assert!(root.join("Generated").join("Moon").join("BuildTarget.cs").exists());
    assert!(root.join("Generated").join("Moon").join("BuildTarget.mnmap.json").exists());
    assert!(root.join(".moon").join("cache").exists());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn compile_source_map_sidecar_contains_member_anchors() {
    let root = unique_temp_dir("moonc_source_map_sidecar_smoke");
    let source = root.join("Player.mn");
    write_file(
        &source,
        r#"component Player : MonoBehaviour {
    serialize speed: Float = 5.0

    update {
        speed += 1.0
    }

    func jump(): Unit {
        return
    }
}
"#,
    );

    let output = Command::new(moonc())
        .args(["compile", source.to_str().unwrap(), "--json"])
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));

    let source_map_path = root.join("Player.mnmap.json");
    let source_map_text = fs::read_to_string(&source_map_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&source_map_text).unwrap();

    assert_eq!(json["version"], 1);
    assert_eq!(json["declaration"]["name"], "Player");
    assert!(json["members"]
        .as_array()
        .unwrap()
        .iter()
        .any(|member| member["name"] == "speed" && member["generated_name_span"]["line"].as_u64().is_some()));
    assert!(json["members"]
        .as_array()
        .unwrap()
        .iter()
        .any(|member| member["name"] == "update" && member["generated_name_span"]["line"].as_u64().is_some()));
    assert!(json["members"]
        .as_array()
        .unwrap()
        .iter()
        .any(|member| member["name"] == "jump" && member["generated_name_span"]["line"].as_u64().is_some()));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn build_project_rejects_unknown_language_version() {
    let root = unique_temp_dir("moonc_build_invalid_language_version");
    write_file(
        &root.join(".mnproject"),
        r#"[project]
name = "BrokenProject"

[language]
version = "3.0"
features = []

[source]
include = ["Assets/**/*.mn"]
exclude = []
"#,
    );
    write_file(
        &root.join("Assets").join("BuildTarget.mn"),
        "component BuildTarget : MonoBehaviour {}",
    );

    let output = Command::new(moonc())
        .args(["build", "--json"])
        .current_dir(&root)
        .output()
        .unwrap();

    assert!(!output.status.success(), "build should fail for invalid language version");

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Unsupported language version '3.0'"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn index_project_json_smoke() {
    let root = unique_temp_dir("moonc_index_smoke");
    write_file(
        &root.join(".mnproject"),
        r#"[project]
name = "IndexProject"

[language]
version = "2.0"
features = ["pattern-bindings"]

[source]
include = ["Assets/**/*.mn"]
exclude = []
"#,
    );
    write_file(
        &root.join("Assets").join("Player.mn"),
        r#"component Player : MonoBehaviour {
    func jump(): Unit {
        return
    }
}
"#,
    );

    let output = Command::new(moonc())
        .args(["index", ".", "--json"])
        .current_dir(&root)
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(json["project"], "IndexProject");
    assert_eq!(json["language_version"], "2.0");
    assert_eq!(json["language_features"][0], "pattern-bindings");
    assert_eq!(json["index"]["files_indexed"], 1);
    assert_eq!(json["index"]["total_symbols"], 2);
    assert_eq!(json["matches"].as_array().unwrap().len(), 2);
    assert!(json["matches"]
        .as_array()
        .unwrap()
        .iter()
        .any(|symbol| symbol["qualified_name"] == "Player.jump" && symbol["kind"] == "function"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn index_project_qualified_name_filter_smoke() {
    let root = unique_temp_dir("moonc_index_filter_smoke");
    write_file(
        &root.join(".mnproject"),
        r#"[project]
name = "IndexProject"

[source]
include = ["Assets/**/*.mn"]
exclude = []
"#,
    );
    write_file(
        &root.join("Assets").join("Player.mn"),
        r#"component Player : MonoBehaviour {
    func jump(): Unit {
        return
    }

    func land(): Unit {
        return
    }
}
"#,
    );

    let output = Command::new(moonc())
        .args(["index", ".", "--json", "--qualified-name", "Player.jump"])
        .current_dir(&root)
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let matches = json["matches"].as_array().unwrap();

    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0]["qualified_name"], "Player.jump");
    assert_eq!(matches[0]["kind"], "function");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn index_project_position_lookup_smoke() {
    let root = unique_temp_dir("moonc_index_position_smoke");
    write_file(
        &root.join(".mnproject"),
        r#"[project]
name = "IndexProject"

[source]
include = ["Assets/**/*.mn"]
exclude = []
"#,
    );
    write_file(
        &root.join("Assets").join("Player.mn"),
        r#"component Player : MonoBehaviour {
    func jump(): Unit {
        return
    }
}
"#,
    );

    let output = Command::new(moonc())
        .args([
            "index",
            ".",
            "--json",
            "--file",
            "Assets/Player.mn",
            "--line",
            "2",
            "--col",
            "10",
        ])
        .current_dir(&root)
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(json["position_query"]["line"], 2);
    assert_eq!(json["position_query"]["col"], 10);
    assert_eq!(json["symbol_at"]["qualified_name"], "Player.jump");
    assert_eq!(json["symbol_at"]["kind"], "function");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn index_project_top_level_precision_smoke() {
    let root = unique_temp_dir("moonc_index_top_level_precision_smoke");
    write_file(
        &root.join(".mnproject"),
        r#"[project]
name = "IndexProject"

[source]
include = ["Assets/**/*.mn"]
exclude = []
"#,
    );
    write_file(
        &root.join("Assets").join("Player.mn"),
        "component Player : MonoBehaviour {}",
    );

    let name_output = Command::new(moonc())
        .args([
            "index",
            ".",
            "--json",
            "--file",
            "Assets/Player.mn",
            "--line",
            "1",
            "--col",
            "11",
        ])
        .current_dir(&root)
        .output()
        .unwrap();

    assert!(name_output.status.success(), "{}", String::from_utf8_lossy(&name_output.stderr));

    let name_stdout = String::from_utf8(name_output.stdout).unwrap();
    let name_json: serde_json::Value = serde_json::from_str(&name_stdout).unwrap();

    assert_eq!(name_json["symbol_at"]["qualified_name"], "Player");
    assert_eq!(name_json["symbol_at"]["kind"], "component");
    assert_eq!(name_json["symbol_at"]["line"], 1);
    assert_eq!(name_json["symbol_at"]["col"], 11);

    let base_output = Command::new(moonc())
        .args([
            "index",
            ".",
            "--json",
            "--file",
            "Assets/Player.mn",
            "--line",
            "1",
            "--col",
            "20",
        ])
        .current_dir(&root)
        .output()
        .unwrap();

    assert!(base_output.status.success(), "{}", String::from_utf8_lossy(&base_output.stderr));

    let base_stdout = String::from_utf8(base_output.stdout).unwrap();
    let base_json: serde_json::Value = serde_json::from_str(&base_stdout).unwrap();

    assert!(base_json["symbol_at"].is_null());
    assert_eq!(base_json["reference_at"]["name"], "MonoBehaviour");
    assert_eq!(base_json["reference_at"]["kind"], "type");
    assert!(base_json["reference_at"]["resolved_symbol"].is_null());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn index_project_header_type_reference_smoke() {
    let root = unique_temp_dir("moonc_index_header_type_reference_smoke");
    write_file(
        &root.join(".mnproject"),
        r#"[project]
name = "IndexProject"

[source]
include = ["Assets/**/*.mn"]
exclude = []
"#,
    );
    write_file(
        &root.join("Assets").join("BaseActor.mn"),
        "class BaseActor {}",
    );
    write_file(
        &root.join("Assets").join("InterfaceLike.mn"),
        "class InterfaceLike {}",
    );
    write_file(
        &root.join("Assets").join("Player.mn"),
        "component Player : BaseActor, InterfaceLike {}",
    );

    let base_output = Command::new(moonc())
        .args([
            "index",
            ".",
            "--json",
            "--file",
            "Assets/Player.mn",
            "--line",
            "1",
            "--col",
            "22",
        ])
        .current_dir(&root)
        .output()
        .unwrap();

    assert!(base_output.status.success(), "{}", String::from_utf8_lossy(&base_output.stderr));

    let base_stdout = String::from_utf8(base_output.stdout).unwrap();
    let base_json: serde_json::Value = serde_json::from_str(&base_stdout).unwrap();

    assert!(base_json["symbol_at"].is_null());
    assert_eq!(base_json["reference_at"]["name"], "BaseActor");
    assert_eq!(base_json["reference_at"]["kind"], "type");
    assert_eq!(base_json["reference_at"]["resolved_symbol"]["qualified_name"], "BaseActor");

    let interface_output = Command::new(moonc())
        .args([
            "index",
            ".",
            "--json",
            "--file",
            "Assets/Player.mn",
            "--line",
            "1",
            "--col",
            "33",
        ])
        .current_dir(&root)
        .output()
        .unwrap();

    assert!(interface_output.status.success(), "{}", String::from_utf8_lossy(&interface_output.stderr));

    let interface_stdout = String::from_utf8(interface_output.stdout).unwrap();
    let interface_json: serde_json::Value = serde_json::from_str(&interface_stdout).unwrap();

    assert!(interface_json["symbol_at"].is_null());
    assert_eq!(interface_json["reference_at"]["name"], "InterfaceLike");
    assert_eq!(interface_json["reference_at"]["kind"], "type");
    assert_eq!(interface_json["reference_at"]["resolved_symbol"]["qualified_name"], "InterfaceLike");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn index_project_member_type_reference_smoke() {
    let root = unique_temp_dir("moonc_index_member_type_reference_smoke");
    write_file(
        &root.join(".mnproject"),
        r#"[project]
name = "IndexProject"

[source]
include = ["Assets/**/*.mn"]
exclude = []
"#,
    );
    write_file(
        &root.join("Assets").join("WeaponData.mn"),
        "data class WeaponData(val damage: Int)",
    );
    write_file(
        &root.join("Assets").join("Player.mn"),
        r#"component Player : MonoBehaviour {
    serialize var equipped: WeaponData

    func equip(next: WeaponData): WeaponData {
        val backup: List<WeaponData> = next
        return next
    }
}
"#,
    );

    let field_output = Command::new(moonc())
        .args([
            "index",
            ".",
            "--json",
            "--file",
            "Assets/Player.mn",
            "--line",
            "2",
            "--col",
            "31",
        ])
        .current_dir(&root)
        .output()
        .unwrap();

    assert!(field_output.status.success(), "{}", String::from_utf8_lossy(&field_output.stderr));

    let field_stdout = String::from_utf8(field_output.stdout).unwrap();
    let field_json: serde_json::Value = serde_json::from_str(&field_stdout).unwrap();

    assert!(field_json["symbol_at"].is_null());
    assert_eq!(field_json["reference_at"]["name"], "WeaponData");
    assert_eq!(field_json["reference_at"]["kind"], "type");
    assert_eq!(field_json["reference_at"]["resolved_symbol"]["qualified_name"], "WeaponData");

    let return_output = Command::new(moonc())
        .args([
            "index",
            ".",
            "--json",
            "--file",
            "Assets/Player.mn",
            "--line",
            "4",
            "--col",
            "37",
        ])
        .current_dir(&root)
        .output()
        .unwrap();

    assert!(return_output.status.success(), "{}", String::from_utf8_lossy(&return_output.stderr));

    let return_stdout = String::from_utf8(return_output.stdout).unwrap();
    let return_json: serde_json::Value = serde_json::from_str(&return_stdout).unwrap();

    assert!(return_json["symbol_at"].is_null());
    assert_eq!(return_json["reference_at"]["name"], "WeaponData");
    assert_eq!(return_json["reference_at"]["resolved_symbol"]["qualified_name"], "WeaponData");

    let local_output = Command::new(moonc())
        .args([
            "index",
            ".",
            "--json",
            "--file",
            "Assets/Player.mn",
            "--line",
            "5",
            "--col",
            "31",
        ])
        .current_dir(&root)
        .output()
        .unwrap();

    assert!(local_output.status.success(), "{}", String::from_utf8_lossy(&local_output.stderr));

    let local_stdout = String::from_utf8(local_output.stdout).unwrap();
    let local_json: serde_json::Value = serde_json::from_str(&local_stdout).unwrap();

    assert!(local_json["symbol_at"].is_null());
    assert_eq!(local_json["reference_at"]["name"], "WeaponData");
    assert_eq!(local_json["reference_at"]["resolved_symbol"]["qualified_name"], "WeaponData");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn hir_project_json_smoke() {
    let root = unique_temp_dir("moonc_hir_smoke");
    write_file(
        &root.join(".mnproject"),
        r#"[project]
name = "HirProject"

[source]
include = ["Assets/**/*.mn"]
exclude = []
"#,
    );
    write_file(
        &root.join("Assets").join("Player.mn"),
        r#"component Player : MonoBehaviour {
    func jump(): Unit {
        val speed = 5
        val next = speed
    }
}
"#,
    );

    let output = Command::new(moonc())
        .args(["hir", ".", "--json"])
        .current_dir(&root)
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(json["project"], "HirProject");
    assert_eq!(json["stats"]["files_indexed"], 1);
    assert_eq!(json["stats"]["definitions"], 4);
    assert_eq!(json["stats"]["references"], 1);
    assert_eq!(json["stats"]["resolved_references"], 1);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn definition_local_symbol_smoke() {
    let root = unique_temp_dir("moonc_definition_smoke");
    write_file(
        &root.join(".mnproject"),
        r#"[project]
name = "DefinitionProject"

[source]
include = ["Assets/**/*.mn"]
exclude = []
"#,
    );
    write_file(
        &root.join("Assets").join("Player.mn"),
        r#"component Player : MonoBehaviour {
    func jump(): Unit {
        val speed = 5
        val next = speed
    }
}
"#,
    );

    let output = Command::new(moonc())
        .args([
            "definition",
            ".",
            "--json",
            "--file",
            "Assets/Player.mn",
            "--line",
            "4",
            "--col",
            "20",
        ])
        .current_dir(&root)
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(json["definition"]["qualified_name"], "Player.jump.speed");
    assert_eq!(json["definition"]["kind"], "local");
    assert_eq!(json["definition"]["line"], 3);
    assert_eq!(json["definition"]["col"], 13);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn definition_parameter_symbol_smoke() {
    let root = unique_temp_dir("moonc_definition_parameter_smoke");
    write_file(
        &root.join(".mnproject"),
        r#"[project]
name = "DefinitionProject"

[source]
include = ["Assets/**/*.mn"]
exclude = []
"#,
    );
    write_file(
        &root.join("Assets").join("Player.mn"),
        r#"component Player : MonoBehaviour {
    func jump(weapon: WeaponData): Unit {
        val amount = weapon
    }
}
"#,
    );

    let output = Command::new(moonc())
        .args([
            "definition",
            ".",
            "--json",
            "--file",
            "Assets/Player.mn",
            "--line",
            "3",
            "--col",
            "23",
        ])
        .current_dir(&root)
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(json["definition"]["qualified_name"], "Player.jump.weapon");
    assert_eq!(json["definition"]["kind"], "parameter");
    assert_eq!(json["definition"]["line"], 2);
    assert_eq!(json["definition"]["col"], 15);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn definition_parameter_type_symbol_smoke() {
    let root = unique_temp_dir("moonc_definition_parameter_type_smoke");
    write_file(
        &root.join(".mnproject"),
        r#"[project]
name = "DefinitionProject"

[source]
include = ["Assets/**/*.mn"]
exclude = []
"#,
    );
    write_file(
        &root.join("Assets").join("WeaponData.mn"),
        r#"data class WeaponData(
    val damage: Int
)
"#,
    );
    write_file(
        &root.join("Assets").join("Player.mn"),
        r#"component Player : MonoBehaviour {
    func jump(weapon: WeaponData): Unit {
        val amount = weapon
    }
}
"#,
    );

    let output = Command::new(moonc())
        .args([
            "definition",
            ".",
            "--json",
            "--file",
            "Assets/Player.mn",
            "--line",
            "2",
            "--col",
            "25",
        ])
        .current_dir(&root)
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(json["definition"]["qualified_name"], "WeaponData");
    assert_eq!(json["definition"]["kind"], "type");
    assert_eq!(json["definition"]["line"], 1);
    assert_eq!(json["definition"]["col"], 12);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn definition_generic_type_argument_symbol_smoke() {
    let root = unique_temp_dir("moonc_definition_generic_type_arg_smoke");
    write_file(
        &root.join(".mnproject"),
        r#"[project]
name = "DefinitionProject"

[source]
include = ["Assets/**/*.mn"]
exclude = []
"#,
    );
    write_file(
        &root.join("Assets").join("WeaponData.mn"),
        r#"data class WeaponData(
    val damage: Int
)
"#,
    );
    write_file(
        &root.join("Assets").join("Player.mn"),
        r#"component Player : MonoBehaviour {
    func jump(weapons: List<WeaponData>): Unit {
        return
    }
}
"#,
    );

    let output = Command::new(moonc())
        .args([
            "definition",
            ".",
            "--json",
            "--file",
            "Assets/Player.mn",
            "--line",
            "2",
            "--col",
            "31",
        ])
        .current_dir(&root)
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(json["definition"]["qualified_name"], "WeaponData");
    assert_eq!(json["definition"]["kind"], "type");
    assert_eq!(json["definition"]["line"], 1);
    assert_eq!(json["definition"]["col"], 12);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn definition_component_header_type_symbol_smoke() {
    let root = unique_temp_dir("moonc_definition_component_header_type_smoke");
    write_file(
        &root.join(".mnproject"),
        r#"[project]
name = "DefinitionProject"

[source]
include = ["Assets/**/*.mn"]
exclude = []
"#,
    );
    write_file(
        &root.join("Assets").join("BaseActor.mn"),
        "class BaseActor {}",
    );
    write_file(
        &root.join("Assets").join("InterfaceLike.mn"),
        "class InterfaceLike {}",
    );
    write_file(
        &root.join("Assets").join("Player.mn"),
        "component Player : BaseActor, InterfaceLike {}",
    );

    let base_output = Command::new(moonc())
        .args([
            "definition",
            ".",
            "--json",
            "--file",
            "Assets/Player.mn",
            "--line",
            "1",
            "--col",
            "22",
        ])
        .current_dir(&root)
        .output()
        .unwrap();

    assert!(base_output.status.success(), "{}", String::from_utf8_lossy(&base_output.stderr));

    let base_stdout = String::from_utf8(base_output.stdout).unwrap();
    let base_json: serde_json::Value = serde_json::from_str(&base_stdout).unwrap();

    assert_eq!(base_json["definition"]["qualified_name"], "BaseActor");
    assert_eq!(base_json["definition"]["kind"], "type");
    assert_eq!(base_json["definition"]["line"], 1);
    assert_eq!(base_json["definition"]["col"], 7);

    let interface_output = Command::new(moonc())
        .args([
            "definition",
            ".",
            "--json",
            "--file",
            "Assets/Player.mn",
            "--line",
            "1",
            "--col",
            "33",
        ])
        .current_dir(&root)
        .output()
        .unwrap();

    assert!(interface_output.status.success(), "{}", String::from_utf8_lossy(&interface_output.stderr));

    let interface_stdout = String::from_utf8(interface_output.stdout).unwrap();
    let interface_json: serde_json::Value = serde_json::from_str(&interface_stdout).unwrap();

    assert_eq!(interface_json["definition"]["qualified_name"], "InterfaceLike");
    assert_eq!(interface_json["definition"]["kind"], "type");
    assert_eq!(interface_json["definition"]["line"], 1);
    assert_eq!(interface_json["definition"]["col"], 7);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn definition_class_header_type_symbol_smoke() {
    let root = unique_temp_dir("moonc_definition_class_header_type_smoke");
    write_file(
        &root.join(".mnproject"),
        r#"[project]
name = "DefinitionProject"

[source]
include = ["Assets/**/*.mn"]
exclude = []
"#,
    );
    write_file(
        &root.join("Assets").join("BaseHelper.mn"),
        "class BaseHelper {}",
    );
    write_file(
        &root.join("Assets").join("InterfaceLike.mn"),
        "class InterfaceLike {}",
    );
    write_file(
        &root.join("Assets").join("Helper.mn"),
        "class Helper : BaseHelper, InterfaceLike {}",
    );

    let base_output = Command::new(moonc())
        .args([
            "definition",
            ".",
            "--json",
            "--file",
            "Assets/Helper.mn",
            "--line",
            "1",
            "--col",
            "18",
        ])
        .current_dir(&root)
        .output()
        .unwrap();

    assert!(base_output.status.success(), "{}", String::from_utf8_lossy(&base_output.stderr));

    let base_stdout = String::from_utf8(base_output.stdout).unwrap();
    let base_json: serde_json::Value = serde_json::from_str(&base_stdout).unwrap();

    assert_eq!(base_json["definition"]["qualified_name"], "BaseHelper");
    assert_eq!(base_json["definition"]["kind"], "type");
    assert_eq!(base_json["definition"]["line"], 1);
    assert_eq!(base_json["definition"]["col"], 7);

    let interface_output = Command::new(moonc())
        .args([
            "definition",
            ".",
            "--json",
            "--file",
            "Assets/Helper.mn",
            "--line",
            "1",
            "--col",
            "30",
        ])
        .current_dir(&root)
        .output()
        .unwrap();

    assert!(interface_output.status.success(), "{}", String::from_utf8_lossy(&interface_output.stderr));

    let interface_stdout = String::from_utf8(interface_output.stdout).unwrap();
    let interface_json: serde_json::Value = serde_json::from_str(&interface_stdout).unwrap();

    assert_eq!(interface_json["definition"]["qualified_name"], "InterfaceLike");
    assert_eq!(interface_json["definition"]["kind"], "type");
    assert_eq!(interface_json["definition"]["line"], 1);
    assert_eq!(interface_json["definition"]["col"], 7);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn definition_cross_file_member_smoke() {
    let root = unique_temp_dir("moonc_definition_cross_file_smoke");
    write_file(
        &root.join(".mnproject"),
        r#"[project]
name = "DefinitionProject"

[source]
include = ["Assets/**/*.mn"]
exclude = []
"#,
    );
    write_file(
        &root.join("Assets").join("WeaponData.mn"),
        r#"data class WeaponData(
    val damage: Int
)
"#,
    );
    write_file(
        &root.join("Assets").join("Player.mn"),
        r#"component Player : MonoBehaviour {
    func jump(weapon: WeaponData): Unit {
        val amount = weapon.damage
    }
}
"#,
    );

    let output = Command::new(moonc())
        .args([
            "definition",
            ".",
            "--json",
            "--file",
            "Assets/Player.mn",
            "--line",
            "3",
            "--col",
            "31",
        ])
        .current_dir(&root)
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(json["definition"]["qualified_name"], "WeaponData.damage");
    assert_eq!(json["definition"]["kind"], "field");
    assert_eq!(json["definition"]["line"], 2);
    assert_eq!(json["definition"]["col"], 9);
    assert!(json["definition"]["file"]
        .as_str()
        .unwrap()
        .ends_with("WeaponData.mn"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn references_local_symbol_smoke() {
    let root = unique_temp_dir("moonc_references_local_smoke");
    write_file(
        &root.join(".mnproject"),
        r#"[project]
name = "ReferencesProject"

[source]
include = ["Assets/**/*.mn"]
exclude = []
"#,
    );
    write_file(
        &root.join("Assets").join("Player.mn"),
        r#"component Player : MonoBehaviour {
    func jump(): Unit {
        val speed = 5
        val next = speed
    }
}
"#,
    );

    let output = Command::new(moonc())
        .args([
            "references",
            ".",
            "--json",
            "--file",
            "Assets/Player.mn",
            "--line",
            "3",
            "--col",
            "13",
        ])
        .current_dir(&root)
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let references = json["references"].as_array().unwrap();

    assert_eq!(json["definition"]["qualified_name"], "Player.jump.speed");
    assert_eq!(references.len(), 1);
    assert_eq!(references[0]["name"], "speed");
    assert_eq!(references[0]["kind"], "identifier");
    assert_eq!(references[0]["line"], 4);
    assert_eq!(references[0]["col"], 20);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn references_cross_file_member_smoke() {
    let root = unique_temp_dir("moonc_references_cross_file_member_smoke");
    write_file(
        &root.join(".mnproject"),
        r#"[project]
name = "ReferencesProject"

[source]
include = ["Assets/**/*.mn"]
exclude = []
"#,
    );
    write_file(
        &root.join("Assets").join("WeaponData.mn"),
        r#"data class WeaponData(
    val damage: Int
)
"#,
    );
    write_file(
        &root.join("Assets").join("Player.mn"),
        r#"component Player : MonoBehaviour {
    func jump(weapon: WeaponData): Unit {
        val amount = weapon.damage
    }
}
"#,
    );

    let output = Command::new(moonc())
        .args([
            "references",
            ".",
            "--json",
            "--file",
            "Assets/WeaponData.mn",
            "--line",
            "2",
            "--col",
            "9",
        ])
        .current_dir(&root)
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let references = json["references"].as_array().unwrap();

    assert_eq!(json["definition"]["qualified_name"], "WeaponData.damage");
    assert_eq!(references.len(), 1);
    assert_eq!(references[0]["name"], "damage");
    assert_eq!(references[0]["kind"], "member");
    assert_eq!(references[0]["candidate_qualified_name"], "WeaponData.damage");
    assert!(references[0]["file"].as_str().unwrap().ends_with("Player.mn"));
    assert_eq!(references[0]["line"], 3);
    assert_eq!(references[0]["col"], 29);

    let _ = fs::remove_dir_all(root);
}
