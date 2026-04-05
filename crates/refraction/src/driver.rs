use crate::codegen::emitter;
use crate::diagnostics::{Diagnostic, Severity};
use crate::hir::{HirFile, HirProject, HirStats};
use crate::lexer::{
    lexer::Lexer,
    token::{Position, Span},
};
use crate::lowering::lower::lower_file;
use crate::parser::parser::Parser;
use crate::project_graph::ProjectGraph;
use crate::project_index::{build_project_index, DeclarationKind, ProjectIndex, ProjectIndexStats};
use crate::semantic::analyzer::Analyzer;
use crate::source_map;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
pub struct JsonDiagnostic {
    pub code: String,
    pub severity: String,
    pub message: String,
    pub file: String,
    pub line: u32,
    pub col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

#[derive(Debug, Clone)]
pub struct FileResult {
    pub source_path: PathBuf,
    pub output_path: Option<PathBuf>,
    pub source_map_path: Option<PathBuf>,
    pub diagnostics: Vec<Diagnostic>,
    pub has_errors: bool,
}

#[derive(Debug, Clone)]
pub struct DriverReport {
    pub files: usize,
    pub compiled: usize,
    pub errors: u32,
    pub warnings: u32,
    pub diagnostics: Vec<JsonDiagnostic>,
    pub file_results: Vec<FileResult>,
}

#[derive(Debug, Clone)]
pub struct ProjectBuildReport {
    pub project_name: String,
    pub project_root: PathBuf,
    pub output_dir: PathBuf,
    pub output_dir_display: String,
    pub cache_dir: PathBuf,
    pub sources: Vec<PathBuf>,
    pub language_version: String,
    pub language_features: Vec<String>,
    pub unity_input_system: bool,
    pub hir_project: HirProject,
    pub hir_stats: HirStats,
    pub project_index: ProjectIndex,
    pub index_stats: ProjectIndexStats,
    pub report: DriverReport,
}

pub fn format_diagnostic(diagnostic: &Diagnostic, file_path: &str) -> String {
    let severity = match diagnostic.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
    };
    format!(
        "{}:{}:{}: {} [{}]: {}",
        file_path,
        diagnostic.span.start.line,
        diagnostic.span.start.col,
        severity,
        diagnostic.code,
        diagnostic.message
    )
}

pub fn to_json_diagnostic(diagnostic: &Diagnostic, file_path: &str) -> JsonDiagnostic {
    JsonDiagnostic {
        code: diagnostic.code.to_string(),
        severity: match diagnostic.severity {
            Severity::Error => "error".into(),
            Severity::Warning => "warning".into(),
        },
        message: diagnostic.message.clone(),
        file: file_path.into(),
        line: diagnostic.span.start.line,
        col: diagnostic.span.start.col,
        end_line: diagnostic.span.end.line,
        end_col: diagnostic.span.end.col,
    }
}

pub fn compile_file(source_path: &Path, output_dir: Option<&Path>) -> FileResult {
    let mut result = analyze_file(source_path);
    if result.has_errors {
        return result;
    }

    let file = match fs::read_to_string(source_path) {
        Ok(source) => {
            let mut lexer = Lexer::new(&source);
            let tokens = lexer.tokenize();
            let mut parser = Parser::new(tokens);
            parser.parse_file()
        }
        Err(error) => {
            result
                .diagnostics
                .push(io_error(format!("Cannot read file: {}", error)));
            result.has_errors = true;
            return result;
        }
    };

    let mut analyzer = Analyzer::new();
    let hir_file = analyzer.analyze_file_with_hir(&file, source_path);

    let ir = lower_file(&file);
    let output = emitter::emit(&ir);

    let out_path = if let Some(out_dir) = output_dir {
        let file_name = source_path.file_stem().unwrap().to_string_lossy();
        out_dir.join(format!("{}.cs", file_name))
    } else {
        source_path.with_extension("cs")
    };

    if let Some(parent) = out_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    match fs::write(&out_path, output) {
        Ok(_) => {
            result.output_path = Some(out_path.clone());
            match source_map::write_source_map(&hir_file, &ir, &out_path, &fs::read_to_string(&out_path).unwrap_or_default()) {
                Ok(source_map_path) => result.source_map_path = Some(source_map_path),
                Err(error) => {
                    result.diagnostics.push(io_error(error));
                    result.has_errors = true;
                }
            }
        }
        Err(error) => {
            result
                .diagnostics
                .push(io_error(format!("Cannot write output: {}", error)));
            result.has_errors = true;
        }
    }

    result
}

