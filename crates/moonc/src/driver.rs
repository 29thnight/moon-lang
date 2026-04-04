use crate::codegen::emitter;
use crate::diagnostics::{Diagnostic, Severity};
use crate::lexer::{
    lexer::Lexer,
    token::{Position, Span},
};
use crate::lowering::lower::lower_file;
use crate::parser::parser::Parser;
use crate::project::MoonProject;
use crate::semantic::analyzer::Analyzer;
use serde::Serialize;
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
}

#[derive(Debug, Clone)]
pub struct FileResult {
    pub source_path: PathBuf,
    pub output_path: Option<PathBuf>,
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
    pub sources: Vec<PathBuf>,
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
        Ok(_) => result.output_path = Some(out_path),
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
    let (project, project_root) = MoonProject::find_and_load(start_dir)?;
    let sources = project.collect_sources(&project_root);
    if sources.is_empty() {
        return Err("No .mn files found matching source patterns".into());
    }

    let output_dir = project_root.join(&project.compiler.output_dir);
    fs::create_dir_all(&output_dir)
        .map_err(|error| format!("Cannot create output directory {}: {}", output_dir.display(), error))?;

    let report = compile_paths(&sources, Some(output_dir.as_path()));

    Ok(ProjectBuildReport {
        project_name: project.project.name,
        project_root,
        output_dir,
        output_dir_display: project.compiler.output_dir,
        sources,
        report,
    })
}

pub fn collect_moon_files(path: &str) -> Result<Vec<PathBuf>, String> {
    let path = Path::new(path);
    if path.is_file() {
        Ok(vec![path.to_path_buf()])
    } else if path.is_dir() {
        Ok(collect_moon_files_recursive(path))
    } else {
        Err(format!("'{}' is not a file or directory", path.display()))
    }
}

fn collect_moon_files_recursive(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(collect_moon_files_recursive(&path));
            } else if path.extension().map_or(false, |ext| ext == "mn") {
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
