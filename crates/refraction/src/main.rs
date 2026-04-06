use clap::{Parser as ClapParser, Subcommand};
use refraction::{driver, lsp, project_graph, project_index, r#where};
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

#[derive(ClapParser)]
#[command(name = "prism")]
#[command(version = "0.1.0")]
#[command(about = "Refraction compiler for PrSM - Unity-first scripting language")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile .prsm file(s) to C#
    Compile {
        /// File or directory to compile
        path: String,

        /// Output directory (default: same as source)
        #[arg(short, long)]
        output: Option<String>,

        /// Output diagnostics as JSON
        #[arg(long)]
        json: bool,

        /// Suppress warnings
        #[arg(short = 'w', long)]
        no_warnings: bool,
    },

    /// Check .prsm file(s) without generating output
    Check {
        /// File or directory to check
        path: String,

        /// Output diagnostics as JSON
        #[arg(long)]
        json: bool,
    },

    /// Build project using .prsmproject configuration
    Build {
        /// Output diagnostics as JSON
        #[arg(long)]
        json: bool,

        /// Watch for file changes and rebuild automatically
        #[arg(long)]
        watch: bool,
    },

    /// Emit Typed HIR for a file or project
    Hir {
        /// Project directory, a path inside the project, or a single .prsm file
        #[arg(default_value = ".")]
        path: String,

        /// Output HIR as JSON
        #[arg(long)]
        json: bool,
    },

    /// Prototype go-to-definition lookup using Typed HIR
    Definition {
        /// Project directory or a path inside the project
        #[arg(default_value = ".")]
        path: String,

        /// File path relative to the project root, or an absolute path
        #[arg(long)]
        file: String,

        /// 1-based line number
        #[arg(long)]
        line: u32,

        /// 1-based column number
        #[arg(long)]
        col: u32,

        /// Output definition data as JSON
        #[arg(long)]
        json: bool,
    },

    /// Prototype find-references lookup using Typed HIR
    References {
        /// Project directory or a path inside the project
        #[arg(default_value = ".")]
        path: String,

        /// File path relative to the project root, or an absolute path
        #[arg(long)]
        file: String,

        /// 1-based line number
        #[arg(long)]
        line: u32,

        /// 1-based column number
        #[arg(long)]
        col: u32,

        /// Output references data as JSON
        #[arg(long)]
        json: bool,
    },

    /// Inspect the project symbol index
    Index {
        /// Project directory or a path inside the project
        #[arg(default_value = ".")]
        path: String,

        /// Output index data as JSON
        #[arg(long)]
        json: bool,

        /// Filter by exact symbol name
        #[arg(long)]
        symbol: Option<String>,

        /// Filter by exact qualified name (for example Player.jump)
        #[arg(long = "qualified-name")]
        qualified_name: Option<String>,

        /// Resolve the symbol that contains this file position
        #[arg(long, requires_all = ["line", "col"])]
        file: Option<String>,

        /// 1-based line for --file lookup
        #[arg(long, requires_all = ["file", "col"])]
        line: Option<u32>,

        /// 1-based column for --file lookup
        #[arg(long, requires_all = ["file", "line"])]
        col: Option<u32>,
    },

    /// Initialize a new .prsmproject in the current directory
    Init {
        /// Project name
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Print the absolute path of the prism binary
    Where,

    /// Run the PrSM Language Server Protocol (LSP) server over stdio
    Lsp,

    /// Print version information
    Version,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Where => {
            r#where::print_where();
        }
        Commands::Lsp => {
            if let Err(error) = lsp::run_server() {
                eprintln!("{}", error);
                process::exit(1);
            }
        }
        Commands::Version => {
            println!("prism 0.1.0");
            println!("Refraction compiler for PrSM - Unity-first scripting language");
            println!("Backend: C# source generation");
        }
        Commands::Compile {
            path,
            output,
            json,
            no_warnings,
        } => {
            let files = match driver::collect_prsm_files(&path) {
                Ok(files) if !files.is_empty() => files,
                Ok(_) => {
                    eprintln!("No .prsm files found in '{}'", path);
                    process::exit(1);
                }
                Err(error) => {
                    eprintln!("Error: {}", error);
                    process::exit(1);
                }
            };

            let output_dir = output.as_ref().map(|value| Path::new(value.as_str()));
            let report = driver::compile_paths(&files, output_dir);

            if json {
                print_json(serde_json::json!({
                    "files": report.files,
                    "compiled": report.compiled,
                    "errors": report.errors,
                    "warnings": report.warnings,
                    "diagnostics": report.diagnostics,
                    "outputs": compiled_outputs_to_json(&report.file_results),
                }));
            } else {
                print_text_diagnostics(&report, no_warnings);
                print_compiled_outputs(&report);
                println!();
                if report.errors > 0 {
                    eprintln!(
                        "Build failed: {} error(s), {} warning(s) in {} file(s)",
                        report.errors, report.warnings, report.files
                    );
                } else {
                    println!(
                        "Build succeeded: {} file(s) compiled, {} warning(s)",
                        report.compiled, report.warnings
                    );
                }
            }

            if report.errors > 0 {
                process::exit(1);
            }
        }
        Commands::Check { path, json } => {
            let files = match driver::collect_prsm_files(&path) {
                Ok(files) if !files.is_empty() => files,
                Ok(_) => {
                    eprintln!("No .prsm files found in '{}'", path);
                    process::exit(1);
                }
                Err(error) => {
                    eprintln!("Error: {}", error);
                    process::exit(1);
                }
            };

            let report = driver::check_paths(&files);

            if json {
                print_json(serde_json::json!({
                    "files": report.files,
                    "errors": report.errors,
                    "warnings": report.warnings,
                    "diagnostics": report.diagnostics,
                }));
            } else {
                print_text_diagnostics(&report, false);
                if report.errors > 0 {
                    eprintln!(
                        "Check failed: {} error(s), {} warning(s) in {} file(s)",
                        report.errors, report.warnings, report.files
                    );
                } else {
                    println!(
                        "Check passed: {} file(s), {} warning(s)",
                        report.files, report.warnings
                    );
                }
            }

            if report.errors > 0 {
                process::exit(1);
            }
        }
        Commands::Build { json, watch } => {
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let build = match driver::build_project(&cwd) {
                Ok(build) => build,
                Err(error) => {
                    eprintln!("{}", error);
                    process::exit(1);
                }
            };

            println!(
                "Project: {} ({})",
                build.project_name,
                build.project_root.display()
            );

            if json {
                print_json(serde_json::json!({
                    "project": build.project_name,
                    "files": build.report.files,
                    "compiled": build.report.compiled,
                    "cached": build.report.cached,
                    "errors": build.report.errors,
                    "warnings": build.report.warnings,
                    "language_version": build.language_version,
                    "language_features": build.language_features,
                    "output_dir": build.output_dir_display,
                    "cache_dir": build.cache_dir,
                    "hir": build.hir_stats,
                    "index": build.index_stats,
                    "unity_capabilities": {
                        "input_system": build.unity_input_system,
                    },
                    "diagnostics": build.report.diagnostics,
                    "outputs": compiled_outputs_to_json(&build.report.file_results),
                }));
            } else {
                print_text_diagnostics(&build.report, false);
                print_compiled_outputs(&build.report);
                println!();
                println!("Language: {}", build.language_version);
                if !build.language_features.is_empty() {
                    println!("Features: {}", build.language_features.join(", "));
                }
                println!(
                    "HIR: {} definition(s), {} reference(s)",
                    build.hir_stats.definitions,
                    build.hir_stats.references
                );
                println!(
                    "Index: {} file(s), {} symbol(s)",
                    build.index_stats.files_indexed,
                    build.index_stats.total_symbols
                );
                if build.report.errors > 0 {
                    eprintln!(
                        "Build failed: {} error(s), {} warning(s) in {} file(s)",
                        build.report.errors, build.report.warnings, build.report.files
                    );
                } else {
                    if build.report.cached > 0 {
                        println!(
                            "Build succeeded: {} compiled, {} cached -> {}",
                            build.report.compiled,
                            build.report.cached,
                            build.output_dir_display
                        );
                    } else {
                        println!(
                            "Build succeeded: {} file(s) -> {}",
                            build.report.compiled, build.output_dir_display
                        );
                    }
                }
            }

            if build.report.errors > 0 && !watch {
                process::exit(1);
            }

            if watch {
                watch_project(build.project_root, build.output_dir);
            }
        }
        Commands::Hir { path, json } => {
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let start_path = resolve_start_path(&cwd, &path);

            if start_path.is_file() {
                let hir_file = match driver::build_hir_file(&start_path) {
                    Ok(hir_file) => hir_file,
                    Err(error) => {
                        eprintln!("{}", error);
                        process::exit(1);
                    }
                };

                if json {
                    print_json(serde_json::json!({
                        "file": hir_file.path,
                        "definitions": hir_file.definitions,
                        "references": hir_file.references,
                    }));
                } else {
                    println!("File: {}", hir_file.path.display());
                    println!("Definitions: {}", hir_file.definitions.len());
                    println!("References: {}", hir_file.references.len());
                }
            } else {
                let graph = match project_graph::ProjectGraph::discover(&start_path) {
                    Ok(graph) => graph,
                    Err(error) => {
                        eprintln!("{}", error);
                        process::exit(1);
                    }
                };
                let hir_project = driver::build_hir_project(&graph.source_files);
                let stats = hir_project.stats();

                if json {
                    print_json(serde_json::json!({
                        "project": graph.config.project.name,
                        "project_root": graph.project_root,
                        "language_version": graph.language_version.as_str(),
                        "language_features": graph.feature_names(),
                        "stats": stats,
                        "hir": hir_project,
                    }));
                } else {
                    println!(
                        "Project: {} ({})",
                        graph.config.project.name,
                        graph.project_root.display()
                    );
                    println!(
                        "HIR: {} file(s), {} definition(s), {} reference(s)",
                        stats.files_indexed,
                        stats.definitions,
                        stats.references
                    );

                    for file in &hir_project.files {
                        println!(
                            "  {} -> {} definition(s), {} reference(s)",
                            file.path.display(),
                            file.definitions.len(),
                            file.references.len()
                        );
                    }
                }
            }
        }
        Commands::Definition {
            path,
            file,
            line,
            col,
            json,
        } => {
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let start_path = resolve_start_path(&cwd, &path);
            let graph = match project_graph::ProjectGraph::discover(&start_path) {
                Ok(graph) => graph,
                Err(error) => {
                    eprintln!("{}", error);
                    process::exit(1);
                }
            };
            let query_file = resolve_query_file_path(&graph.project_root, &file);
            if !query_file.exists() {
                eprintln!("Query file does not exist: {}", query_file.display());
                process::exit(1);
            }

            let hir_project = driver::build_hir_project(&graph.source_files);
            let definition = hir_project.find_definition_for_position(&query_file, line, col);

            if json {
                print_json(serde_json::json!({
                    "project": graph.config.project.name,
                    "project_root": graph.project_root,
                    "query": {
                        "file": query_file,
                        "line": line,
                        "col": col,
                    },
                    "hir": hir_project.stats(),
                    "definition": definition.map(hir_definition_to_json),
                }));
            } else {
                match definition {
                    Some(definition) => println!(
                        "Definition: {} [{}] {}:{}:{}",
                        definition.qualified_name,
                        definition.kind.as_str(),
                        definition.file_path.display(),
                        definition.span.start.line,
                        definition.span.start.col
                    ),
                    None => println!(
                        "Definition: <none> for {}:{}:{}",
                        query_file.display(),
                        line,
                        col
                    ),
                }
            }
        }
        Commands::References {
            path,
            file,
            line,
            col,
            json,
        } => {
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let start_path = resolve_start_path(&cwd, &path);
            let graph = match project_graph::ProjectGraph::discover(&start_path) {
                Ok(graph) => graph,
                Err(error) => {
                    eprintln!("{}", error);
                    process::exit(1);
                }
            };
            let query_file = resolve_query_file_path(&graph.project_root, &file);
            if !query_file.exists() {
                eprintln!("Query file does not exist: {}", query_file.display());
                process::exit(1);
            }

            let hir_project = driver::build_hir_project(&graph.source_files);
            let references_result = hir_project.find_references_for_position(&query_file, line, col);
            let definition = references_result.as_ref().map(|(definition, _)| *definition);
            let references = references_result
                .map(|(_, references)| references)
                .unwrap_or_default();

            if json {
                print_json(serde_json::json!({
                    "project": graph.config.project.name,
                    "project_root": graph.project_root,
                    "query": {
                        "file": query_file,
                        "line": line,
                        "col": col,
                    },
                    "hir": hir_project.stats(),
                    "definition": definition.map(hir_definition_to_json),
                    "references": references.iter().map(|reference| hir_reference_to_json(reference)).collect::<Vec<_>>(),
                }));
            } else {
                match definition {
                    Some(definition) => {
                        println!(
                            "References: {} for {} [{}]",
                            references.len(),
                            definition.qualified_name,
                            definition.kind.as_str()
                        );
                        for reference in references {
                            println!(
                                "  {} [{}] {}:{}:{}",
                                reference.name,
                                reference.kind.as_str(),
                                reference.file_path.display(),
                                reference.span.start.line,
                                reference.span.start.col
                            );
                        }
                    }
                    None => println!(
                        "References: <none> for {}:{}:{}",
                        query_file.display(),
                        line,
                        col
                    ),
                }
            }
        }
        Commands::Index {
            path,
            json,
            symbol,
            qualified_name,
            file,
            line,
            col,
        } => {
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let start_path = resolve_start_path(&cwd, &path);
            let graph = match project_graph::ProjectGraph::discover(&start_path) {
                Ok(graph) => graph,
                Err(error) => {
                    eprintln!("{}", error);
                    process::exit(1);
                }
            };
            let index = project_index::build_project_index(&graph.source_files);
            let stats = index.stats();
            let query = project_index::SymbolQuery {
                name: symbol,
                qualified_name,
            };
            let matches = index.query_symbols(&query);
            let position_query = match (file, line, col) {
                (Some(file), Some(line), Some(col)) => {
                    let file_path = resolve_query_file_path(&graph.project_root, &file);
                    if !file_path.exists() {
                        eprintln!("Query file does not exist: {}", file_path.display());
                        process::exit(1);
                    }
                    Some((file_path, line, col))
                }
                _ => None,
            };
            let symbol_at = position_query
                .as_ref()
                .and_then(|(file_path, line, col)| index.find_symbol_at(file_path, *line, *col));
            let reference_at = position_query
                .as_ref()
                .and_then(|(file_path, line, col)| index.find_reference_at(file_path, *line, *col));
            let reference_target = reference_at
                .and_then(|reference| index.resolve_reference_target(reference));

            if json {
                print_json(serde_json::json!({
                    "project": graph.config.project.name,
                    "project_root": graph.project_root,
                    "language_version": graph.language_version.as_str(),
                    "language_features": graph.feature_names(),
                    "index": stats,
                    "query": {
                        "symbol": query.name,
                        "qualified_name": query.qualified_name,
                    },
                    "position_query": position_query.as_ref().map(|(file_path, line, col)| serde_json::json!({
                        "file": file_path,
                        "line": line,
                        "col": col,
                    })),
                    "symbol_at": symbol_at.map(symbol_to_json),
                    "reference_at": reference_at.map(|reference| reference_to_json(reference, reference_target)),
                    "matches": matches.iter().map(|symbol| symbol_to_json(symbol)).collect::<Vec<_>>(),
                }));
            } else {
                println!(
                    "Project: {} ({})",
                    graph.config.project.name,
                    graph.project_root.display()
                );
                println!(
                    "Index: {} file(s), {} symbol(s)",
                    stats.files_indexed,
                    stats.total_symbols
                );
                if query.name.is_some() || query.qualified_name.is_some() {
                    println!("Matches: {}", matches.len());
                }
                if let Some((file_path, line, col)) = &position_query {
                    match (symbol_at, reference_at) {
                        (Some(symbol), _) => println!(
                            "Symbol at {}:{}:{} -> {} [{}]",
                            file_path.display(),
                            line,
                            col,
                            symbol.qualified_name,
                            symbol.kind.as_str()
                        ),
                        (None, Some(reference)) => {
                            if let Some(target) = reference_target {
                                println!(
                                    "Reference at {}:{}:{} -> {} [{}] => {} [{}]",
                                    file_path.display(),
                                    line,
                                    col,
                                    reference.name,
                                    reference.kind.as_str(),
                                    target.qualified_name,
                                    target.kind.as_str()
                                );
                            } else {
                                println!(
                                    "Reference at {}:{}:{} -> {} [{}]",
                                    file_path.display(),
                                    line,
                                    col,
                                    reference.name,
                                    reference.kind.as_str()
                                );
                            }
                        }
                        (None, None) => println!(
                            "Symbol at {}:{}:{} -> <none>",
                            file_path.display(),
                            line,
                            col
                        ),
                    }
                }

                for symbol in matches {
                    println!(
                        "  {} [{}] {}:{}:{}",
                        symbol.qualified_name,
                        symbol.kind.as_str(),
                        symbol.file_path.display(),
                        symbol.span.start.line,
                        symbol.span.start.col
                    );
                }
            }
        }
        Commands::Init { name } => {
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let project_file = cwd.join(".prsmproject");
            if project_file.exists() {
                eprintln!(".prsmproject already exists");
                process::exit(1);
            }

            let project_name = name.unwrap_or_else(|| {
                cwd.file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| "MyProject".into())
            });

            let content = format!(
                r#"[project]
name = "{}"
prsm_version = "0.1.0"

[language]
version = "1.0"
features = []

[compiler]
prism_path = "prism"
output_dir = "Assets/Generated/PrSM"

[source]
include = ["Assets/Scripts/**/*.prsm"]
exclude = []

[features]
auto_compile_on_save = true
generate_meta_files = true
pascal_case_methods = true
"#,
                project_name
            );

            fs::write(&project_file, content).expect("Failed to write .prsmproject");
            println!("Created .prsmproject for '{}'", project_name);
        }
    }
}

