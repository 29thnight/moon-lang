//! .mnproject file parser.
//!
//! Reads a TOML project file that configures the Moon build pipeline.

use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::fs;

const PROJECT_FILE: &str = ".mnproject";

#[derive(Debug, Deserialize)]
pub struct MoonProject {
    pub project: ProjectSection,
    #[serde(default)]
    pub compiler: CompilerSection,
    #[serde(default)]
    pub source: SourceSection,
    #[serde(default)]
    pub features: FeaturesSection,
}

#[derive(Debug, Deserialize)]
pub struct ProjectSection {
    pub name: String,
    #[serde(default = "default_version")]
    pub moon_version: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct CompilerSection {
    #[serde(default = "default_moonc_path")]
    pub moonc_path: String,
    #[serde(default = "default_output_dir")]
    pub output_dir: String,
    #[serde(default)]
    pub target_unity: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct SourceSection {
    #[serde(default = "default_include")]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct FeaturesSection {
    #[serde(default = "default_true")]
    pub auto_compile_on_save: bool,
    #[serde(default)]
    pub generate_meta_files: bool,
    #[serde(default = "default_true")]
    pub pascal_case_methods: bool,
}

fn default_version() -> String { "0.1.0".into() }
fn default_moonc_path() -> String { "moonc".into() }
fn default_output_dir() -> String { "Assets/Generated/Moon".into() }
fn default_include() -> Vec<String> { vec!["**/*.mn".into()] }
fn default_true() -> bool { true }

impl MoonProject {
    /// Find and load .mnproject from the given directory or its parents.
    pub fn find_and_load(start_dir: &Path) -> Result<(MoonProject, PathBuf), String> {
        let mut dir = start_dir.to_path_buf();
        loop {
            let project_file = dir.join(PROJECT_FILE);
            if project_file.exists() {
                let content = fs::read_to_string(&project_file)
                    .map_err(|e| format!("Cannot read {}: {}", project_file.display(), e))?;
                let project: MoonProject = toml::from_str(&content)
                    .map_err(|e| format!("Invalid .mnproject: {}", e))?;
                return Ok((project, dir));
            }
            if !dir.pop() {
                return Err(format!("No {} found in {} or parent directories", PROJECT_FILE, start_dir.display()));
            }
        }
    }

    /// Collect all .mn source files based on include/exclude patterns.
    pub fn collect_sources(&self, project_root: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();

        for pattern in &self.source.include {
            let full_pattern = project_root.join(pattern).to_string_lossy().to_string();
            match glob::glob(&full_pattern) {
                Ok(paths) => {
                    for entry in paths.flatten() {
                        if entry.extension().map_or(false, |ext| ext == "mn") {
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