pub fn check_file(source_path: &Path) -> FileResult {
    analyze_file(source_path)
}

pub fn compile_paths(files: &[PathBuf], output_dir: Option<&Path>) -> DriverReport {
    summarize(files.iter().map(|file| compile_file(file, output_dir)).collect())
}

pub fn check_paths(files: &[PathBuf]) -> DriverReport {
    summarize(files.iter().map(|file| check_file(file)).collect())
}

pub fn build_project(start_dir: &Path) -> Result<ProjectBuildReport, String> {
    let graph = ProjectGraph::discover(start_dir)?;
    if graph.source_files.is_empty() {
        return Err("No .prsm files found matching source patterns".into());
    }

    let hir_project = build_hir_project(&graph.source_files);
    let hir_stats = hir_project.stats();
    let project_index = build_project_index(&graph.source_files);
    let index_stats = project_index.stats();

    fs::create_dir_all(&graph.output_dir)
        .map_err(|error| format!("Cannot create output directory {}: {}", graph.output_dir.display(), error))?;
    fs::create_dir_all(&graph.cache_dir)
        .map_err(|error| format!("Cannot create cache directory {}: {}", graph.cache_dir.display(), error))?;

    let report = compile_paths(&graph.source_files, Some(graph.output_dir.as_path()));

    Ok(ProjectBuildReport {
        project_name: graph.config.project.name.clone(),
        project_root: graph.project_root.clone(),
        output_dir: graph.output_dir.clone(),
        output_dir_display: graph.output_dir_display.clone(),
        cache_dir: graph.cache_dir.clone(),
        sources: graph.source_files.clone(),
        language_version: graph.language_version.as_str().to_string(),
        language_features: graph
            .feature_names()
            .into_iter()
            .map(str::to_string)
            .collect(),
        unity_input_system: graph.unity_capabilities.input_system_package,
        hir_project,
        hir_stats,
        project_index,
        index_stats,
        report,
    })
}

pub fn build_hir_file(source_path: &Path) -> Result<HirFile, String> {
    let source = fs::read_to_string(source_path)
        .map_err(|error| format!("Cannot read file {}: {}", source_path.display(), error))?;

    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let file = parser.parse_file();

    if !parser.errors().is_empty() {
        let messages = parser
            .errors()
            .iter()
            .map(|error| error.message.clone())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!("Parse errors in {}: {}", source_path.display(), messages));
    }

    let mut analyzer = Analyzer::new();
    Ok(analyzer.analyze_file_with_hir(&file, source_path))
}

pub fn build_hir_project(files: &[PathBuf]) -> HirProject {
    let mut project = HirProject::default();
    let known_project_types = collect_project_types(files);

    for file in files {
        match build_hir_file_with_known_types(file, &known_project_types) {
            Ok(hir_file) => project.files.push(hir_file),
            Err(_) => project.skipped_files.push(file.clone()),
        }
    }

    project
}

fn build_hir_file_with_known_types(
    source_path: &Path,
    known_project_types: &HashMap<String, crate::semantic::types::PrismType>,
) -> Result<HirFile, String> {
    let source = fs::read_to_string(source_path)
        .map_err(|error| format!("Cannot read file {}: {}", source_path.display(), error))?;

    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let file = parser.parse_file();

    if !parser.errors().is_empty() {
        let messages = parser
            .errors()
            .iter()
            .map(|error| error.message.clone())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!("Parse errors in {}: {}", source_path.display(), messages));
    }

    let mut analyzer = Analyzer::with_known_project_types(known_project_types.clone());
    Ok(analyzer.analyze_file_with_hir(&file, source_path))
}

