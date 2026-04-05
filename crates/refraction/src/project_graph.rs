use crate::project::PrismProject;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageVersion {
    V1,
    V2,
}

impl LanguageVersion {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "1.0" => Ok(Self::V1),
            "2.0" => Ok(Self::V2),
            other => Err(format!(
                "Unsupported language version '{}'. Supported versions: 1.0, 2.0",
                other
            )),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::V1 => "1.0",
            Self::V2 => "2.0",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LanguageFeature {
    AutoUnlisten,
    InputSystem,
    PatternBindings,
}

impl LanguageFeature {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "auto-unlisten" => Ok(Self::AutoUnlisten),
            "input-system" => Ok(Self::InputSystem),
            "pattern-bindings" => Ok(Self::PatternBindings),
            other => Err(format!(
                "Unsupported language feature '{}'. Supported features: auto-unlisten, input-system, pattern-bindings",
                other
            )),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::AutoUnlisten => "auto-unlisten",
            Self::InputSystem => "input-system",
            Self::PatternBindings => "pattern-bindings",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UnityCapabilities {
    pub input_system_package: bool,
}

#[derive(Debug, Clone)]
pub struct ProjectGraph {
    pub config: PrismProject,
    pub project_root: PathBuf,
    pub output_dir: PathBuf,
    pub output_dir_display: String,
    pub source_files: Vec<PathBuf>,
    pub watch_roots: Vec<PathBuf>,
    pub cache_dir: PathBuf,
    pub language_version: LanguageVersion,
    pub enabled_features: Vec<LanguageFeature>,
    pub unity_capabilities: UnityCapabilities,
}

impl ProjectGraph {
    pub fn discover(start_dir: &Path) -> Result<Self, String> {
        let (config, project_root) = PrismProject::find_and_load(start_dir)?;
        let language_version = LanguageVersion::parse(&config.language.version)?;
        let enabled_features = parse_language_features(&config.language.features)?;
        let source_files = config.collect_sources(&project_root);
        let watch_roots = collect_watch_roots(&config, &project_root);
        let unity_capabilities = detect_unity_capabilities(&project_root);
        let output_dir = project_root.join(&config.compiler.output_dir);
        let cache_dir = project_root.join(".prsm").join("cache");

        Ok(Self {
            output_dir_display: config.compiler.output_dir.clone(),
            config,
            project_root,
            output_dir,
            source_files,
            watch_roots,
            cache_dir,
            language_version,
            enabled_features,
            unity_capabilities,
        })
    }

    pub fn feature_names(&self) -> Vec<&'static str> {
        self.enabled_features
            .iter()
            .map(|feature| feature.as_str())
            .collect()
    }
}

fn parse_language_features(values: &[String]) -> Result<Vec<LanguageFeature>, String> {
    let mut features = Vec::new();
    for value in values {
        features.push(LanguageFeature::parse(value)?);
    }
    features.sort();
    features.dedup();
    Ok(features)
}

fn collect_watch_roots(config: &PrismProject, project_root: &Path) -> Vec<PathBuf> {
    let mut roots = Vec::new();

    for pattern in &config.source.include {
        let directory = pattern.split("**").next().unwrap_or(".");
        let watch_root = project_root.join(directory);
        if watch_root.exists() {
            roots.push(watch_root);
        }
    }

    if roots.is_empty() {
        roots.push(project_root.to_path_buf());
    }

    roots.sort();
    roots.dedup();
    roots
}

fn detect_unity_capabilities(project_root: &Path) -> UnityCapabilities {
    let manifest_path = project_root.join("Packages").join("manifest.json");
    let mut capabilities = UnityCapabilities::default();

    let Ok(content) = fs::read_to_string(&manifest_path) else {
        return capabilities;
    };
    let Ok(value) = serde_json::from_str::<Value>(&content) else {
        return capabilities;
    };

    let Some(dependencies) = value.get("dependencies").and_then(Value::as_object) else {
        return capabilities;
    };

    capabilities.input_system_package = dependencies.contains_key("com.unity.inputsystem");
    capabilities
}

#[cfg(test)]
mod tests {
    use super::{LanguageVersion, ProjectGraph};
    use std::fs;
    use std::path::{Path, PathBuf};
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

    fn write_file(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    #[test]
    fn discover_collects_language_and_unity_capabilities() {
        let root = unique_temp_dir("prism_project_graph");
        write_file(
            &root.join(".prsmproject"),
            r#"[project]
name = "GraphProject"
prsm_version = "0.1.0"

[language]
version = "2.0"
features = ["pattern-bindings", "input-system"]

[compiler]
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
            &root.join("Assets").join("Hero.prsm"),
            "component Hero : MonoBehaviour {}",
        );

        let graph = ProjectGraph::discover(&root).unwrap();

        assert_eq!(graph.language_version, LanguageVersion::V2);
        assert_eq!(graph.feature_names(), vec!["input-system", "pattern-bindings"]);
        assert!(graph.unity_capabilities.input_system_package);
        assert_eq!(graph.output_dir, root.join("Generated").join("PrSM"));
        assert_eq!(graph.cache_dir, root.join(".prsm").join("cache"));
        assert_eq!(graph.source_files, vec![root.join("Assets").join("Hero.prsm")]);
        assert_eq!(graph.watch_roots, vec![root.join("Assets")]);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn discover_rejects_unknown_language_feature() {
        let root = unique_temp_dir("prism_project_graph_feature_error");
        write_file(
            &root.join(".prsmproject"),
            r#"[project]
name = "GraphProject"

[language]
version = "1.0"
features = ["unknown-feature"]
"#,
        );

        let error = ProjectGraph::discover(&root).unwrap_err();
        assert!(error.contains("unknown-feature"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn discover_applies_defaults_when_sections_are_omitted() {
        let root = unique_temp_dir("prism_project_graph_defaults");
        write_file(
            &root.join(".prsmproject"),
            r#"[project]
name = "DefaultsProject"
"#,
        );
        write_file(
            &root.join("Scene.prsm"),
            "component Scene : MonoBehaviour {}",
        );

        let graph = ProjectGraph::discover(&root).unwrap();

        assert_eq!(graph.language_version, LanguageVersion::V1);
        assert!(graph.enabled_features.is_empty());
        assert_eq!(graph.output_dir, root.join("Assets").join("Generated").join("PrSM"));
        assert_eq!(graph.source_files, vec![root.join("Scene.prsm")]);
        assert_eq!(graph.watch_roots, vec![root.clone()]);

        let _ = fs::remove_dir_all(root);
    }
}