fn resolve_start_path(cwd: &Path, value: &str) -> PathBuf {
    let path = Path::new(value);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    }
}

fn resolve_query_file_path(project_root: &Path, value: &str) -> PathBuf {
    let path = Path::new(value);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        project_root.join(path)
    }
}

fn print_json(value: serde_json::Value) {
    println!("{}", serde_json::to_string_pretty(&value).unwrap());
}

fn symbol_to_json(symbol: &project_index::IndexedSymbol) -> serde_json::Value {
    serde_json::json!({
        "name": symbol.name,
        "qualified_name": symbol.qualified_name,
        "container_name": symbol.container_name,
        "kind": symbol.kind.as_str(),
        "signature": symbol.signature,
        "file": symbol.file_path,
        "line": symbol.span.start.line,
        "col": symbol.span.start.col,
        "end_line": symbol.span.end.line,
        "end_col": symbol.span.end.col,
    })
}

fn reference_to_json(
    reference: &project_index::IndexedReference,
    resolved_symbol: Option<&project_index::IndexedSymbol>,
) -> serde_json::Value {
    serde_json::json!({
        "name": reference.name,
        "container_name": reference.container_name,
        "kind": reference.kind.as_str(),
        "file": reference.file_path,
        "line": reference.span.start.line,
        "col": reference.span.start.col,
        "end_line": reference.span.end.line,
        "end_col": reference.span.end.col,
        "target_qualified_name": reference.target_qualified_name,
        "resolved_symbol": resolved_symbol.map(symbol_to_json),
    })
}

