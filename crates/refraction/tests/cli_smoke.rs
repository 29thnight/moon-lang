use std::fs;
use std::io::{BufRead, BufReader, Write};
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

fn prism() -> &'static str {
    env!("CARGO_BIN_EXE_prism")
}

fn write_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

fn write_lsp_message(writer: &mut impl Write, value: &serde_json::Value) {
    let payload = serde_json::to_vec(value).unwrap();
    write!(writer, "Content-Length: {}\r\n\r\n", payload.len()).unwrap();
    writer.write_all(&payload).unwrap();
    writer.flush().unwrap();
}

fn read_lsp_message(reader: &mut impl BufRead) -> serde_json::Value {
    let mut content_length = None;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        if line.is_empty() {
            panic!("unexpected EOF while reading LSP headers");
        }
        if line == "\r\n" {
            break;
        }
        if let Some(value) = line.strip_prefix("Content-Length: ") {
            content_length = Some(value.trim().parse::<usize>().unwrap());
        }
    }

    let length = content_length.expect("missing LSP Content-Length header");
    let mut payload = vec![0u8; length];
    reader.read_exact(&mut payload).unwrap();
    serde_json::from_slice(&payload).unwrap()
}

fn read_lsp_response(reader: &mut impl BufRead, id: i64) -> serde_json::Value {
    loop {
        let message = read_lsp_message(reader);
        if message.get("id").and_then(|value| value.as_i64()) == Some(id) {
            return message;
        }
    }
}

fn read_lsp_notification(reader: &mut impl BufRead, method: &str) -> serde_json::Value {
    loop {
        let message = read_lsp_message(reader);
        if message.get("method").and_then(|value| value.as_str()) == Some(method) {
            return message;
        }
    }
}