fn collect_project_types(files: &[PathBuf]) -> HashMap<String, crate::semantic::types::PrismType> {
    let project_index = build_project_index(files);
    let mut known_types = HashMap::new();

    for file in &project_index.files {
        let decl = &file.declaration;
        let ty = match decl.kind {
            DeclarationKind::Component => crate::semantic::types::PrismType::Component(decl.name.clone()),
            DeclarationKind::Asset => crate::semantic::types::PrismType::Asset(decl.name.clone()),
            DeclarationKind::Class | DeclarationKind::DataClass | DeclarationKind::Attribute => {
                crate::semantic::types::PrismType::Class(decl.name.clone())
            }
            DeclarationKind::Enum => crate::semantic::types::PrismType::Enum(decl.name.clone()),
        };
        known_types.insert(decl.name.clone(), ty);
    }

    known_types
}

pub fn collect_prsm_files(path: &str) -> Result<Vec<PathBuf>, String> {
    let path = Path::new(path);
    if path.is_file() {
        Ok(vec![path.to_path_buf()])
    } else if path.is_dir() {
        Ok(collect_prsm_files_recursive(path))
    } else {
        Err(format!("'{}' is not a file or directory", path.display()))
    }
}

fn collect_prsm_files_recursive(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(collect_prsm_files_recursive(&path));
            } else if path.extension().map_or(false, |ext| ext == "prsm") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

fn analyze_file(source_path: &Path) -> FileResult {
    let mut result = FileResult {
        source_path: source_path.to_path_buf(),
        output_path: None,
        source_map_path: None,
        diagnostics: Vec::new(),
        has_errors: false,
    };

    let source = match fs::read_to_string(source_path) {
        Ok(source) => source,
        Err(error) => {
            result
                .diagnostics
                .push(io_error(format!("Cannot read file: {}", error)));
            result.has_errors = true;
            return result;
        }
    };

    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize();

    let mut parser = Parser::new(tokens);
    let file = parser.parse_file();
    for error in parser.errors() {
        result.diagnostics.push(Diagnostic::error(
            "E100",
            error.message.clone(),
            error.span,
        ));
        result.has_errors = true;
    }
    if result.has_errors {
        return result;
    }

    let mut analyzer = Analyzer::new();
    analyzer.analyze_file(&file);
    for diagnostic in &analyzer.diag.diagnostics {
        let is_error = diagnostic.severity == Severity::Error;
        result.diagnostics.push(diagnostic.clone());
        if is_error {
            result.has_errors = true;
        }
    }

    result
}

fn summarize(file_results: Vec<FileResult>) -> DriverReport {
    let mut compiled = 0usize;
    let mut errors = 0u32;
    let mut warnings = 0u32;
    let mut diagnostics = Vec::new();

    for file_result in &file_results {
        let file_path = file_result.source_path.to_string_lossy().to_string();
        if file_result.output_path.is_some() {
            compiled += 1;
        }

        for diagnostic in &file_result.diagnostics {
            match diagnostic.severity {
                Severity::Error => errors += 1,
                Severity::Warning => warnings += 1,
            }
            diagnostics.push(to_json_diagnostic(diagnostic, &file_path));
        }
    }

    DriverReport {
        files: file_results.len(),
        compiled,
        errors,
        warnings,
        diagnostics,
        file_results,
    }
}

fn io_error(message: String) -> Diagnostic {
    Diagnostic::error(
        "E000",
        message,
        Span {
            start: Position { line: 0, col: 0 },
            end: Position { line: 0, col: 0 },
        },
    )
}