fn hir_definition_to_json(definition: &refraction::hir::HirDefinition) -> serde_json::Value {
    serde_json::json!({
        "id": definition.id,
        "name": definition.name,
        "qualified_name": definition.qualified_name,
        "kind": definition.kind.as_str(),
        "type": definition.ty.display_name(),
        "mutable": definition.mutable,
        "file": definition.file_path,
        "line": definition.span.start.line,
        "col": definition.span.start.col,
        "end_line": definition.span.end.line,
        "end_col": definition.span.end.col,
    })
}

fn hir_reference_to_json(reference: &refraction::hir::HirReference) -> serde_json::Value {
    serde_json::json!({
        "name": reference.name,
        "kind": reference.kind.as_str(),
        "resolved_definition_id": reference.resolved_definition_id,
        "candidate_qualified_name": reference.candidate_qualified_name,
        "file": reference.file_path,
        "line": reference.span.start.line,
        "col": reference.span.start.col,
        "end_line": reference.span.end.line,
        "end_col": reference.span.end.col,
    })
}

fn print_text_diagnostics(report: &driver::DriverReport, no_warnings: bool) {
    for file_result in &report.file_results {
        let file_path = file_result.source_path.to_string_lossy();
        for diagnostic in &file_result.diagnostics {
            if no_warnings && diagnostic.severity == refraction::diagnostics::Severity::Warning {
                continue;
            }
            eprintln!("{}", driver::format_diagnostic(diagnostic, &file_path));
        }
    }
}

