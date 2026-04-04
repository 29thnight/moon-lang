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

[compiler]
moonc_path = "moonc"
output_dir = "Generated/Moon"

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

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json_start = stdout.find('{').expect("expected JSON output");
    let json: serde_json::Value = serde_json::from_str(&stdout[json_start..]).unwrap();

    assert_eq!(json["project"], "SmokeProject");
    assert_eq!(json["files"], 1);
    assert_eq!(json["compiled"], 1);
    assert_eq!(json["errors"], 0);
    assert!(root.join("Generated").join("Moon").join("BuildTarget.cs").exists());

    let _ = fs::remove_dir_all(root);
}
