//! .prsmproject file parser.
//!
//! Reads a TOML project file that configures the PrSM build pipeline.

use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

const PROJECT_FILE: &str = ".prsmproject";
const LEGACY_PROJECT_FILE: &str = ".mnproject";

#[derive(Debug, Clone, Deserialize)]
pub struct PrismProject {
    pub project: ProjectSection,
    #[serde(default)]
    pub compiler: CompilerSection,
    #[serde(default)]
    pub source: SourceSection,
    #[serde(default)]
    pub language: LanguageSection,
    #[serde(default)]
    pub features: FeaturesSection,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectSection {
    pub name: String,
    #[serde(default = "default_version", alias = "moon_version")]
    pub prsm_version: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CompilerSection {
    #[serde(default = "default_prism_path", alias = "moonc_path")]
    pub prism_path: String,
    #[serde(default = "default_output_dir")]
    pub output_dir: String,
    #[serde(default)]
    pub target_unity: String,
}

impl Default for CompilerSection {
    fn default() -> Self {
        Self {
            prism_path: default_prism_path(),
            output_dir: default_output_dir(),
            target_unity: String::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SourceSection {
    #[serde(default = "default_include")]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

impl Default for SourceSection {
    fn default() -> Self {
        Self {
            include: default_include(),
            exclude: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct LanguageSection {
    #[serde(default = "default_language_version")]
    pub version: String,
    #[serde(default)]
    pub features: Vec<String>,
}

impl Default for LanguageSection {
    fn default() -> Self {
        Self {
            version: default_language_version(),
            features: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct FeaturesSection {
    #[serde(default = "default_true")]
    pub auto_compile_on_save: bool,
    #[serde(default)]
    pub generate_meta_files: bool,
    #[serde(default = "default_true")]
    pub pascal_case_methods: bool,
}

impl Default for FeaturesSection {
    fn default() -> Self {
        Self {
            auto_compile_on_save: default_true(),
            generate_meta_files: false,
            pascal_case_methods: default_true(),
        }
    }
}

fn default_version() -> String { "0.1.0".into() }
fn default_language_version() -> String { "1.0".into() }
fn default_prism_path() -> String { "prism".into() }
fn default_output_dir() -> String { "Assets/Generated/PrSM".into() }
fn default_include() -> Vec<String> { vec!["**/*.prsm".into()] }
fn default_true() -> bool { true }

fn is_prism_source_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| matches!(extension, "prsm" | "mn"))
        .unwrap_or(false)
}

impl PrismProject {
    /// Find and load .prsmproject from the given directory or its parents.
    pub fn find_and_load(start_dir: &Path) -> Result<(PrismProject, PathBuf), String> {
        let mut dir = start_dir.to_path_buf();
        loop {
            let project_file = dir.join(PROJECT_FILE);
            let legacy_project_file = dir.join(LEGACY_PROJECT_FILE);
            let active_project_file = if project_file.exists() {
                project_file
            } else if legacy_project_file.exists() {
                legacy_project_file
            } else {
                if !dir.pop() {
                    return Err(format!("No {} found in {} or parent directories", PROJECT_FILE, start_dir.display()));
                }
                continue;
            };

            let content = fs::read_to_string(&active_project_file)
                .map_err(|e| format!("Cannot read {}: {}", active_project_file.display(), e))?;
            let project: PrismProject = toml::from_str(&content)
                .map_err(|e| format!("Invalid {}: {}", active_project_file.display(), e))?;
            return Ok((project, dir));
        }
    }

    /// Collect all .prsm source files based on include/exclude patterns.
    pub fn collect_sources(&self, project_root: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();

        for pattern in &self.source.include {
            let full_pattern = project_root.join(pattern).to_string_lossy().to_string();
            match glob::glob(&full_pattern) {
                Ok(paths) => {
                    for entry in paths.flatten() {
                        if is_prism_source_file(&entry) {
                            if !self.is_excluded(&entry, project_root) {
                                files.push(entry);
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Invalid glob pattern '{}': {}", pattern, e);
                }
            }
        }

        files.sort();
        files.dedup();
        files
    }

    fn is_excluded(&self, path: &Path, project_root: &Path) -> bool {
        let rel = path.strip_prefix(project_root).unwrap_or(path);
        let rel_str = rel.to_string_lossy();
        for pattern in &self.source.exclude {
            if let Ok(glob_pattern) = glob::Pattern::new(pattern) {
                if glob_pattern.matches(&rel_str) {
                    return true;
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::PrismProject;
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
    fn collect_sources_accepts_legacy_and_current_extensions() {
        let root = unique_temp_dir("prism_project_sources");
        write_file(
            &root.join(".prsmproject"),
            r#"[project]
name = "MixedSources"

[source]
include = ["Assets/**/*.prsm", "Assets/**/*.mn"]
"#,
        );
        write_file(&root.join("Assets").join("Current.prsm"), "component Current : MonoBehaviour {}");
        write_file(&root.join("Assets").join("Legacy.mn"), "component Legacy : MonoBehaviour {}");

        let (project, project_root) = PrismProject::find_and_load(&root).unwrap();
        let sources = project.collect_sources(&project_root);

        assert_eq!(
            sources,
            vec![
                root.join("Assets").join("Current.prsm"),
                root.join("Assets").join("Legacy.mn"),
            ]
        );

        let _ = fs::remove_dir_all(root);
    }
}