fn print_compiled_outputs(report: &driver::DriverReport) {
    for file_result in &report.file_results {
        if let Some(output_path) = &file_result.output_path {
            match &file_result.source_map_path {
                Some(source_map_path) => println!(
                    "  {} -> {} [{}]{}",
                    file_result.source_path.display(),
                    output_path.display(),
                    source_map_path.display(),
                    if file_result.was_cached { " [cached]" } else { "" }
                ),
                None => println!(
                    "  {} -> {}{}",
                    file_result.source_path.display(),
                    output_path.display(),
                    if file_result.was_cached { " [cached]" } else { "" }
                ),
            }
        }
    }
}

fn compiled_outputs_to_json(file_results: &[driver::FileResult]) -> Vec<serde_json::Value> {
    file_results
        .iter()
        .filter_map(|file_result| {
            file_result.output_path.as_ref().map(|output_path| {
                serde_json::json!({
                    "source": file_result.source_path,
                    "generated": output_path,
                    "source_map": file_result.source_map_path,
                    "cached": file_result.was_cached,
                })
            })
        })
        .collect()
}

fn watch_project(project_root: PathBuf, output_dir: PathBuf) {
    use notify::Watcher;
    use std::sync::mpsc;
    use std::time::Duration;

    let graph = match project_graph::ProjectGraph::discover(&project_root) {
        Ok(graph) => graph,
        Err(error) => {
            eprintln!("{}", error);
            process::exit(1);
        }
    };

    println!("\nWatching for changes... (Ctrl+C to stop)");

    let (tx, rx) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |result: Result<notify::Event, notify::Error>| {
        if let Ok(event) = result {
            if event.kind.is_modify() || event.kind.is_create() {
                let has_mn = event
                    .paths
                    .iter()
                    .any(|path| path.extension().map_or(false, |ext| ext == "mn"));
                if has_mn {
                    let _ = tx.send(());
                }
            }
        }
    })
    .expect("Failed to create file watcher");

    for watch_dir in &graph.watch_roots {
        if watch_dir.exists() {
            watcher
                .watch(watch_dir, notify::RecursiveMode::Recursive)
                .unwrap_or_else(|error| {
                    eprintln!("Watch error on {}: {}", watch_dir.display(), error)
                });
        }
    }

    loop {
        match rx.recv() {
            Ok(()) => {
                std::thread::sleep(Duration::from_millis(300));
                while rx.try_recv().is_ok() {}

                println!("\n--- Rebuilding... ---");
                let graph = match project_graph::ProjectGraph::discover(&project_root) {
                    Ok(graph) => graph,
                    Err(error) => {
                        eprintln!("{}", error);
                        continue;
                    }
                };
                let report = driver::compile_paths(&graph.source_files, Some(output_dir.as_path()));
                print_text_diagnostics(&report, false);
                if report.errors > 0 {
                    eprintln!("Rebuild: {} error(s)", report.errors);
                } else {
                    println!("Rebuild: {} file(s) compiled", report.compiled);
                }
            }
            Err(_) => break,
        }
    }
}