#[test]
fn compile_json_smoke() {
    let root = unique_temp_dir("prism_compile_smoke");
    let source = root.join("Hello.prsm");
    write_file(&source, "component Hello : MonoBehaviour {}");

    let output = Command::new(prism())
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
    assert!(root.join("Hello.prsmmap.json").exists());
    assert_eq!(json["outputs"][0]["source_map"], root.join("Hello.prsmmap.json").to_string_lossy().to_string());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn check_json_reports_errors() {
    let root = unique_temp_dir("prism_check_smoke");
    let source = root.join("Broken.prsm");
    write_file(&source, "enum Broken {}");

    let output = Command::new(prism())
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
    let root = unique_temp_dir("prism_build_smoke");
    write_file(
        &root.join(".prsmproject"),
        r#"[project]
name = "SmokeProject"
prsm_version = "0.1.0"

[language]
version = "1.0"
features = ["input-system"]

[compiler]
prism_path = "prism"
output_dir = "Generated/PrSM"

[source]
include = ["Assets/**/*.prsm"]
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
        &root.join("Assets").join("BuildTarget.prsm"),
        "component BuildTarget : MonoBehaviour {}",
    );

    let output = Command::new(prism())
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
    assert!(root.join("Generated").join("PrSM").join("BuildTarget.cs").exists());
    assert!(root.join("Generated").join("PrSM").join("BuildTarget.prsmmap.json").exists());
    assert!(root.join(".prsm").join("cache").exists());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn build_project_listen_and_intrinsic_smoke() {
    let root = unique_temp_dir("prism_build_listen_intrinsic_smoke");
    write_file(
        &root.join(".prsmproject"),
        r#"[project]
name = "SyntaxProject"

[compiler]
output_dir = "Generated/PrSM"

[source]
include = ["Assets/**/*.prsm"]
exclude = []
"#,
    );
    write_file(
        &root.join("Assets").join("UiController.prsm"),
        r#"using UnityEngine.UI

component UiController : MonoBehaviour {
    serialize button: Button

    start {
        listen button.onClick {
            nativeLog("clicked")
        }
    }

    func rawBridge(): Unit {
        intrinsic {
            Debug.Log("raw");
        }
    }

    intrinsic func nativeLog(message: String) {
        Debug.Log(message);
    }

    intrinsic coroutine waitNative() {
        yield return null;
    }
}
"#,
    );

    let output = Command::new(prism())
        .args(["build", "--json"])
        .current_dir(&root)
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json_start = stdout.find('{').expect("expected JSON output");
    let json: serde_json::Value = serde_json::from_str(&stdout[json_start..]).unwrap();

    assert_eq!(json["project"], "SyntaxProject");
    assert_eq!(json["files"], 1);
    assert_eq!(json["compiled"], 1);
    assert_eq!(json["errors"], 0);

    let generated_path = root.join("Generated").join("PrSM").join("UiController.cs");
    let generated_source = fs::read_to_string(&generated_path).unwrap();

    assert!(generated_source.contains("button.onClick.AddListener(() =>"));
    assert!(generated_source.contains("nativeLog(\"clicked\");"));
    assert!(generated_source.contains("Debug.Log(\"raw\");"));
    assert!(generated_source.contains("public void nativeLog(string message)"));
    assert!(generated_source.contains("Debug.Log(message);"));
    assert!(generated_source.contains("private System.Collections.IEnumerator waitNative()"));
    assert!(generated_source.contains("yield return null;"));
    assert!(root.join("Generated").join("PrSM").join("UiController.prsmmap.json").exists());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn build_project_incremental_cache_skips_unchanged_files() {
    let root = unique_temp_dir("prism_build_incremental_cache_smoke");
    write_file(
        &root.join(".prsmproject"),
        r#"[project]
name = "IncrementalProject"

[compiler]
output_dir = "Generated/PrSM"

[source]
include = ["Assets/**/*.prsm"]
exclude = []
"#,
    );
    write_file(
        &root.join("Assets").join("Player.prsm"),
        r#"component Player : MonoBehaviour {
    func ping(): Unit {}
}
"#,
    );
    write_file(
        &root.join("Assets").join("Enemy.prsm"),
        r#"component Enemy : MonoBehaviour {
    func pong(): Unit {}
}
"#,
    );

    let first = Command::new(prism())
        .args(["build", "--json"])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(first.status.success(), "{}", String::from_utf8_lossy(&first.stderr));

    let first_stdout = String::from_utf8(first.stdout).unwrap();
    let first_json_start = first_stdout.find('{').expect("expected JSON output");
    let first_json: serde_json::Value = serde_json::from_str(&first_stdout[first_json_start..]).unwrap();
    assert_eq!(first_json["files"], 2);
    assert_eq!(first_json["compiled"], 2);
    assert_eq!(first_json["cached"], 0);
    assert!(root.join(".prsm").join("cache").join("build-manifest.json").exists());

    let second = Command::new(prism())
        .args(["build", "--json"])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(second.status.success(), "{}", String::from_utf8_lossy(&second.stderr));

    let second_stdout = String::from_utf8(second.stdout).unwrap();
    let second_json_start = second_stdout.find('{').expect("expected JSON output");
    let second_json: serde_json::Value = serde_json::from_str(&second_stdout[second_json_start..]).unwrap();
    assert_eq!(second_json["compiled"], 0);
    assert_eq!(second_json["cached"], 2);
    assert!(
        second_json["outputs"]
            .as_array()
            .unwrap()
            .iter()
            .all(|entry| entry["cached"] == serde_json::Value::Bool(true)),
        "expected all outputs to be cached on second build: {second_stdout}"
    );

    write_file(
        &root.join("Assets").join("Enemy.prsm"),
        r#"component Enemy : MonoBehaviour {
    func pong(): Unit {
        log("changed")
    }
}
"#,
    );

    let third = Command::new(prism())
        .args(["build", "--json"])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(third.status.success(), "{}", String::from_utf8_lossy(&third.stderr));

    let third_stdout = String::from_utf8(third.stdout).unwrap();
    let third_json_start = third_stdout.find('{').expect("expected JSON output");
    let third_json: serde_json::Value = serde_json::from_str(&third_stdout[third_json_start..]).unwrap();
    assert_eq!(third_json["compiled"], 1);
    assert_eq!(third_json["cached"], 1);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn compile_source_map_sidecar_contains_member_anchors() {
    let root = unique_temp_dir("prism_source_map_sidecar_smoke");
    let source = root.join("Player.prsm");
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

    let output = Command::new(prism())
        .args(["compile", source.to_str().unwrap(), "--json"])
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));

    let source_map_path = root.join("Player.prsmmap.json");
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
fn compile_source_map_sidecar_tracks_listen_and_intrinsic_statement_segments() {
    let root = unique_temp_dir("prism_source_map_listen_intrinsic_segments_smoke");
    let source = root.join("UiController.prsm");
    write_file(
        &source,
        r#"using UnityEngine.UI

component UiController : MonoBehaviour {
    serialize button: Button

    start {
        listen button.onClick {
            nativeLog("clicked")
        }
    }

    func rawBridge(): Unit {
        intrinsic {
            Debug.Log("raw");
        }
    }

    intrinsic func nativeLog(message: String) {
        Debug.Log(message);
    }
}
"#,
    );

    let output = Command::new(prism())
        .args(["compile", source.to_str().unwrap(), "--json"])
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));

    let source_map_path = root.join("UiController.prsmmap.json");
    let source_map_text = fs::read_to_string(&source_map_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&source_map_text).unwrap();
    let members = json["members"].as_array().unwrap();

    let start_member = members
        .iter()
        .find(|member| member["qualified_name"] == "UiController.start")
        .expect("expected start lifecycle anchor");
    let start_segments = start_member["segments"].as_array().unwrap();
    assert_eq!(start_segments.len(), 1);
    assert_eq!(start_segments[0]["source_span"]["line"], 7);
    assert!(start_segments[0]["generated_span"]["end_line"].as_u64().unwrap()
        > start_segments[0]["generated_span"]["line"].as_u64().unwrap());

    let raw_member = members
        .iter()
        .find(|member| member["qualified_name"] == "UiController.rawBridge")
        .expect("expected rawBridge function anchor");
    let raw_segments = raw_member["segments"].as_array().unwrap();
    assert_eq!(raw_segments.len(), 1);
    assert_eq!(raw_segments[0]["source_span"]["line"], 13);
    assert!(raw_segments[0]["generated_span"]["line"].as_u64().is_some());
    assert!(raw_segments[0]["generated_span"]["end_line"].as_u64().unwrap()
        >= raw_segments[0]["generated_span"]["line"].as_u64().unwrap());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn build_project_rejects_unknown_language_version() {
    let root = unique_temp_dir("prism_build_invalid_language_version");
    write_file(
        &root.join(".prsmproject"),
        r#"[project]
name = "BrokenProject"

[language]
version = "3.0"
features = []

[source]
include = ["Assets/**/*.prsm"]
exclude = []
"#,
    );
    write_file(
        &root.join("Assets").join("BuildTarget.prsm"),
        "component BuildTarget : MonoBehaviour {}",
    );

    let output = Command::new(prism())
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
    let root = unique_temp_dir("prism_index_smoke");
    write_file(
        &root.join(".prsmproject"),
        r#"[project]
name = "IndexProject"

[language]
version = "2.0"
features = ["pattern-bindings"]

[source]
include = ["Assets/**/*.prsm"]
exclude = []
"#,
    );
    write_file(
        &root.join("Assets").join("Player.prsm"),
        r#"component Player : MonoBehaviour {
    func jump(): Unit {
        return
    }
}
"#,
    );

    let output = Command::new(prism())
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
    let root = unique_temp_dir("prism_index_filter_smoke");
    write_file(
        &root.join(".prsmproject"),
        r#"[project]
name = "IndexProject"

[source]
include = ["Assets/**/*.prsm"]
exclude = []
"#,
    );
    write_file(
        &root.join("Assets").join("Player.prsm"),
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

    let output = Command::new(prism())
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
    let root = unique_temp_dir("prism_index_position_smoke");
    write_file(
        &root.join(".prsmproject"),
        r#"[project]
name = "IndexProject"

[source]
include = ["Assets/**/*.prsm"]
exclude = []
"#,
    );
    write_file(
        &root.join("Assets").join("Player.prsm"),
        r#"component Player : MonoBehaviour {
    func jump(): Unit {
        return
    }
}
"#,
    );

    let output = Command::new(prism())
        .args([
            "index",
            ".",
            "--json",
            "--file",
            "Assets/Player.prsm",
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
    let root = unique_temp_dir("prism_index_top_level_precision_smoke");
    write_file(
        &root.join(".prsmproject"),
        r#"[project]
name = "IndexProject"

[source]
include = ["Assets/**/*.prsm"]
exclude = []
"#,
    );
    write_file(
        &root.join("Assets").join("Player.prsm"),
        "component Player : MonoBehaviour {}",
    );

    let name_output = Command::new(prism())
        .args([
            "index",
            ".",
            "--json",
            "--file",
            "Assets/Player.prsm",
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

    let base_output = Command::new(prism())
        .args([
            "index",
            ".",
            "--json",
            "--file",
            "Assets/Player.prsm",
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
    let root = unique_temp_dir("prism_index_header_type_reference_smoke");
    write_file(
        &root.join(".prsmproject"),
        r#"[project]
name = "IndexProject"

[source]
include = ["Assets/**/*.prsm"]
exclude = []
"#,
    );
    write_file(
        &root.join("Assets").join("BaseActor.prsm"),
        "class BaseActor {}",
    );
    write_file(
        &root.join("Assets").join("InterfaceLike.prsm"),
        "class InterfaceLike {}",
    );
    write_file(
        &root.join("Assets").join("Player.prsm"),
        "component Player : BaseActor, InterfaceLike {}",
    );

    let base_output = Command::new(prism())
        .args([
            "index",
            ".",
            "--json",
            "--file",
            "Assets/Player.prsm",
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

    let interface_output = Command::new(prism())
        .args([
            "index",
            ".",
            "--json",
            "--file",
            "Assets/Player.prsm",
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
    let root = unique_temp_dir("prism_index_member_type_reference_smoke");
    write_file(
        &root.join(".prsmproject"),
        r#"[project]
name = "IndexProject"

[source]
include = ["Assets/**/*.prsm"]
exclude = []
"#,
    );
    write_file(
        &root.join("Assets").join("WeaponData.prsm"),
        "data class WeaponData(val damage: Int)",
    );
    write_file(
        &root.join("Assets").join("Player.prsm"),
        r#"component Player : MonoBehaviour {
    serialize var equipped: WeaponData

    func equip(next: WeaponData): WeaponData {
        val backup: List<WeaponData> = next
        return next
    }
}
"#,
    );

    let field_output = Command::new(prism())
        .args([
            "index",
            ".",
            "--json",
            "--file",
            "Assets/Player.prsm",
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

    let return_output = Command::new(prism())
        .args([
            "index",
            ".",
            "--json",
            "--file",
            "Assets/Player.prsm",
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

    let local_output = Command::new(prism())
        .args([
            "index",
            ".",
            "--json",
            "--file",
            "Assets/Player.prsm",
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
    let root = unique_temp_dir("prism_hir_smoke");
    write_file(
        &root.join(".prsmproject"),
        r#"[project]
name = "HirProject"

[source]
include = ["Assets/**/*.prsm"]
exclude = []
"#,
    );
    write_file(
        &root.join("Assets").join("Player.prsm"),
        r#"component Player : MonoBehaviour {
    func jump(): Unit {
        val speed = 5
        val next = speed
    }
}
"#,
    );

    let output = Command::new(prism())
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
    let root = unique_temp_dir("prism_definition_smoke");
    write_file(
        &root.join(".prsmproject"),
        r#"[project]
name = "DefinitionProject"

[source]
include = ["Assets/**/*.prsm"]
exclude = []
"#,
    );
    write_file(
        &root.join("Assets").join("Player.prsm"),
        r#"component Player : MonoBehaviour {
    func jump(): Unit {
        val speed = 5
        val next = speed
    }
}
"#,
    );

    let output = Command::new(prism())
        .args([
            "definition",
            ".",
            "--json",
            "--file",
            "Assets/Player.prsm",
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
    let root = unique_temp_dir("prism_definition_parameter_smoke");
    write_file(
        &root.join(".prsmproject"),
        r#"[project]
name = "DefinitionProject"

[source]
include = ["Assets/**/*.prsm"]
exclude = []
"#,
    );
    write_file(
        &root.join("Assets").join("Player.prsm"),
        r#"component Player : MonoBehaviour {
    func jump(weapon: WeaponData): Unit {
        val amount = weapon
    }
}
"#,
    );

    let output = Command::new(prism())
        .args([
            "definition",
            ".",
            "--json",
            "--file",
            "Assets/Player.prsm",
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
    let root = unique_temp_dir("prism_definition_parameter_type_smoke");
    write_file(
        &root.join(".prsmproject"),
        r#"[project]
name = "DefinitionProject"

[source]
include = ["Assets/**/*.prsm"]
exclude = []
"#,
    );
    write_file(
        &root.join("Assets").join("WeaponData.prsm"),
        r#"data class WeaponData(
    val damage: Int
)
"#,
    );
    write_file(
        &root.join("Assets").join("Player.prsm"),
        r#"component Player : MonoBehaviour {
    func jump(weapon: WeaponData): Unit {
        val amount = weapon
    }
}
"#,
    );

    let output = Command::new(prism())
        .args([
            "definition",
            ".",
            "--json",
            "--file",
            "Assets/Player.prsm",
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
    let root = unique_temp_dir("prism_definition_generic_type_arg_smoke");
    write_file(
        &root.join(".prsmproject"),
        r#"[project]
name = "DefinitionProject"

[source]
include = ["Assets/**/*.prsm"]
exclude = []
"#,
    );
    write_file(
        &root.join("Assets").join("WeaponData.prsm"),
        r#"data class WeaponData(
    val damage: Int
)
"#,
    );
    write_file(
        &root.join("Assets").join("Player.prsm"),
        r#"component Player : MonoBehaviour {
    func jump(weapons: List<WeaponData>): Unit {
        return
    }
}
"#,
    );

    let output = Command::new(prism())
        .args([
            "definition",
            ".",
            "--json",
            "--file",
            "Assets/Player.prsm",
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
    let root = unique_temp_dir("prism_definition_component_header_type_smoke");
    write_file(
        &root.join(".prsmproject"),
        r#"[project]
name = "DefinitionProject"

[source]
include = ["Assets/**/*.prsm"]
exclude = []
"#,
    );
    write_file(
        &root.join("Assets").join("BaseActor.prsm"),
        "class BaseActor {}",
    );
    write_file(
        &root.join("Assets").join("InterfaceLike.prsm"),
        "class InterfaceLike {}",
    );
    write_file(
        &root.join("Assets").join("Player.prsm"),
        "component Player : BaseActor, InterfaceLike {}",
    );

    let base_output = Command::new(prism())
        .args([
            "definition",
            ".",
            "--json",
            "--file",
            "Assets/Player.prsm",
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

    let interface_output = Command::new(prism())
        .args([
            "definition",
            ".",
            "--json",
            "--file",
            "Assets/Player.prsm",
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
    let root = unique_temp_dir("prism_definition_class_header_type_smoke");
    write_file(
        &root.join(".prsmproject"),
        r#"[project]
name = "DefinitionProject"

[source]
include = ["Assets/**/*.prsm"]
exclude = []
"#,
    );
    write_file(
        &root.join("Assets").join("BaseHelper.prsm"),
        "class BaseHelper {}",
    );
    write_file(
        &root.join("Assets").join("InterfaceLike.prsm"),
        "class InterfaceLike {}",
    );
    write_file(
        &root.join("Assets").join("Helper.prsm"),
        "class Helper : BaseHelper, InterfaceLike {}",
    );

    let base_output = Command::new(prism())
        .args([
            "definition",
            ".",
            "--json",
            "--file",
            "Assets/Helper.prsm",
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

    let interface_output = Command::new(prism())
        .args([
            "definition",
            ".",
            "--json",
            "--file",
            "Assets/Helper.prsm",
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
    let root = unique_temp_dir("prism_definition_cross_file_smoke");
    write_file(
        &root.join(".prsmproject"),
        r#"[project]
name = "DefinitionProject"

[source]
include = ["Assets/**/*.prsm"]
exclude = []
"#,
    );
    write_file(
        &root.join("Assets").join("WeaponData.prsm"),
        r#"data class WeaponData(
    val damage: Int
)
"#,
    );
    write_file(
        &root.join("Assets").join("Player.prsm"),
        r#"component Player : MonoBehaviour {
    func jump(weapon: WeaponData): Unit {
        val amount = weapon.damage
    }
}
"#,
    );

    let output = Command::new(prism())
        .args([
            "definition",
            ".",
            "--json",
            "--file",
            "Assets/Player.prsm",
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
        .ends_with("WeaponData.prsm"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn references_local_symbol_smoke() {
    let root = unique_temp_dir("prism_references_local_smoke");
    write_file(
        &root.join(".prsmproject"),
        r#"[project]
name = "ReferencesProject"

[source]
include = ["Assets/**/*.prsm"]
exclude = []
"#,
    );
    write_file(
        &root.join("Assets").join("Player.prsm"),
        r#"component Player : MonoBehaviour {
    func jump(): Unit {
        val speed = 5
        val next = speed
    }
}
"#,
    );

    let output = Command::new(prism())
        .args([
            "references",
            ".",
            "--json",
            "--file",
            "Assets/Player.prsm",
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
fn lsp_definition_smoke() {
    let root = unique_temp_dir("prism_lsp_definition_smoke");
    write_file(
        &root.join(".prsmproject"),
        r#"[project]
name = "LspProject"

[compiler]
output_dir = "Generated/PrSM"

[source]
include = ["Assets/**/*.prsm"]
exclude = []
"#,
    );
    write_file(
        &root.join("Assets").join("WeaponData.prsm"),
        r#"data class WeaponData(
    val damage: Int
)
"#,
    );
    write_file(
        &root.join("Assets").join("Player.prsm"),
        r#"component Player : MonoBehaviour {
    func jump(weapon: WeaponData): Unit {
        val amount = weapon
    }
}
"#,
    );

    let mut child = Command::new(prism())
        .arg("lsp")
        .current_dir(&root)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    let root_uri = url::Url::from_file_path(&root).unwrap().to_string();
    let player_path = root.join("Assets").join("Player.prsm");
    let weapon_path = root.join("Assets").join("WeaponData.prsm");
    let player_uri = url::Url::from_file_path(&player_path).unwrap().to_string();

    {
        let stdin = child.stdin.as_mut().unwrap();
        write_lsp_message(
            stdin,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "processId": null,
                    "rootUri": root_uri,
                    "capabilities": {}
                }
            }),
        );
    }

    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let initialize_response = read_lsp_response(&mut stdout, 1);
    assert!(initialize_response.get("result").is_some(), "{initialize_response:?}");

    {
        let stdin = child.stdin.as_mut().unwrap();
        write_lsp_message(
            stdin,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "method": "initialized",
                "params": {}
            }),
        );
        write_lsp_message(
            stdin,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "textDocument/definition",
                "params": {
                    "textDocument": { "uri": player_uri },
                    "position": { "line": 1, "character": 24 }
                }
            }),
        );
    }

    let definition_response = read_lsp_response(&mut stdout, 2);
    let result = definition_response.get("result").expect("definition result");
    assert_eq!(result["uri"], url::Url::from_file_path(&weapon_path).unwrap().to_string());
    assert_eq!(result["range"]["start"]["line"], 0);
    assert_eq!(result["range"]["start"]["character"], 11);

    {
        let stdin = child.stdin.as_mut().unwrap();
        write_lsp_message(
            stdin,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "shutdown",
                "params": null
            }),
        );
    }
    let shutdown_response = read_lsp_response(&mut stdout, 3);
    assert!(shutdown_response.get("result").is_some(), "{shutdown_response:?}");

    {
        let stdin = child.stdin.as_mut().unwrap();
        write_lsp_message(
            stdin,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "method": "exit",
                "params": null
            }),
        );
    }

    let status = child.wait().unwrap();
    assert!(status.success(), "LSP child exited with status {status}");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn lsp_publish_diagnostics_smoke() {
    let root = unique_temp_dir("prism_lsp_diagnostics_smoke");
    write_file(
        &root.join(".prsmproject"),
        r#"[project]
name = "LspProject"

[compiler]
output_dir = "Generated/PrSM"

[source]
include = ["Assets/**/*.prsm"]
exclude = []
"#,
    );
    let broken_path = root.join("Assets").join("Broken.prsm");
    write_file(&broken_path, "enum Broken {}");

    let mut child = Command::new(prism())
        .arg("lsp")
        .current_dir(&root)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    let root_uri = url::Url::from_file_path(&root).unwrap().to_string();
    let broken_uri = url::Url::from_file_path(&broken_path).unwrap().to_string();
    let broken_text = fs::read_to_string(&broken_path).unwrap();

    {
        let stdin = child.stdin.as_mut().unwrap();
        write_lsp_message(
            stdin,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "processId": null,
                    "rootUri": root_uri,
                    "capabilities": {}
                }
            }),
        );
    }

    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let initialize_response = read_lsp_response(&mut stdout, 1);
    assert!(initialize_response.get("result").is_some(), "{initialize_response:?}");

    {
        let stdin = child.stdin.as_mut().unwrap();
        write_lsp_message(
            stdin,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "method": "initialized",
                "params": {}
            }),
        );
        write_lsp_message(
            stdin,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {
                    "textDocument": {
                        "uri": broken_uri,
                        "languageId": "prsm",
                        "version": 1,
                        "text": broken_text
                    }
                }
            }),
        );
    }

    let diagnostics_notification = read_lsp_notification(&mut stdout, "textDocument/publishDiagnostics");
    assert_eq!(diagnostics_notification["params"]["uri"], url::Url::from_file_path(&broken_path).unwrap().to_string());
    assert_eq!(diagnostics_notification["params"]["version"], 1);
    assert!(diagnostics_notification["params"]["diagnostics"].as_array().unwrap().len() >= 1);
    assert_eq!(diagnostics_notification["params"]["diagnostics"][0]["code"], "E050");

    {
        let stdin = child.stdin.as_mut().unwrap();
        write_lsp_message(
            stdin,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "shutdown",
                "params": null
            }),
        );
    }
    let shutdown_response = read_lsp_response(&mut stdout, 2);
    assert!(shutdown_response.get("result").is_some(), "{shutdown_response:?}");

    {
        let stdin = child.stdin.as_mut().unwrap();
        write_lsp_message(
            stdin,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "method": "exit",
                "params": null
            }),
        );
    }

    let status = child.wait().unwrap();
    assert!(status.success(), "LSP child exited with status {status}");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn lsp_member_completion_smoke() {
    let root = unique_temp_dir("prism_lsp_completion_smoke");
    write_file(
        &root.join(".prsmproject"),
        r#"[project]
name = "LspProject"

[compiler]
output_dir = "Generated/PrSM"

[source]
include = ["Assets/**/*.prsm"]
exclude = []
"#,
    );
    write_file(
        &root.join("Assets").join("WeaponData.prsm"),
        r#"data class WeaponData(
    val damage: Int
)
"#,
    );
    let player_path = root.join("Assets").join("Player.prsm");
    write_file(
        &player_path,
        r#"component Player : MonoBehaviour {
    func jump(weapon: WeaponData): Unit {
        weapon.
    }
}
"#,
    );

    let mut child = Command::new(prism())
        .arg("lsp")
        .current_dir(&root)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    let root_uri = url::Url::from_file_path(&root).unwrap().to_string();
    let player_uri = url::Url::from_file_path(&player_path).unwrap().to_string();

    {
        let stdin = child.stdin.as_mut().unwrap();
        write_lsp_message(
            stdin,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "processId": null,
                    "rootUri": root_uri,
                    "capabilities": {}
                }
            }),
        );
    }

    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let initialize_response = read_lsp_response(&mut stdout, 1);
    assert!(initialize_response.get("result").is_some(), "{initialize_response:?}");

    {
        let stdin = child.stdin.as_mut().unwrap();
        write_lsp_message(
            stdin,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "method": "initialized",
                "params": {}
            }),
        );
        write_lsp_message(
            stdin,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "textDocument/completion",
                "params": {
                    "textDocument": { "uri": player_uri },
                    "position": { "line": 2, "character": 15 }
                }
            }),
        );
    }

    let completion_response = read_lsp_response(&mut stdout, 2);
    let items = completion_response["result"]["items"].as_array().unwrap();
    let damage_item = items
        .iter()
        .find(|item| item["label"] == "damage")
        .expect("expected WeaponData.damage completion item");
    assert_eq!(damage_item["detail"], "damage: Int");

    {
        let stdin = child.stdin.as_mut().unwrap();
        write_lsp_message(
            stdin,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "shutdown",
                "params": null
            }),
        );
    }
    let shutdown_response = read_lsp_response(&mut stdout, 3);
    assert!(shutdown_response.get("result").is_some(), "{shutdown_response:?}");

    {
        let stdin = child.stdin.as_mut().unwrap();
        write_lsp_message(
            stdin,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "method": "exit",
                "params": null
            }),
        );
    }

    let status = child.wait().unwrap();
    assert!(status.success(), "LSP child exited with status {status}");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn lsp_core_unity_completion_smoke() {
    let root = unique_temp_dir("prism_lsp_core_unity_completion_smoke");
    write_file(
        &root.join(".prsmproject"),
        r#"[project]
name = "LspProject"

[compiler]
output_dir = "Generated/PrSM"

[source]
include = ["Assets/**/*.prsm"]
exclude = []
"#,
    );
    let player_path = root.join("Assets").join("Player.prsm");
    write_file(
        &player_path,
        r#"component Player : MonoBehaviour {
    serialize camera: Camera
    serialize button: Button

    func wire(): Unit {
        camera.
        button.onClick.
    }
}
"#,
    );

    let mut child = Command::new(prism())
        .arg("lsp")
        .current_dir(&root)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    let root_uri = url::Url::from_file_path(&root).unwrap().to_string();
    let player_uri = url::Url::from_file_path(&player_path).unwrap().to_string();

    {
        let stdin = child.stdin.as_mut().unwrap();
        write_lsp_message(
            stdin,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "processId": null,
                    "rootUri": root_uri,
                    "capabilities": {}
                }
            }),
        );
    }

    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let initialize_response = read_lsp_response(&mut stdout, 1);
    assert!(initialize_response.get("result").is_some(), "{initialize_response:?}");

    {
        let stdin = child.stdin.as_mut().unwrap();
        write_lsp_message(
            stdin,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "method": "initialized",
                "params": {}
            }),
        );
        write_lsp_message(
            stdin,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "textDocument/completion",
                "params": {
                    "textDocument": { "uri": player_uri },
                    "position": { "line": 5, "character": 15 }
                }
            }),
        );
        write_lsp_message(
            stdin,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "textDocument/completion",
                "params": {
                    "textDocument": { "uri": player_uri },
                    "position": { "line": 6, "character": 23 }
                }
            }),
        );
    }

    let camera_completion = read_lsp_response(&mut stdout, 2);
    let camera_items = camera_completion["result"]["items"].as_array().unwrap();
    let field_of_view = camera_items
        .iter()
        .find(|item| item["label"] == "fieldOfView")
        .expect("expected Camera.fieldOfView completion item");
    assert_eq!(field_of_view["detail"], "fieldOfView: Float");

    let event_completion = read_lsp_response(&mut stdout, 3);
    let event_items = event_completion["result"]["items"].as_array().unwrap();
    let add_listener = event_items
        .iter()
        .find(|item| item["label"] == "addListener")
        .expect("expected UnityEvent.addListener completion item");
    assert_eq!(add_listener["detail"], "addListener(callback: Any): Unit");

    {
        let stdin = child.stdin.as_mut().unwrap();
        write_lsp_message(
            stdin,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "shutdown",
                "params": null
            }),
        );
    }
    let shutdown_response = read_lsp_response(&mut stdout, 4);
    assert!(shutdown_response.get("result").is_some(), "{shutdown_response:?}");

    {
        let stdin = child.stdin.as_mut().unwrap();
        write_lsp_message(
            stdin,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "method": "exit",
                "params": null
            }),
        );
    }

    let status = child.wait().unwrap();
    assert!(status.success(), "LSP child exited with status {status}");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn lsp_hover_and_document_symbols_smoke() {
    let root = unique_temp_dir("prism_lsp_hover_symbols_smoke");
    write_file(
        &root.join(".prsmproject"),
        r#"[project]
name = "LspProject"

[compiler]
output_dir = "Generated/PrSM"

[source]
include = ["Assets/**/*.prsm"]
exclude = []
"#,
    );
    let player_path = root.join("Assets").join("Player.prsm");
    write_file(
        &player_path,
        r#"component Player : MonoBehaviour {
    require rb: Rigidbody

    func jump(): Unit {
    }
}
"#,
    );
    write_file(
        &root.join("Generated").join("PrSM").join("Player.cs"),
        "public class Player {}\n",
    );

    let mut child = Command::new(prism())
        .arg("lsp")
        .current_dir(&root)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    let root_uri = url::Url::from_file_path(&root).unwrap().to_string();
    let player_uri = url::Url::from_file_path(&player_path).unwrap().to_string();

    {
        let stdin = child.stdin.as_mut().unwrap();
        write_lsp_message(
            stdin,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "processId": null,
                    "rootUri": root_uri,
                    "capabilities": {}
                }
            }),
        );
    }

    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let initialize_response = read_lsp_response(&mut stdout, 1);
    assert!(initialize_response.get("result").is_some(), "{initialize_response:?}");

    {
        let stdin = child.stdin.as_mut().unwrap();
        write_lsp_message(
            stdin,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "method": "initialized",
                "params": {}
            }),
        );
        write_lsp_message(
            stdin,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "textDocument/hover",
                "params": {
                    "textDocument": { "uri": player_uri },
                    "position": { "line": 0, "character": 10 }
                }
            }),
        );
        write_lsp_message(
            stdin,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "textDocument/documentSymbol",
                "params": {
                    "textDocument": { "uri": player_uri }
                }
            }),
        );
    }

    let hover_response = read_lsp_response(&mut stdout, 2);
    let hover_markdown = hover_response["result"]["contents"]["value"]
        .as_str()
        .expect("expected hover markdown");
    assert!(hover_markdown.contains("```prsm\ncomponent Player : MonoBehaviour\n```"));
    assert!(hover_markdown.contains("**[Unity API]**"));
    assert!(hover_markdown.contains("UnityEngine.MonoBehaviour"));
    assert!(hover_markdown.contains("MonoBehaviour.html"));
    assert!(!hover_markdown.contains("**Status:**"));
    assert!(!hover_markdown.contains("**Definition:**"));
    assert!(!hover_markdown.contains("**Lookup:**"));
    assert!(!hover_markdown.contains("**File:**"));

    let symbol_response = read_lsp_response(&mut stdout, 3);
    let symbols = symbol_response["result"].as_array().unwrap();
    assert_eq!(symbols.len(), 1);
    assert_eq!(symbols[0]["name"], "Player");
    let children = symbols[0]["children"].as_array().unwrap();
    let child_names = children
        .iter()
        .map(|child| child["name"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert!(child_names.contains(&"rb"));
    assert!(child_names.contains(&"jump"));

    {
        let stdin = child.stdin.as_mut().unwrap();
        write_lsp_message(
            stdin,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "textDocument/hover",
                "params": {
                    "textDocument": { "uri": player_uri },
                    "position": { "line": 0, "character": 24 }
                }
            }),
        );
    }

    let mono_hover_response = read_lsp_response(&mut stdout, 4);
    let mono_hover_markdown = mono_hover_response["result"]["contents"]["value"]
        .as_str()
        .expect("expected MonoBehaviour hover markdown");
    assert!(mono_hover_markdown.contains("```prsm\nMonoBehaviour\n```"));
    assert!(mono_hover_markdown.contains("**[Unity API]**"));
    assert!(mono_hover_markdown.contains("UnityEngine.MonoBehaviour"));
    assert!(mono_hover_markdown.contains("MonoBehaviour.html"));

    {
        let stdin = child.stdin.as_mut().unwrap();
        write_lsp_message(
            stdin,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 5,
                "method": "shutdown",
                "params": null
            }),
        );
    }
    let shutdown_response = read_lsp_response(&mut stdout, 5);
    assert!(shutdown_response.get("result").is_some(), "{shutdown_response:?}");

    {
        let stdin = child.stdin.as_mut().unwrap();
        write_lsp_message(
            stdin,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "method": "exit",
                "params": null
            }),
        );
    }

    let status = child.wait().unwrap();
    assert!(status.success(), "LSP child exited with status {status}");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn references_cross_file_member_smoke() {
    let root = unique_temp_dir("prism_references_cross_file_member_smoke");
    write_file(
        &root.join(".prsmproject"),
        r#"[project]
name = "ReferencesProject"

[source]
include = ["Assets/**/*.prsm"]
exclude = []
"#,
    );
    write_file(
        &root.join("Assets").join("WeaponData.prsm"),
        r#"data class WeaponData(
    val damage: Int
)
"#,
    );
    write_file(
        &root.join("Assets").join("Player.prsm"),
        r#"component Player : MonoBehaviour {
    func jump(weapon: WeaponData): Unit {
        val amount = weapon.damage
    }
}
"#,
    );

    let output = Command::new(prism())
        .args([
            "references",
            ".",
            "--json",
            "--file",
            "Assets/WeaponData.prsm",
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
    assert!(references[0]["file"].as_str().unwrap().ends_with("Player.prsm"));
    assert_eq!(references[0]["line"], 3);
    assert_eq!(references[0]["col"], 29);

    let _ = fs::remove_dir_all(root);
}

// ── Multi-file end-to-end build ──────────────────────────────────────────────
//
// Builds a realistic mini-game project (4 files, all major language features)
// and validates generated C# output for every major lowering path.

#[test]
fn build_multifile_game_project_end_to_end() {
    let root = unique_temp_dir("prism_e2e_game");
    write_file(
        &root.join(".prsmproject"),
        r#"[project]
name = "E2EGame"
prsm_version = "0.1.0"

[compiler]
output_dir = "Generated/PrSM"

[source]
include = ["Assets/**/*.prsm"]
exclude = []
"#,
    );

    // 1) Asset class — ScriptableObject lowering, serialize, Bool func
    write_file(
        &root.join("Assets").join("ItemData.prsm"),
        r#"using UnityEngine

asset ItemData : ScriptableObject {
    serialize itemName: String = "Item"
    serialize value: Int = 10
    serialize weight: Float = 1.5
    serialize icon: Sprite?

    func isHeavy(): Bool = weight > 5.0
}
"#,
    );

    // 2) Component with coroutine, wait, start call
    write_file(
        &root.join("Assets").join("ActorHealth.prsm"),
        r#"using UnityEngine

component ActorHealth : MonoBehaviour {
    serialize maxHp: Int = 100
    serialize invincibleTime: Float = 1.0

    var hp: Int = 100
    var invincible: Bool = false

    func takeDamage(amount: Int) {
        if invincible { return }
        hp -= amount
        start hitCooldown()
        if hp <= 0 {
            gameObject.setActive(false)
        }
    }

    coroutine hitCooldown() {
        invincible = true
        wait invincibleTime.s
        invincible = false
    }
}
"#,
    );

    // 3) Component with serialize, require, optional, input sugar, listen, intrinsic
    write_file(
        &root.join("Assets").join("ActorController.prsm"),
        r#"using UnityEngine
using UnityEngine.UI

component ActorController : MonoBehaviour {
    serialize speed: Float = 5.0
    serialize jumpForce: Float = 10.0
    serialize jumpButton: Button

    require rb: Rigidbody
    optional animator: Animator

    start {
        listen jumpButton.onClick {
            jump()
        }
    }

    update {
        val h = input.axis("Horizontal")
        val v = input.axis("Vertical")
        rb.velocity = vec3(h, 0, v) * speed
    }

    func jump() {
        rb.addForce(vec3(0, jumpForce, 0))
        animator?.play("Jump")
    }

    intrinsic func log(message: String) {
        Debug.Log(message);
    }
}
"#,
    );

    // 4) UI component — multiple listen handlers, if-else flag toggle
    write_file(
        &root.join("Assets").join("GameUI.prsm"),
        r#"using UnityEngine
using UnityEngine.UI
using UnityEngine.SceneManagement

component GameUI : MonoBehaviour {
    serialize playButton: Button
    serialize quitButton: Button
    serialize pauseButton: Button

    var paused: Bool = false

    start {
        listen playButton.onClick {
            SceneManager.loadScene("Game")
        }

        listen quitButton.onClick {
            Application.quit()
        }

        listen pauseButton.onClick {
            togglePause()
        }
    }

    func togglePause() {
        if paused {
            paused = false
        } else {
            paused = true
        }
    }
}
"#,
    );

    let output = Command::new(prism())
        .args(["build", "--json"])
        .current_dir(&root)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "build failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json_start = stdout.find('{').expect("expected JSON output");
    let json: serde_json::Value = serde_json::from_str(&stdout[json_start..]).unwrap();

    assert_eq!(json["project"], "E2EGame");
    assert_eq!(json["errors"], 0, "expected 0 errors; got {}", json["errors"]);
    assert_eq!(json["files"], 4);
    assert_eq!(json["compiled"], 4);

    let gen = root.join("Generated").join("PrSM");

    // All four generated .cs and .prsmmap.json sidecar files must be present
    for name in ["ItemData", "ActorHealth", "ActorController", "GameUI"] {
        assert!(gen.join(format!("{name}.cs")).exists(), "{name}.cs missing");
        assert!(gen.join(format!("{name}.prsmmap.json")).exists(), "{name}.prsmmap.json missing");
    }

    // asset → extends ScriptableObject, [SerializeField]
    let item_cs = fs::read_to_string(gen.join("ItemData.cs")).unwrap();
    assert!(item_cs.contains("ScriptableObject"), "ItemData.cs: expected ScriptableObject");
    assert!(item_cs.contains("[SerializeField]"), "ItemData.cs: expected [SerializeField]");

    // coroutine → IEnumerator + yield return + StartCoroutine
    let health_cs = fs::read_to_string(gen.join("ActorHealth.cs")).unwrap();
    assert!(health_cs.contains("IEnumerator"), "ActorHealth.cs: expected IEnumerator");
    assert!(health_cs.contains("yield return"), "ActorHealth.cs: expected yield return");
    assert!(health_cs.contains("StartCoroutine"), "ActorHealth.cs: expected StartCoroutine");

    // require → GetComponent<Rigidbody> + null guard + enabled = false
    // listen → AddListener
    // input sugar → Input.GetAxis
    // intrinsic → verbatim Debug.Log body
    let ctrl_cs = fs::read_to_string(gen.join("ActorController.cs")).unwrap();
    assert!(ctrl_cs.contains("GetComponent<Rigidbody>"), "ActorController.cs: expected Rigidbody GetComponent");
    assert!(ctrl_cs.contains("enabled = false"), "ActorController.cs: expected enabled = false");
    assert!(ctrl_cs.contains("AddListener"), "ActorController.cs: expected AddListener");
    assert!(ctrl_cs.contains("Input.GetAxis"), "ActorController.cs: expected Input.GetAxis");
    assert!(ctrl_cs.contains("Debug.Log(message)"), "ActorController.cs: expected intrinsic body");

    // three listen → three AddListener calls
    let ui_cs = fs::read_to_string(gen.join("GameUI.cs")).unwrap();
    let listener_count = ui_cs.matches("AddListener").count();
    assert!(
        listener_count >= 3,
        "GameUI.cs: expected ≥3 AddListener calls, got {listener_count}"
    );

    // HIR and index stats must cover all 4 files
    assert_eq!(
        json["hir"]["files_indexed"].as_u64().unwrap_or(0),
        4,
        "hir.files_indexed mismatch"
    );
    assert!(
        json["index"]["top_level_symbols"].as_u64().unwrap_or(0) >= 4,
        "expected ≥4 top-level index symbols, got {}",
        json["index"]["top_level_symbols"]
    );

    let _ = fs::remove_dir_all(root);
}

// ── Negative test suite ──────────────────────────────────────────────────────
//
// Every file under `tests/invalid/` must cause `prism check` to report at
// least one error.  The expected diagnostic code is embedded in a comment on
// the first line of each file:  `// E050: description`.
//
// This test discovers all files automatically — adding a new negative case is
// as simple as dropping a `.prsm` file into tests/invalid/.

#[test]
fn invalid_files_all_produce_errors() {
    // Locate the repo root relative to this integration test binary.
    // CARGO_MANIFEST_DIR is the crates/refraction directory.
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let invalid_dir = manifest_dir.join("..").join("..").join("tests").join("invalid");
    let invalid_dir = invalid_dir.canonicalize().unwrap_or(invalid_dir);

    let entries: Vec<_> = fs::read_dir(&invalid_dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {}", invalid_dir.display(), e))
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("prsm"))
        .collect();

    assert!(!entries.is_empty(), "No .prsm files found in {}", invalid_dir.display());

    let total = entries.len();
    let mut failures: Vec<String> = Vec::new();

    for entry in entries {
        let path = entry.path();
        let file_name = path.file_name().unwrap().to_string_lossy().to_string();

        // Read expected code from first-line comment: `// EXXX: ...`
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {}", path.display(), e));
        let expected_code = source.lines().next().and_then(|line| {
            let line = line.trim_start_matches('/').trim();
            line.split(':').next().map(|s| s.trim().to_string())
        });

        let output = Command::new(prism())
            .args(["check", path.to_str().unwrap(), "--json"])
            .output()
            .unwrap_or_else(|e| panic!("failed to run prism check on {}: {}", file_name, e));

        // prism check returns non-zero exit code when there are errors
        if output.status.success() {
            failures.push(format!(
                "FAIL [{}]: expected at least one error but prism check exited successfully",
                file_name
            ));
            continue;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = match serde_json::from_str(stdout.as_ref()) {
            Ok(v) => v,
            Err(e) => {
                failures.push(format!(
                    "FAIL [{}]: could not parse JSON output: {} — stdout: {}",
                    file_name, e, stdout
                ));
                continue;
            }
        };

        let error_count = json["errors"].as_u64().unwrap_or(0);
        if error_count == 0 {
            failures.push(format!(
                "FAIL [{}]: JSON shows 0 errors",
                file_name
            ));
            continue;
        }

        // If the file declares an expected code, verify at least one diagnostic matches.
        if let Some(code) = expected_code {
            if code.starts_with('E') || code.starts_with('W') {
                let diagnostics = json["diagnostics"].as_array().cloned().unwrap_or_default();
                let has_code = diagnostics
                    .iter()
                    .any(|d| d["code"].as_str() == Some(code.as_str()));
                if !has_code {
                    let actual_codes: Vec<&str> = diagnostics
                        .iter()
                        .filter_map(|d| d["code"].as_str())
                        .collect();
                    failures.push(format!(
                        "FAIL [{}]: expected diagnostic code {} but got {:?}",
                        file_name, code, actual_codes
                    ));
                }
            }
        }
    }

    if !failures.is_empty() {
        panic!(
            "Negative test suite failures ({}/{} files):\n  {}",
            failures.len(),
            total,
            failures.join("\n  ")
        );
    }
}

// ── v2 listen lifetime tests ──────────────────────────────────────────────────

#[test]
fn v2_listen_until_disable_emits_cleanup_in_on_disable() {
    let root = unique_temp_dir("prism_v2_listen_until_disable");
    write_file(
        &root.join(".prsmproject"),
        r#"[project]
name = "V2ListenProject"

[compiler]
output_dir = "Generated/PrSM"

[source]
include = ["Assets/**/*.prsm"]
exclude = []
"#,
    );
    write_file(
        &root.join("Assets").join("UiController.prsm"),
        r#"using UnityEngine.UI

component UiController : MonoBehaviour {
    serialize button: Button

    onEnable {
        listen button.onClick until disable {
            nativeLog("clicked")
        }
    }

    intrinsic func nativeLog(message: String) {
        Debug.Log(message);
    }
}
"#,
    );

    let output = Command::new(prism())
        .args(["build", "--json"])
        .current_dir(&root)
        .output()
        .unwrap();

    assert!(output.status.success(), "build failed:\n{}", String::from_utf8_lossy(&output.stderr));

    let generated_path = root.join("Generated").join("PrSM").join("UiController.cs");
    let src = fs::read_to_string(&generated_path).unwrap();

    // Handler field declared
    assert!(src.contains("private System.Action _prsm_h0"), "missing handler field:\n{src}");
    // AddListener uses named handler
    assert!(src.contains("button.onClick.AddListener(_prsm_h0)"), "missing AddListener:\n{src}");
    // Cleanup method synthesised
    assert!(src.contains("__prsm_cleanup_disable"), "missing cleanup method:\n{src}");
    // RemoveListener called in cleanup
    assert!(src.contains("button.onClick.RemoveListener(_prsm_h0)"), "missing RemoveListener:\n{src}");
    // OnDisable calls cleanup
    assert!(src.contains("OnDisable"), "missing OnDisable:\n{src}");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn v2_listen_until_destroy_emits_cleanup_in_on_destroy() {
    let root = unique_temp_dir("prism_v2_listen_until_destroy");
    write_file(
        &root.join(".prsmproject"),
        r#"[project]
name = "V2ListenDestroyProject"

[compiler]
output_dir = "Generated/PrSM"

[source]
include = ["Assets/**/*.prsm"]
exclude = []
"#,
    );
    write_file(
        &root.join("Assets").join("Spawner.prsm"),
        r#"component Spawner : MonoBehaviour {
    serialize source: GameObject

    start {
        listen source.GetComponent_EventEmitter().onSpawn until destroy {
            debug("spawned")
        }
    }

    intrinsic func debug(msg: String) {
        Debug.Log(msg);
    }
}
"#,
    );

    let output = Command::new(prism())
        .args(["build", "--json"])
        .current_dir(&root)
        .output()
        .unwrap();

    assert!(output.status.success(), "build failed:\n{}", String::from_utf8_lossy(&output.stderr));

    let generated_path = root.join("Generated").join("PrSM").join("Spawner.cs");
    let src = fs::read_to_string(&generated_path).unwrap();

    assert!(src.contains("private System.Action _prsm_h0"), "missing handler field:\n{src}");
    assert!(src.contains("__prsm_cleanup_destroy"), "missing cleanup_destroy method:\n{src}");
    assert!(src.contains("RemoveListener(_prsm_h0)"), "missing RemoveListener:\n{src}");
    assert!(src.contains("OnDestroy"), "missing OnDestroy:\n{src}");
    // v1 cleanup_disable should NOT be present
    assert!(!src.contains("__prsm_cleanup_disable"), "unexpected cleanup_disable:\n{src}");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn v1_listen_register_unchanged() {
    let root = unique_temp_dir("prism_v1_listen_register");
    write_file(
        &root.join(".prsmproject"),
        r#"[project]
name = "V1ListenProject"

[compiler]
output_dir = "Generated/PrSM"

[source]
include = ["Assets/**/*.prsm"]
exclude = []
"#,
    );
    write_file(
        &root.join("Assets").join("Button.prsm"),
        r#"using UnityEngine.UI

component ButtonHandler : MonoBehaviour {
    serialize button: Button

    start {
        listen button.onClick {
            fire()
        }
    }

    func fire(): Unit {
        intrinsic { Debug.Log("fire"); }
    }
}
"#,
    );

    let output = Command::new(prism())
        .args(["build", "--json"])
        .current_dir(&root)
        .output()
        .unwrap();

    assert!(output.status.success(), "build failed:\n{}", String::from_utf8_lossy(&output.stderr));

    let generated_path = root.join("Generated").join("PrSM").join("Button.cs");
    let src = fs::read_to_string(&generated_path).unwrap();

    // v1: inline AddListener, no handler fields, no cleanup
    assert!(src.contains("button.onClick.AddListener(()"), "missing inline AddListener:\n{src}");
    assert!(!src.contains("_prsm_h"), "unexpected handler field for v1 listen:\n{src}");
    assert!(!src.contains("__prsm_cleanup"), "unexpected cleanup for v1 listen:\n{src}");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn v2_listen_manual_with_unlisten() {
    let root = unique_temp_dir("prism_v2_listen_manual");
    write_file(
        &root.join(".prsmproject"),
        r#"[project]
name = "V2ManualListenProject"

[compiler]
output_dir = "Generated/PrSM"

[source]
include = ["Assets/**/*.prsm"]
exclude = []
"#,
    );
    write_file(
        &root.join("Assets").join("Timer.prsm"),
        r#"component TimerHandler : MonoBehaviour {
    serialize timer: UnityEvent

    onEnable {
        val sub = listen timer manual {
            tick()
        }
    }

    onDisable {
        unlisten sub
    }

    intrinsic func tick() {
        Debug.Log("tick");
    }
}
"#,
    );

    let output = Command::new(prism())
        .args(["build", "--json"])
        .current_dir(&root)
        .output()
        .unwrap();

    assert!(output.status.success(), "build failed:\n{}", String::from_utf8_lossy(&output.stderr));

    let generated_path = root.join("Generated").join("PrSM").join("Timer.cs");
    let src = fs::read_to_string(&generated_path).unwrap();

    // Handler field declared
    assert!(src.contains("private System.Action _prsm_h0"), "missing handler field:\n{src}");
    // AddListener called
    assert!(src.contains("timer.AddListener(_prsm_h0)"), "missing AddListener:\n{src}");
    // unlisten resolved to RemoveListener
    assert!(src.contains("timer.RemoveListener(_prsm_h0)"), "missing RemoveListener:\n{src}");

    let _ = fs::remove_dir_all(root);
}

// ── Item #6: v2 pattern matching & destructuring ──────────────────────────────

#[test]
fn v2_when_enum_payload_binding_pattern() {
    let root = unique_temp_dir("prism_when_binding_smoke");
    write_file(
        &root.join("Enemy.prsm"),
        r#"component Enemy : MonoBehaviour {
    val state: EnemyState = EnemyState.Idle

    func tick(): Unit {
        when state {
            EnemyState.Chase(target) => Debug.Log(target)
            EnemyState.Idle => Debug.Log("idle")
            else => {}
        }
    }
}
"#,
    );

    let output = Command::new(prism())
        .args(["compile", root.join("Enemy.prsm").to_str().unwrap(), "--json"])
        .output()
        .unwrap();

    assert!(output.status.success(), "compile failed:\n{}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["errors"], 0, "expected 0 errors; got {}", json["errors"]);

    let cs = fs::read_to_string(root.join("Enemy.cs")).unwrap();
    // Binding pattern must emit a typed case variable
    assert!(cs.contains("case EnemyState.Chase"), "missing binding case:\n{cs}");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn v2_when_guard_condition() {
    let root = unique_temp_dir("prism_when_guard_smoke");
    write_file(
        &root.join("Guard.prsm"),
        r#"component Guard : MonoBehaviour {
    val hp: Int = 100

    func check(): Unit {
        when {
            hp > 50 if hp < 100 => Debug.Log("injured")
            hp > 0 => Debug.Log("alive")
            else => Debug.Log("dead")
        }
    }
}
"#,
    );

    let output = Command::new(prism())
        .args(["compile", root.join("Guard.prsm").to_str().unwrap(), "--json"])
        .output()
        .unwrap();

    assert!(output.status.success(), "compile failed:\n{}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["errors"], 0, "expected 0 errors; got {}", json["errors"]);

    let cs = fs::read_to_string(root.join("Guard.cs")).unwrap();
    // Guard is folded into the condition with &&
    assert!(cs.contains("&&"), "missing && guard:\n{cs}");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn v2_val_destructure_binding() {
    let root = unique_temp_dir("prism_val_destructure_smoke");
    write_file(
        &root.join("Destruct.prsm"),
        r#"component Destruct : MonoBehaviour {
    func run(): Unit {
        val stats = PlayerStats(10, 5)
        val PlayerStats(hp, speed) = stats
        Debug.Log(hp)
    }
}
"#,
    );

    let output = Command::new(prism())
        .args(["compile", root.join("Destruct.prsm").to_str().unwrap(), "--json"])
        .output()
        .unwrap();

    assert!(output.status.success(), "compile failed:\n{}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["errors"], 0, "expected 0 errors; got {}", json["errors"]);

    let cs = fs::read_to_string(root.join("Destruct.cs")).unwrap();
    // Destructure must declare individual binding vars
    assert!(cs.contains("var hp"), "missing 'var hp':\n{cs}");
    assert!(cs.contains("var speed"), "missing 'var speed':\n{cs}");

    let _ = fs::remove_dir_all(root);
}

// ── Item #7: v2 New Input System sugar ──────────────────────────────────────

#[test]
fn v2_new_input_system_action_pressed() {
    // `input.action("Jump").pressed` with feature enabled → PlayerInput field + WasPressedThisFrame
    let root = unique_temp_dir("prism_new_input_pressed_smoke");
    write_file(
        &root.join(".prsmproject"),
        r#"[project]
name = "InputTest"

[language]
version = "2.0"
features = ["input-system"]

[compiler]
output_dir = "Generated/PrSM"

[source]
include = ["*.prsm"]
exclude = []
"#,
    );
    write_file(
        &root.join("Player.prsm"),
        r#"component Player : MonoBehaviour {
    update {
        if input.action("Jump").pressed {
            jump()
        }
    }

    func jump(): Unit {}
}
"#,
    );

    let output = Command::new(prism())
        .args(["build", "--json"])
        .current_dir(&root)
        .output()
        .unwrap();

    assert!(output.status.success(), "build failed:\n{}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json_start = stdout.find('{').expect("expected JSON");
    let json: serde_json::Value = serde_json::from_str(&stdout[json_start..]).unwrap();
    assert_eq!(json["errors"], 0, "expected 0 errors; got {}", json["errors"]);

    let cs = fs::read_to_string(root.join("Generated").join("PrSM").join("Player.cs")).unwrap();
    // PlayerInput backing field injected
    assert!(cs.contains("PlayerInput _prsmInput"), "missing PlayerInput field:\n{cs}");
    // GetComponent<PlayerInput> in Awake
    assert!(cs.contains("GetComponent<PlayerInput>"), "missing GetComponent:\n{cs}");
    // WasPressedThisFrame call lowered correctly
    assert!(cs.contains("WasPressedThisFrame"), "missing WasPressedThisFrame:\n{cs}");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn v2_new_input_system_vector2() {
    // `input.action("Move").vector2` → ReadValue<UnityEngine.Vector2>()
    let root = unique_temp_dir("prism_new_input_vector2_smoke");
    write_file(
        &root.join(".prsmproject"),
        r#"[project]
name = "InputVec2Test"

[language]
version = "2.0"
features = ["input-system"]

[compiler]
output_dir = "Generated/PrSM"

[source]
include = ["*.prsm"]
exclude = []
"#,
    );
    write_file(
        &root.join("Mover.prsm"),
        r#"component Mover : MonoBehaviour {
    update {
        val move = input.action("Move").vector2
    }
}
"#,
    );

    let output = Command::new(prism())
        .args(["build", "--json"])
        .current_dir(&root)
        .output()
        .unwrap();

    assert!(output.status.success(), "build failed:\n{}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json_start = stdout.find('{').expect("expected JSON");
    let json: serde_json::Value = serde_json::from_str(&stdout[json_start..]).unwrap();
    assert_eq!(json["errors"], 0, "expected 0 errors; got {}", json["errors"]);

    let cs = fs::read_to_string(root.join("Generated").join("PrSM").join("Mover.cs")).unwrap();
    assert!(cs.contains("ReadValue<UnityEngine.Vector2>"), "missing ReadValue<Vector2>:\n{cs}");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn v2_new_input_system_feature_gate_error() {
    // `input.action(...)` without feature → compile error E070
    let root = unique_temp_dir("prism_new_input_gate_smoke");
    write_file(
        &root.join("Gate.prsm"),
        r#"component Gate : MonoBehaviour {
    update {
        if input.action("Fire").pressed {
            fire()
        }
    }
    func fire(): Unit {}
}
"#,
    );

    let output = Command::new(prism())
        .args(["compile", root.join("Gate.prsm").to_str().unwrap(), "--json"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    // Must produce error E070
    assert!(
        json["errors"].as_u64().unwrap_or(0) > 0,
        "expected E070 error for missing input-system feature; got:\n{stdout}"
    );
    let diagnostics = json["diagnostics"].as_array().unwrap();
    assert!(
        diagnostics.iter().any(|d| d["code"].as_str() == Some("E070")),
        "expected E070 in diagnostics:\n{:?}", diagnostics
    );

    let _ = fs::remove_dir_all(root);
}

// ── Item #8: v2 generic call type inference ─────────────────────────────────

#[test]
fn v2_generic_inference_from_variable_type() {
    let root = unique_temp_dir("prism_generic_infer_val_smoke");
    let source = root.join("InferVal.prsm");
    write_file(
        &source,
        r#"component InferVal : MonoBehaviour {
    start {
        val health: Health? = child()
    }
}
"#,
    );

    let output = Command::new(prism())
        .args(["compile", source.to_str().unwrap(), "--json"])
        .output()
        .unwrap();

    assert!(output.status.success(), "compile failed:\n{}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["errors"], 0, "expected 0 errors; got {}", json["errors"]);

    let cs = fs::read_to_string(root.join("InferVal.cs")).unwrap();
    assert!(
        cs.contains("GetComponentInChildren<Health>()"),
        "expected nullable lhs inference to strip ? and infer Health:\n{cs}"
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn v2_generic_inference_from_return_type() {
    let root = unique_temp_dir("prism_generic_infer_return_smoke");
    let source = root.join("InferReturn.prsm");
    write_file(
        &source,
        r#"component InferReturn : MonoBehaviour {
    func getRb(): Rigidbody = get()
}
"#,
    );

    let output = Command::new(prism())
        .args(["compile", source.to_str().unwrap(), "--json"])
        .output()
        .unwrap();

    assert!(output.status.success(), "compile failed:\n{}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["errors"], 0, "expected 0 errors; got {}", json["errors"]);

    let cs = fs::read_to_string(root.join("InferReturn.cs")).unwrap();
    assert!(
        cs.contains("return GetComponent<Rigidbody>();"),
        "expected return type inference to emit GetComponent<Rigidbody>:\n{cs}"
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn v2_generic_inference_from_argument_type() {
    let root = unique_temp_dir("prism_generic_infer_arg_smoke");
    let source = root.join("InferArg.prsm");
    write_file(
        &source,
        r#"component InferArg : MonoBehaviour {
    func useRb(rb: Rigidbody): Unit {}

    start {
        useRb(get())
    }
}
"#,
    );

    let output = Command::new(prism())
        .args(["compile", source.to_str().unwrap(), "--json"])
        .output()
        .unwrap();

    assert!(output.status.success(), "compile failed:\n{}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["errors"], 0, "expected 0 errors; got {}", json["errors"]);

    let cs = fs::read_to_string(root.join("InferArg.cs")).unwrap();
    assert!(
        cs.contains("useRb(GetComponent<Rigidbody>())"),
        "expected argument type inference to emit GetComponent<Rigidbody> inside call:\n{cs}"
    );

    let _ = fs::remove_dir_all(root);
}


