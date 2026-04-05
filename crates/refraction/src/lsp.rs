use crate::diagnostics::{Diagnostic, Severity};
use crate::driver;
use crate::hir::{HirDefinition, HirDefinitionKind};
use crate::lsp_support;
use crate::project_graph::ProjectGraph;
use crate::project_index::{self, DeclarationSummary, FileSummary, IndexedReference, IndexedSymbol, MemberSummary, ProjectIndex};
use crate::roslyn_sidecar_client::{RoslynSidecarCommand, StdioRoslynSidecarClient};
use crate::roslyn_sidecar_protocol::{
    GeneratedContext, SidecarCompletionItemKind, SidecarInitializeParams, SidecarLoadProjectParams,
    UnityCompleteMembersParams, UnityCompletionItem, UnityGetHoverParams, UnityHoverResult,
    SIDECAR_PROTOCOL_VERSION,
};
use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use serde_json::{json, Value};
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process;
use url::Url;

const TEXT_DOCUMENT_PUBLISH_DIAGNOSTICS: &str = "textDocument/publishDiagnostics";
const TEXT_DOCUMENT_DID_OPEN: &str = "textDocument/didOpen";
const TEXT_DOCUMENT_DID_CHANGE: &str = "textDocument/didChange";
const TEXT_DOCUMENT_DID_SAVE: &str = "textDocument/didSave";
const TEXT_DOCUMENT_DID_CLOSE: &str = "textDocument/didClose";
const TEXT_DOCUMENT_COMPLETION: &str = "textDocument/completion";
const TEXT_DOCUMENT_DEFINITION: &str = "textDocument/definition";
const TEXT_DOCUMENT_REFERENCES: &str = "textDocument/references";
const TEXT_DOCUMENT_HOVER: &str = "textDocument/hover";
const TEXT_DOCUMENT_RENAME: &str = "textDocument/rename";
const TEXT_DOCUMENT_PREPARE_RENAME: &str = "textDocument/prepareRename";
const TEXT_DOCUMENT_DOCUMENT_SYMBOL: &str = "textDocument/documentSymbol";
const WORKSPACE_SYMBOL: &str = "workspace/symbol";

const INVALID_PARAMS: i32 = -32602;
const METHOD_NOT_FOUND: i32 = -32601;
const ROSLYN_SIDECAR_EXE_ENV: &str = "PRISM_ROSLYN_SIDECAR_EXE";
const ROSLYN_SIDECAR_ARGS_ENV: &str = "PRISM_ROSLYN_SIDECAR_ARGS";
const UNITY_MANAGED_DIR_ENV: &str = "PRISM_UNITY_MANAGED_DIR";
const UNITY_EDITOR_DIR_ENV: &str = "PRISM_UNITY_EDITOR_DIR";

pub fn run_server() -> Result<(), String> {
    let (connection, io_threads) = Connection::stdio();
    let capabilities = json!({
        "textDocumentSync": 1,
        "completionProvider": {
            "resolveProvider": false,
            "triggerCharacters": [".", ":", "<"]
        },
        "definitionProvider": true,
        "referencesProvider": true,
        "hoverProvider": true,
        "renameProvider": {
            "prepareProvider": true
        },
        "documentSymbolProvider": true,
        "workspaceSymbolProvider": true
    });

    let initialize_params = connection
        .initialize(capabilities)
        .map_err(|error| format!("Failed to initialize LSP connection: {}", error))?;

    let overlay_root = env::temp_dir()
        .join("prism-lsp")
        .join(process::id().to_string());
    fs::create_dir_all(&overlay_root)
        .map_err(|error| format!("Failed to create LSP overlay directory: {}", error))?;

    let roslyn_sidecar = match initialize_roslyn_sidecar_session() {
        Ok(session) => session,
        Err(error) => {
            eprintln!("Warning: {}", error);
            None
        }
    };

    let mut server = PrismLspServer {
        connection,
        workspace_roots: extract_workspace_roots(&initialize_params),
        open_documents: HashMap::new(),
        overlay_root,
        roslyn_sidecar,
    };

    let result = server.run();
    let cleanup_result = server.cleanup();
    drop(server);
    io_threads
        .join()
        .map_err(|error| format!("Failed to join LSP IO threads: {:?}", error))?;
    let _ = cleanup_result;
    result
}

struct PrismLspServer {
    connection: Connection,
    workspace_roots: Vec<PathBuf>,
    open_documents: HashMap<PathBuf, OpenDocument>,
    overlay_root: PathBuf,
    roslyn_sidecar: Option<RoslynSidecarSession>,
}

#[derive(Debug, Clone)]
struct OpenDocument {
    uri: String,
    version: Option<i32>,
    text: String,
    overlay_path: PathBuf,
}

#[derive(Debug, Clone)]
struct QueryContext {
    source_files: Vec<PathBuf>,
    original_to_runtime: HashMap<PathBuf, PathBuf>,
    runtime_to_original: HashMap<PathBuf, PathBuf>,
    output_dir: Option<PathBuf>,
}

#[derive(Debug, Clone)]
struct TextDocumentPosition {
    file_path: PathBuf,
    line: u32,
    col: u32,
}

#[derive(Debug, Clone)]
struct RenamePlan {
    placeholder: String,
    locations: Vec<RenameLocation>,
}

#[derive(Debug, Clone)]
struct RenameLocation {
    file_path: PathBuf,
    span: crate::lexer::token::Span,
}

#[derive(Debug, Clone)]
struct CSharpLookupTarget {
    type_name: String,
    member_name: Option<String>,
    file_path: PathBuf,
}

struct RoslynSidecarSession {
    client: StdioRoslynSidecarClient,
    loaded_project_root: Option<PathBuf>,
}

impl PrismLspServer {
    fn run(&mut self) -> Result<(), String> {
        while let Ok(message) = self.connection.receiver.recv() {
            match message {
                Message::Request(request) => {
                    if self
                        .connection
                        .handle_shutdown(&request)
                        .map_err(|error| format!("Failed to handle shutdown request: {}", error))?
                    {
                        return Ok(());
                    }
                    self.handle_request(request)?;
                }
                Message::Notification(notification) => {
                    self.handle_notification(notification)?;
                }
                Message::Response(_) => {}
            }
        }

        Ok(())
    }

    fn cleanup(&self) -> Result<(), String> {
        fs::remove_dir_all(&self.overlay_root)
            .or_else(|error| {
                if error.kind() == std::io::ErrorKind::NotFound {
                    Ok(())
                } else {
                    Err(error)
                }
            })
            .map_err(|error| format!("Failed to clean up LSP overlay directory: {}", error))
    }

    fn handle_request(&mut self, request: Request) -> Result<(), String> {
        match request.method.as_str() {
            TEXT_DOCUMENT_COMPLETION => self.handle_completion(request),
            TEXT_DOCUMENT_DEFINITION => self.handle_definition(request),
            TEXT_DOCUMENT_REFERENCES => self.handle_references(request),
            TEXT_DOCUMENT_HOVER => self.handle_hover(request),
            TEXT_DOCUMENT_RENAME => self.handle_rename(request),
            TEXT_DOCUMENT_PREPARE_RENAME => self.handle_prepare_rename(request),
            TEXT_DOCUMENT_DOCUMENT_SYMBOL => self.handle_document_symbols(request),
            WORKSPACE_SYMBOL => self.handle_workspace_symbols(request),
            _ => self.send_error(
                request.id,
                METHOD_NOT_FOUND,
                format!("Unsupported LSP method: {}", request.method),
            ),
        }
    }

    fn handle_notification(&mut self, notification: Notification) -> Result<(), String> {
        match notification.method.as_str() {
            TEXT_DOCUMENT_DID_OPEN => self.handle_did_open(notification.params),
            TEXT_DOCUMENT_DID_CHANGE => self.handle_did_change(notification.params),
            TEXT_DOCUMENT_DID_SAVE => self.handle_did_save(notification.params),
            TEXT_DOCUMENT_DID_CLOSE => self.handle_did_close(notification.params),
            _ => Ok(()),
        }
    }

    fn handle_definition(&mut self, request: Request) -> Result<(), String> {
        let position = match parse_text_document_position(&request.params) {
            Ok(position) => position,
            Err(error) => return self.send_error(request.id, INVALID_PARAMS, error),
        };

        let query_context = self.build_query_context(Some(&position.file_path));
        let runtime_path = query_context.runtime_path(&position.file_path);
        let hir_project = driver::build_hir_project(&query_context.source_files);

        if let Some(definition) = hir_project.find_definition_for_position(&runtime_path, position.line, position.col) {
            return self.send_ok(
                request.id,
                location_json(
                    &query_context.original_path(&definition.file_path),
                    definition.span,
                )?,
            );
        }

        let project_index = project_index::build_project_index(&query_context.source_files);
        let symbol_at = project_index.find_symbol_at(&runtime_path, position.line, position.col);
        let reference_at = project_index.find_reference_at(&runtime_path, position.line, position.col);
        let resolved_symbol = reference_at
            .and_then(|reference| project_index.resolve_reference_target(reference))
            .or(symbol_at);

        match resolved_symbol {
            Some(symbol) => self.send_ok(
                request.id,
                location_json(
                    &query_context.original_path(&symbol.file_path),
                    symbol.span,
                )?,
            ),
            None => self.send_ok(request.id, Value::Null),
        }
    }

    fn handle_references(&mut self, request: Request) -> Result<(), String> {
        let position = match parse_text_document_position(&request.params) {
            Ok(position) => position,
            Err(error) => return self.send_error(request.id, INVALID_PARAMS, error),
        };
        let include_declaration = request
            .params
            .get("context")
            .and_then(|context| context.get("includeDeclaration"))
            .and_then(Value::as_bool)
            .unwrap_or(false);

        let query_context = self.build_query_context(Some(&position.file_path));
        let runtime_path = query_context.runtime_path(&position.file_path);
        let hir_project = driver::build_hir_project(&query_context.source_files);
        let references_result = hir_project.find_references_for_position(&runtime_path, position.line, position.col);

        let Some((definition, references)) = references_result else {
            return self.send_ok(request.id, json!([]));
        };

        let mut seen = HashSet::new();
        let mut locations = Vec::new();

        if include_declaration {
            let key = location_key(&query_context.original_path(&definition.file_path), definition.span);
            seen.insert(key);
            locations.push(location_json(
                &query_context.original_path(&definition.file_path),
                definition.span,
            )?);
        }

        for reference in references {
            let original_path = query_context.original_path(&reference.file_path);
            let key = location_key(&original_path, reference.span);
            if !seen.insert(key) {
                continue;
            }
            locations.push(location_json(&original_path, reference.span)?);
        }

        self.send_ok(request.id, Value::Array(locations))
    }

    fn handle_completion(&mut self, request: Request) -> Result<(), String> {
        let position = match parse_text_document_position(&request.params) {
            Ok(position) => position,
            Err(error) => return self.send_error(request.id, INVALID_PARAMS, error),
        };

        let Some(document_text) = self.read_document_text(&position.file_path) else {
            return self.send_ok(request.id, json!({ "isIncomplete": false, "items": [] }));
        };

        let query_context = self.build_query_context(Some(&position.file_path));
        let runtime_path = query_context.runtime_path(&position.file_path);
        let project_index = project_index::build_project_index(&query_context.source_files);
        let hir_project = driver::build_hir_project(&query_context.source_files);
        let fallback_items = lsp_support::completion_items(
            &document_text,
            position.line,
            position.col,
            &runtime_path,
            &project_index,
            &hir_project,
        );
        let items = self
            .sidecar_completion_items(
                &position.file_path,
                &document_text,
                position.line,
                position.col,
                &runtime_path,
                &project_index,
                &hir_project,
            )
            .map(|sidecar_items| merge_completion_items(sidecar_items, fallback_items.clone()))
            .unwrap_or(fallback_items);

        self.send_ok(request.id, json!({
            "isIncomplete": false,
            "items": items,
        }))
    }

    fn handle_hover(&mut self, request: Request) -> Result<(), String> {
        let position = match parse_text_document_position(&request.params) {
            Ok(position) => position,
            Err(error) => return self.send_error(request.id, INVALID_PARAMS, error),
        };

        let query_context = self.build_query_context(Some(&position.file_path));
        let runtime_path = query_context.runtime_path(&position.file_path);
        let project_index = project_index::build_project_index(&query_context.source_files);
        let hir_project = driver::build_hir_project(&query_context.source_files);
        let symbol_at = project_index.find_symbol_at(&runtime_path, position.line, position.col);
        let reference_at = project_index.find_reference_at(&runtime_path, position.line, position.col);
        let resolved_symbol = reference_at.and_then(|reference| project_index.resolve_reference_target(reference));

        let definition = hir_project
            .find_definition_for_position(&runtime_path, position.line, position.col)
            .or_else(|| {
                resolved_symbol.and_then(|symbol| {
                    hir_project.find_definition_by_qualified_name(&symbol.qualified_name)
                })
            })
            .or_else(|| {
                symbol_at.and_then(|symbol| {
                    hir_project.find_definition_by_qualified_name(&symbol.qualified_name)
                })
            });

        let hover_markdown = build_hover_markdown(
            symbol_at,
            reference_at,
            resolved_symbol,
            definition,
            &project_index,
            &query_context,
            self.sidecar_hover_section(
                &position.file_path,
                symbol_at,
                reference_at,
                resolved_symbol,
                definition,
                &project_index,
                &query_context,
            ),
        );
        let hover_range = symbol_at
            .map(|symbol| symbol.span)
            .or_else(|| reference_at.map(|reference| reference.span));

        match (hover_markdown, hover_range) {
            (Some(markdown), Some(range)) => self.send_ok(
                request.id,
                json!({
                    "contents": {
                        "kind": "markdown",
                        "value": markdown,
                    },
                    "range": lsp_range_json(range),
                }),
            ),
            _ => self.send_ok(request.id, Value::Null),
        }
    }

    fn handle_prepare_rename(&mut self, request: Request) -> Result<(), String> {
        let position = match parse_text_document_position(&request.params) {
            Ok(position) => position,
            Err(error) => return self.send_error(request.id, INVALID_PARAMS, error),
        };

        match self.build_rename_plan(&position.file_path, position.line, position.col) {
            Ok(plan) => {
                let target = plan.locations.first().cloned().ok_or_else(|| {
                    "Only PrSM symbols defined in the current project can be renamed.".to_string()
                })?;
                self.send_ok(
                    request.id,
                    json!({
                        "range": lsp_range_json(target.span),
                        "placeholder": plan.placeholder,
                    }),
                )
            }
            Err(error) => self.send_error(request.id, INVALID_PARAMS, error),
        }
    }

    fn handle_rename(&mut self, request: Request) -> Result<(), String> {
        let position = match parse_text_document_position(&request.params) {
            Ok(position) => position,
            Err(error) => return self.send_error(request.id, INVALID_PARAMS, error),
        };
        let new_name = match request.params.get("newName").and_then(Value::as_str) {
            Some(name) => name.to_string(),
            None => return self.send_error(request.id, INVALID_PARAMS, "Missing rename target."),
        };

        if let Some(error) = validate_rename_target(&new_name) {
            return self.send_error(request.id, INVALID_PARAMS, error);
        }

        let plan = match self.build_rename_plan(&position.file_path, position.line, position.col) {
            Ok(plan) => plan,
            Err(error) => return self.send_error(request.id, INVALID_PARAMS, error),
        };

        let mut changes: HashMap<String, Vec<Value>> = HashMap::new();
        for location in plan.locations {
            let uri = path_to_uri_string(&location.file_path)?;
            changes.entry(uri).or_default().push(json!({
                "range": lsp_range_json(location.span),
                "newText": new_name,
            }));
        }

        self.send_ok(request.id, json!({ "changes": changes }))
    }

    fn handle_document_symbols(&mut self, request: Request) -> Result<(), String> {
        let uri = match request
            .params
            .get("textDocument")
            .and_then(|text_document| text_document.get("uri"))
            .and_then(Value::as_str)
        {
            Some(uri) => uri.to_string(),
            None => return self.send_error(request.id, INVALID_PARAMS, "Missing text document URI."),
        };
        let file_path = match file_uri_to_path(&uri) {
            Some(path) => path,
            None => return self.send_error(request.id, INVALID_PARAMS, format!("Invalid file URI: {}", uri)),
        };

        let query_context = self.build_query_context(Some(&file_path));
        let runtime_path = query_context.runtime_path(&file_path);
        let project_index = project_index::build_project_index(&query_context.source_files);
        let mut symbols = project_index
            .query_symbols(&project_index::SymbolQuery::default())
            .into_iter()
            .filter(|symbol| query_context.original_path(&symbol.file_path) == normalize_path(&file_path))
            .collect::<Vec<_>>();
        symbols.sort_by(|left, right| compare_symbols(left, right));

        let document_symbols = build_document_symbols(&project_index, &symbols, &runtime_path, &query_context);
        self.send_ok(request.id, Value::Array(document_symbols))
    }

    fn handle_workspace_symbols(&mut self, request: Request) -> Result<(), String> {
        let query = request
            .params
            .get("query")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();

        let Some(query_context) = self.build_workspace_query_context() else {
            return self.send_ok(request.id, json!([]));
        };
        let project_index = project_index::build_project_index(&query_context.source_files);
        let mut symbols = project_index
            .query_symbols(&project_index::SymbolQuery::default())
            .into_iter()
            .filter_map(|symbol| workspace_symbol_score(symbol, &query).map(|score| (symbol, score)))
            .collect::<Vec<_>>();

        symbols.sort_by(|left, right| {
            left.1
                .cmp(&right.1)
                .then_with(|| compare_symbols(left.0, right.0))
        });

        let response = symbols
            .into_iter()
            .take(200)
            .map(|(symbol, _)| {
                let file_path = query_context.original_path(&symbol.file_path);
                Ok(json!({
                    "name": symbol.name,
                    "kind": symbol_kind_number(symbol.kind.as_str()),
                    "location": {
                        "uri": path_to_uri_string(&file_path)?,
                        "range": lsp_range_json(symbol.span),
                    },
                    "containerName": workspace_symbol_container_name(symbol, &file_path),
                }))
            })
            .collect::<Result<Vec<_>, String>>()?;

        self.send_ok(request.id, Value::Array(response))
    }

    fn handle_did_open(&mut self, params: Value) -> Result<(), String> {
        let document = params
            .get("textDocument")
            .ok_or_else(|| "Missing didOpen textDocument payload.".to_string())?;
        let uri = document
            .get("uri")
            .and_then(Value::as_str)
            .ok_or_else(|| "Missing didOpen textDocument URI.".to_string())?;
        let text = document
            .get("text")
            .and_then(Value::as_str)
            .ok_or_else(|| "Missing didOpen textDocument text.".to_string())?;
        let version = document
            .get("version")
            .and_then(Value::as_i64)
            .map(|value| value as i32);

        self.upsert_open_document(uri, version, text.to_string())?;
        if let Some(file_path) = file_uri_to_path(uri) {
            self.publish_diagnostics(&normalize_path(&file_path))?;
            self.invalidate_roslyn_sidecar_project();
        }
        Ok(())
    }

    fn handle_did_change(&mut self, params: Value) -> Result<(), String> {
        let document = params
            .get("textDocument")
            .ok_or_else(|| "Missing didChange textDocument payload.".to_string())?;
        let uri = document
            .get("uri")
            .and_then(Value::as_str)
            .ok_or_else(|| "Missing didChange textDocument URI.".to_string())?;
        let version = document
            .get("version")
            .and_then(Value::as_i64)
            .map(|value| value as i32);
        let text = params
            .get("contentChanges")
            .and_then(Value::as_array)
            .and_then(|changes| changes.last())
            .and_then(|change| change.get("text"))
            .and_then(Value::as_str)
            .ok_or_else(|| "Missing didChange contentChanges text.".to_string())?;

        self.upsert_open_document(uri, version, text.to_string())?;
        if let Some(file_path) = file_uri_to_path(uri) {
            self.publish_diagnostics(&normalize_path(&file_path))?;
        }
        Ok(())
    }

    fn handle_did_save(&mut self, params: Value) -> Result<(), String> {
        let uri = params
            .get("textDocument")
            .and_then(|text_document| text_document.get("uri"))
            .and_then(Value::as_str)
            .ok_or_else(|| "Missing didSave textDocument URI.".to_string())?;

        if let Some(file_path) = file_uri_to_path(uri) {
            self.publish_diagnostics(&normalize_path(&file_path))?;
        }
        Ok(())
    }

    fn handle_did_close(&mut self, params: Value) -> Result<(), String> {
        let uri = params
            .get("textDocument")
            .and_then(|text_document| text_document.get("uri"))
            .and_then(Value::as_str)
            .ok_or_else(|| "Missing didClose textDocument URI.".to_string())?;
        let Some(file_path) = file_uri_to_path(uri) else {
            return Ok(());
        };
        let normalized_path = normalize_path(&file_path);
        if let Some(document) = self.open_documents.remove(&normalized_path) {
            let _ = fs::remove_file(document.overlay_path);
        }
        self.invalidate_roslyn_sidecar_project();
        self.publish_diagnostics_notification(uri, None, Vec::new())
    }

    fn build_query_context(&self, preferred_file: Option<&Path>) -> QueryContext {
        if let Some(file_path) = preferred_file {
            if let Ok(graph) = ProjectGraph::discover(file_path) {
                return QueryContext::from_project_graph(&graph, self, Some(file_path));
            }
        }

        if let Some(file_path) = preferred_file {
            return QueryContext::from_source_files(&[file_path.to_path_buf()], self, Some(file_path));
        }

        QueryContext::default()
    }

    fn build_workspace_query_context(&self) -> Option<QueryContext> {
        for workspace_root in &self.workspace_roots {
            if let Ok(graph) = ProjectGraph::discover(workspace_root) {
                return Some(QueryContext::from_project_graph(&graph, self, None));
            }
        }

        self.open_documents
            .keys()
            .next()
            .map(|path| QueryContext::from_source_files(&[path.clone()], self, Some(path)))
    }

    fn build_rename_plan(&self, file_path: &Path, line: u32, col: u32) -> Result<RenamePlan, String> {
        let query_context = self.build_query_context(Some(file_path));
        let runtime_path = query_context.runtime_path(file_path);
        let hir_project = driver::build_hir_project(&query_context.source_files);
        let Some((definition, references)) = hir_project.find_references_for_position(&runtime_path, line, col) else {
            return Err("Only PrSM symbols defined in the current project can be renamed.".into());
        };

        if definition.kind == HirDefinitionKind::Lifecycle {
            return Err("Lifecycle blocks map to fixed Unity callback names and cannot be renamed.".into());
        }

        let mut seen = HashSet::new();
        let mut locations = Vec::new();
        push_rename_location(
            &mut seen,
            &mut locations,
            query_context.original_path(&definition.file_path),
            definition.span,
        );
        for reference in references {
            push_rename_location(
                &mut seen,
                &mut locations,
                query_context.original_path(&reference.file_path),
                reference.span,
            );
        }

        Ok(RenamePlan {
            placeholder: definition.name.clone(),
            locations,
        })
    }

    fn read_document_text(&self, file_path: &Path) -> Option<String> {
        let normalized_path = normalize_path(file_path);
        self.open_documents
            .get(&normalized_path)
            .map(|document| document.text.clone())
            .or_else(|| fs::read_to_string(&normalized_path).ok())
    }

    fn sidecar_completion_items(
        &mut self,
        file_path: &Path,
        document_text: &str,
        line: u32,
        col: u32,
        runtime_path: &Path,
        project_index: &ProjectIndex,
        hir_project: &crate::hir::HirProject,
    ) -> Option<Vec<Value>> {
        let query = lsp_support::sidecar_completion_query(
            document_text,
            line,
            col,
            runtime_path,
            project_index,
            hir_project,
        )?;
        self.ensure_roslyn_sidecar_loaded(file_path).ok()?;
        let session = self.roslyn_sidecar.as_mut()?;
        let result = session
            .client
            .complete_members(UnityCompleteMembersParams {
                type_name: query.type_name,
                prefix: query.prefix,
                context: None,
                include_instance_members: query.include_instance_members,
                include_static_members: query.include_static_members,
            })
            .ok()?;
        Some(sidecar_completion_items_json(result.items))
    }

    fn sidecar_hover_section(
        &mut self,
        file_path: &Path,
        symbol_at: Option<&IndexedSymbol>,
        reference_at: Option<&IndexedReference>,
        resolved_symbol: Option<&IndexedSymbol>,
        definition: Option<&HirDefinition>,
        project_index: &ProjectIndex,
        query_context: &QueryContext,
    ) -> Option<String> {
        if let Some(symbol) = symbol_at {
            if let Some(section) = self.sidecar_generated_hover_section_for_symbol(file_path, symbol, query_context) {
                return Some(section);
            }
        }

        if let Some(symbol) = resolved_symbol {
            if let Some(section) = self.sidecar_generated_hover_section_for_symbol(file_path, symbol, query_context) {
                return Some(section);
            }
        }

        if let Some(definition) = definition {
            if let Some(section) = self.sidecar_generated_hover_section_for_definition(file_path, definition, query_context) {
                return Some(section);
            }
            if let Some(section) = self.sidecar_unity_hover_section_for_definition(file_path, definition, project_index) {
                return Some(section);
            }
        }

        reference_at.and_then(|reference| self.sidecar_unity_hover_section_for_reference(file_path, reference))
    }

    fn sidecar_generated_hover_section_for_symbol(
        &mut self,
        file_path: &Path,
        symbol: &IndexedSymbol,
        query_context: &QueryContext,
    ) -> Option<String> {
        let target = csharp_lookup_target_from_symbol(symbol)?;
        let context = generated_context_for_target(&target, query_context);
        let hover = self.request_sidecar_hover(
            file_path,
            UnityGetHoverParams {
                type_name: target.type_name,
                member_name: target.member_name,
                context,
            },
        )?;
        Some(format_sidecar_hover_section("C# Symbol", &hover))
    }

    fn sidecar_generated_hover_section_for_definition(
        &mut self,
        file_path: &Path,
        definition: &HirDefinition,
        query_context: &QueryContext,
    ) -> Option<String> {
        let target = csharp_lookup_target_from_definition(definition)?;
        let context = generated_context_for_target(&target, query_context);
        let hover = self.request_sidecar_hover(
            file_path,
            UnityGetHoverParams {
                type_name: target.type_name,
                member_name: target.member_name,
                context,
            },
        )?;
        Some(format_sidecar_hover_section("C# Symbol", &hover))
    }

    fn sidecar_unity_hover_section_for_definition(
        &mut self,
        file_path: &Path,
        definition: &HirDefinition,
        project_index: &ProjectIndex,
    ) -> Option<String> {
        let type_name = display_type_name(&definition.ty.display_name());
        if !should_query_sidecar_for_type(&type_name, project_index) {
            return None;
        }

        let hover = self.request_sidecar_hover(
            file_path,
            UnityGetHoverParams {
                type_name,
                member_name: None,
                context: None,
            },
        )?;
        Some(format_sidecar_hover_section("Unity API", &hover))
    }

    fn sidecar_unity_hover_section_for_reference(
        &mut self,
        file_path: &Path,
        reference: &IndexedReference,
    ) -> Option<String> {
        if reference.kind.as_str() != "type" || !lsp_support::core_type_is_unity(&reference.name) {
            return None;
        }

        let hover = self.request_sidecar_hover(
            file_path,
            UnityGetHoverParams {
                type_name: display_type_name(&reference.name),
                member_name: None,
                context: None,
            },
        )?;
        Some(format_sidecar_hover_section("Unity API", &hover))
    }

    fn request_sidecar_hover(
        &mut self,
        file_path: &Path,
        params: UnityGetHoverParams,
    ) -> Option<UnityHoverResult> {
        self.ensure_roslyn_sidecar_loaded(file_path).ok()?;
        let session = self.roslyn_sidecar.as_mut()?;
        session.client.get_hover(params).ok()
    }

    fn ensure_roslyn_sidecar_loaded(&mut self, file_path: &Path) -> Result<(), String> {
        let Some(sidecar) = self.roslyn_sidecar.as_mut() else {
            return Ok(());
        };

        let graph = match ProjectGraph::discover(file_path) {
            Ok(graph) => graph,
            Err(_) => return Ok(()),
        };
        let project_root = normalize_path(&graph.project_root);
        if sidecar
            .loaded_project_root
            .as_ref()
            .map(|loaded| loaded == &project_root)
            .unwrap_or(false)
        {
            return Ok(());
        }

        sidecar
            .client
            .load_project(SidecarLoadProjectParams {
                workspace_root: project_root.clone(),
                project_file: active_project_file(&graph.project_root),
                unity_project_root: project_root.clone(),
                output_dir: Some(normalize_path(&graph.output_dir)),
                generated_files: collect_generated_csharp_files(&graph.output_dir),
                metadata_references: discover_unity_metadata_references(&graph),
                package_assemblies: discover_script_assemblies(&graph.project_root),
            })
            .map_err(|error| format!("Failed to load Roslyn sidecar project: {}", error))?;
        sidecar.loaded_project_root = Some(project_root);
        Ok(())
    }

    fn invalidate_roslyn_sidecar_project(&mut self) {
        if let Some(sidecar) = self.roslyn_sidecar.as_mut() {
            sidecar.loaded_project_root = None;
        }
    }

    fn upsert_open_document(&mut self, uri: &str, version: Option<i32>, text: String) -> Result<(), String> {
        let file_path = file_uri_to_path(uri)
            .ok_or_else(|| format!("Invalid file URI: {}", uri))?;
        let normalized_path = normalize_path(&file_path);
        let overlay_path = overlay_path_for(&self.overlay_root, &normalized_path);
        if let Some(parent) = overlay_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("Failed to create overlay parent directory: {}", error))?;
        }
        fs::write(&overlay_path, &text)
            .map_err(|error| format!("Failed to write overlay file {}: {}", overlay_path.display(), error))?;

        self.open_documents.insert(
            normalized_path,
            OpenDocument {
                uri: uri.to_string(),
                version,
                text,
                overlay_path,
            },
        );

        Ok(())
    }

    fn publish_diagnostics(&self, file_path: &Path) -> Result<(), String> {
        let normalized_path = normalize_path(file_path);
        let (uri, version, text, check_path) = if let Some(document) = self.open_documents.get(&normalized_path) {
            (
                document.uri.clone(),
                document.version,
                document.text.clone(),
                document.overlay_path.clone(),
            )
        } else {
            let uri = path_to_uri_string(&normalized_path)?;
            let text = fs::read_to_string(&normalized_path).unwrap_or_default();
            (uri, None, text, normalized_path)
        };

        let report = driver::check_paths(&[check_path]);
        let line_lengths = text.lines().map(|line| line.len()).collect::<Vec<_>>();
        let diagnostics = report
            .file_results
            .first()
            .map(|file_result| {
                file_result
                    .diagnostics
                    .iter()
                    .map(|diagnostic| diagnostic_json(diagnostic, &line_lengths))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        self.publish_diagnostics_notification(&uri, version, diagnostics)
    }

    fn publish_diagnostics_notification(
        &self,
        uri: &str,
        version: Option<i32>,
        diagnostics: Vec<Value>,
    ) -> Result<(), String> {
        let mut params = json!({
            "uri": uri,
            "diagnostics": diagnostics,
        });
        if let Some(version) = version {
            params["version"] = json!(version);
        }
        self.send_notification(TEXT_DOCUMENT_PUBLISH_DIAGNOSTICS, params)
    }

    fn send_ok(&self, id: RequestId, result: Value) -> Result<(), String> {
        self.connection
            .sender
            .send(Message::Response(Response::new_ok(id, result)))
            .map_err(|error| format!("Failed to send LSP response: {}", error))
    }

    fn send_error(&self, id: RequestId, code: i32, message: impl Into<String>) -> Result<(), String> {
        self.connection
            .sender
            .send(Message::Response(Response::new_err(id, code, message.into())))
            .map_err(|error| format!("Failed to send LSP error response: {}", error))
    }

    fn send_notification(&self, method: &str, params: Value) -> Result<(), String> {
        self.connection
            .sender
            .send(Message::Notification(Notification::new(method.to_string(), params)))
            .map_err(|error| format!("Failed to send LSP notification: {}", error))
    }
}

impl Default for QueryContext {
    fn default() -> Self {
        Self {
            source_files: Vec::new(),
            original_to_runtime: HashMap::new(),
            runtime_to_original: HashMap::new(),
            output_dir: None,
        }
    }
}

impl QueryContext {
    fn from_project_graph(
        graph: &ProjectGraph,
        server: &PrismLspServer,
        preferred_file: Option<&Path>,
    ) -> Self {
        let mut context = Self::from_source_files(&graph.source_files, server, preferred_file);
        context.output_dir = Some(normalize_path(&graph.output_dir));
        context
    }

    fn from_source_files(
        source_files: &[PathBuf],
        server: &PrismLspServer,
        preferred_file: Option<&Path>,
    ) -> Self {
        let mut context = Self::default();
        let mut seen = HashSet::new();

        for source_file in source_files {
            context.push_source_file(source_file, server, &mut seen);
        }

        if let Some(preferred_file) = preferred_file {
            context.push_source_file(preferred_file, server, &mut seen);
        }

        context
    }

    fn push_source_file(&mut self, source_file: &Path, server: &PrismLspServer, seen: &mut HashSet<PathBuf>) {
        let original_path = normalize_path(source_file);
        if !seen.insert(original_path.clone()) {
            return;
        }

        let runtime_path = server
            .open_documents
            .get(&original_path)
            .map(|document| document.overlay_path.clone())
            .unwrap_or_else(|| original_path.clone());

        self.original_to_runtime
            .insert(original_path.clone(), runtime_path.clone());
        self.runtime_to_original
            .insert(runtime_path.clone(), original_path);
        self.source_files.push(runtime_path);
    }

    fn runtime_path(&self, original_path: &Path) -> PathBuf {
        self.original_to_runtime
            .get(&normalize_path(original_path))
            .cloned()
            .unwrap_or_else(|| normalize_path(original_path))
    }

    fn original_path(&self, runtime_path: &Path) -> PathBuf {
        self.runtime_to_original
            .get(&normalize_path(runtime_path))
            .cloned()
            .unwrap_or_else(|| normalize_path(runtime_path))
    }
}

fn initialize_roslyn_sidecar_session() -> Result<Option<RoslynSidecarSession>, String> {
    let Some(command) = configured_roslyn_sidecar_command()? else {
        return Ok(None);
    };

    let mut client = command
        .spawn()
        .map_err(|error| format!("Failed to start Roslyn sidecar: {}", error))?;
    client
        .initialize(SidecarInitializeParams {
            protocol_version: SIDECAR_PROTOCOL_VERSION,
            client_name: "prism-lsp".to_string(),
            client_version: Some(env!("CARGO_PKG_VERSION").to_string()),
        })
        .map_err(|error| format!("Failed to initialize Roslyn sidecar: {}", error))?;

    Ok(Some(RoslynSidecarSession {
        client,
        loaded_project_root: None,
    }))
}

fn configured_roslyn_sidecar_command() -> Result<Option<RoslynSidecarCommand>, String> {
    let Some(program) = env::var_os(ROSLYN_SIDECAR_EXE_ENV) else {
        return Ok(None);
    };

    let mut command = RoslynSidecarCommand::new(program);
    if let Some(raw_args) = env::var_os(ROSLYN_SIDECAR_ARGS_ENV) {
        let args = parse_sidecar_args(&raw_args.to_string_lossy())?;
        command = command.args(args);
    }

    Ok(Some(command))
}

fn parse_sidecar_args(raw_args: &str) -> Result<Vec<String>, String> {
    let trimmed = raw_args.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    if trimmed.starts_with('[') {
        return serde_json::from_str(trimmed)
            .map_err(|error| format!("Invalid {} JSON array: {}", ROSLYN_SIDECAR_ARGS_ENV, error));
    }

    Ok(trimmed.split_whitespace().map(str::to_string).collect())
}

fn active_project_file(project_root: &Path) -> Option<PathBuf> {
    let prsm = project_root.join(".prsmproject");
    if prsm.exists() {
        return Some(normalize_path(&prsm));
    }

    let legacy = project_root.join(".mnproject");
    legacy.exists().then(|| normalize_path(&legacy))
}

fn collect_generated_csharp_files(output_dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_generated_csharp_files_recursive(output_dir, &mut files);
    files.sort();
    files.dedup();
    files
}

fn collect_generated_csharp_files_recursive(directory: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_generated_csharp_files_recursive(&path, files);
            continue;
        }

        if path
            .extension()
            .map(|extension| extension.eq_ignore_ascii_case("cs"))
            .unwrap_or(false)
        {
            files.push(normalize_path(&path));
        }
    }
}

fn discover_unity_metadata_references(graph: &ProjectGraph) -> Vec<PathBuf> {
    let mut references = Vec::new();
    let mut seen = HashSet::new();

    for directory in candidate_unity_managed_directories(graph) {
        collect_dlls_from_directory(&directory, &mut references, &mut seen);
    }

    references
}

fn candidate_unity_managed_directories(graph: &ProjectGraph) -> Vec<PathBuf> {
    let mut directories = Vec::new();

    if let Some(path) = env::var_os(UNITY_MANAGED_DIR_ENV) {
        directories.push(normalize_path(Path::new(&path)));
    }

    if let Some(path) = env::var_os(UNITY_EDITOR_DIR_ENV) {
        let editor_dir = normalize_path(Path::new(&path));
        directories.push(editor_dir.join("Data").join("Managed"));
        directories.push(editor_dir.join("Data").join("Managed").join("UnityEngine"));
        directories.push(editor_dir.join("Managed"));
        directories.push(editor_dir.join("Managed").join("UnityEngine"));
    }

    if !graph.config.compiler.target_unity.trim().is_empty() {
        let base = normalize_path(Path::new(graph.config.compiler.target_unity.trim()));
        directories.push(base.clone());
        directories.push(base.join("Data").join("Managed"));
        directories.push(base.join("Data").join("Managed").join("UnityEngine"));
        directories.push(base.join("Editor").join("Data").join("Managed"));
        directories.push(base.join("Editor").join("Data").join("Managed").join("UnityEngine"));
    }

    directories.sort();
    directories.dedup();
    directories
}

fn discover_script_assemblies(project_root: &Path) -> Vec<PathBuf> {
    let script_assemblies_dir = project_root.join("Library").join("ScriptAssemblies");
    let mut assemblies = Vec::new();
    let mut seen = HashSet::new();
    collect_dlls_from_directory(&script_assemblies_dir, &mut assemblies, &mut seen);
    assemblies
}

fn collect_dlls_from_directory(directory: &Path, files: &mut Vec<PathBuf>, seen: &mut HashSet<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_dlls_from_directory(&path, files, seen);
            continue;
        }

        if !path
            .extension()
            .map(|extension| extension.eq_ignore_ascii_case("dll"))
            .unwrap_or(false)
        {
            continue;
        }

        let normalized = normalize_path(&path);
        if seen.insert(normalized.clone()) {
            files.push(normalized);
        }
    }
}

fn extract_workspace_roots(params: &Value) -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Some(workspace_folders) = params.get("workspaceFolders").and_then(Value::as_array) {
        for folder in workspace_folders {
            if let Some(uri) = folder.get("uri").and_then(Value::as_str) {
                if let Some(path) = file_uri_to_path(uri) {
                    roots.push(normalize_path(&path));
                }
            }
        }
    }

    if roots.is_empty() {
        if let Some(root_uri) = params.get("rootUri").and_then(Value::as_str) {
            if let Some(path) = file_uri_to_path(root_uri) {
                roots.push(normalize_path(&path));
            }
        }
    }

    if roots.is_empty() {
        if let Some(root_path) = params.get("rootPath").and_then(Value::as_str) {
            roots.push(normalize_path(Path::new(root_path)));
        }
    }

    roots.sort();
    roots.dedup();
    roots
}

fn parse_text_document_position(params: &Value) -> Result<TextDocumentPosition, String> {
    let uri = params
        .get("textDocument")
        .and_then(|text_document| text_document.get("uri"))
        .and_then(Value::as_str)
        .ok_or_else(|| "Missing text document URI.".to_string())?
        .to_string();
    let file_path = file_uri_to_path(&uri).ok_or_else(|| format!("Invalid file URI: {}", uri))?;
    let line = params
        .get("position")
        .and_then(|position| position.get("line"))
        .and_then(Value::as_u64)
        .map(|value| value as u32 + 1)
        .ok_or_else(|| "Missing text document line position.".to_string())?;
    let col = params
        .get("position")
        .and_then(|position| position.get("character"))
        .and_then(Value::as_u64)
        .map(|value| value as u32 + 1)
        .ok_or_else(|| "Missing text document column position.".to_string())?;

    Ok(TextDocumentPosition { file_path: normalize_path(&file_path), line, col })
}

fn build_hover_markdown(
    symbol_at: Option<&IndexedSymbol>,
    reference_at: Option<&IndexedReference>,
    resolved_symbol: Option<&IndexedSymbol>,
    definition: Option<&HirDefinition>,
    project_index: &ProjectIndex,
    query_context: &QueryContext,
    sidecar_hover_section: Option<String>,
) -> Option<String> {
    if let Some(symbol) = symbol_at {
        let mut details = vec!["**Status:** Defined".to_string()];
        details.push(format!(
            "**Definition:** {}",
            format_location(&query_context.original_path(&symbol.file_path), symbol.span)
        ));
        if let Some(definition) = definition {
            details.push(format!("**Type:** {}", definition.ty.display_name()));
            if shows_mutability(definition.kind) {
                details.push(format!(
                    "**Mutable:** {}",
                    if definition.mutable { "yes" } else { "no" }
                ));
            }
        }
        details.extend(symbol_hover_metadata(symbol, project_index));

        let mut sections = vec![
            format!("**{}** {}", symbol.kind.as_str(), symbol.qualified_name),
            details.join("  \n"),
            "```prsm".to_string(),
            symbol.signature.clone(),
            "```".to_string(),
        ];
        if let Some(section) = generated_csharp_hover_section_for_symbol(symbol, query_context) {
            sections.push(section);
        }
        if let Some(section) = sidecar_hover_section
            .clone()
            .or_else(|| unity_hover_section_for_symbol(symbol, definition, project_index))
        {
            sections.push(section);
        }

        return Some(sections.join("\n\n"));
    }

    if let (Some(reference), Some(symbol)) = (reference_at, resolved_symbol) {
        let mut details = vec![
            "**Status:** Resolved".to_string(),
            format!("**Target:** {} {}", symbol.kind.as_str(), symbol.qualified_name),
            format!(
                "**Definition:** {}",
                format_location(&query_context.original_path(&symbol.file_path), symbol.span)
            ),
        ];
        if let Some(definition) = definition {
            details.push(format!("**Type:** {}", definition.ty.display_name()));
        }
        details.extend(symbol_hover_metadata(symbol, project_index));

        let mut sections = vec![
            format!("**{} reference** {}", reference.kind.as_str(), reference.name),
            details.join("  \n"),
            "```prsm".to_string(),
            symbol.signature.clone(),
            "```".to_string(),
        ];
        if let Some(section) = generated_csharp_hover_section_for_symbol(symbol, query_context) {
            sections.push(section);
        }
        if let Some(section) = sidecar_hover_section
            .clone()
            .or_else(|| unity_hover_section_for_symbol(symbol, definition, project_index))
            .or_else(|| unity_hover_section_for_reference(reference))
        {
            sections.push(section);
        }

        return Some(sections.join("\n\n"));
    }

    if let Some(definition) = definition {
        let mut details = vec![
            "**Status:** Resolved".to_string(),
            format!(
                "**Definition:** {}",
                format_location(&query_context.original_path(&definition.file_path), definition.span)
            ),
            format!("**Type:** {}", definition.ty.display_name()),
        ];
        if shows_mutability(definition.kind) {
            details.push(format!(
                "**Mutable:** {}",
                if definition.mutable { "yes" } else { "no" }
            ));
        }

        let mut sections = vec![
            format!("**{}** {}", definition.kind.as_str(), definition.qualified_name),
            details.join("  \n"),
        ];
        if let Some(section) = generated_csharp_hover_section_for_definition(definition, query_context) {
            sections.push(section);
        }
        if let Some(section) = sidecar_hover_section
            .clone()
            .or_else(|| unity_hover_section_for_definition(definition))
        {
            sections.push(section);
        }

        return Some(sections.join("\n\n"));
    }

    reference_at.map(|reference| {
        let mut sections = vec![
            format!("**{} reference** {}", reference.kind.as_str(), reference.name),
            "**Status:** Unresolved  \n**Definition:** Not found in the current PrSM project index.".to_string(),
        ];
        if let Some(section) = sidecar_hover_section.or_else(|| unity_hover_section_for_reference(reference)) {
            sections.push(section);
        }
        sections.join("\n\n")
    })
}

fn merge_completion_items(primary: Vec<Value>, fallback: Vec<Value>) -> Vec<Value> {
    let mut merged = Vec::new();
    let mut seen = HashSet::new();

    for item in primary.into_iter().chain(fallback) {
        let Some(label) = item.get("label").and_then(Value::as_str) else {
            continue;
        };
        let key = label.to_ascii_lowercase();
        if seen.insert(key) {
            merged.push(item);
        }
    }

    merged
}

fn sidecar_completion_items_json(items: Vec<UnityCompletionItem>) -> Vec<Value> {
    items.into_iter().map(sidecar_completion_item_json).collect()
}

fn sidecar_completion_item_json(item: UnityCompletionItem) -> Value {
    let label = prsm_label_for_sidecar_item(&item);
    let mut value = serde_json::Map::new();
    value.insert("label".into(), json!(label.clone()));
    value.insert("kind".into(), json!(sidecar_completion_item_kind_number(item.kind)));

    if let Some(detail) = item.detail.as_ref() {
        value.insert("detail".into(), json!(detail));
    } else if let Some(signature) = item.signature.as_ref() {
        value.insert("detail".into(), json!(signature));
    }

    let documentation = match (item.signature.as_ref(), item.documentation.as_ref()) {
        (Some(signature), Some(documentation)) => Some(format!("```csharp\n{}\n```\n\n{}", signature, documentation)),
        (Some(signature), None) => Some(format!("```csharp\n{}\n```", signature)),
        (None, Some(documentation)) => Some(documentation.clone()),
        (None, None) => None,
    };
    if let Some(documentation) = documentation {
        value.insert(
            "documentation".into(),
            json!({
                "kind": "markdown",
                "value": documentation,
            }),
        );
    }

    if let Some(insert_text) = prsm_completion_snippet_for_sidecar_item(&item, &label) {
        value.insert("insertText".into(), json!(insert_text));
        value.insert("insertTextFormat".into(), json!(2));
    }

    Value::Object(value)
}

fn sidecar_completion_item_kind_number(kind: SidecarCompletionItemKind) -> u32 {
    match kind {
        SidecarCompletionItemKind::Method | SidecarCompletionItemKind::Constructor => 2,
        SidecarCompletionItemKind::Field => 5,
        SidecarCompletionItemKind::Class => 7,
        SidecarCompletionItemKind::Property => 10,
        SidecarCompletionItemKind::Enum => 13,
        SidecarCompletionItemKind::Struct => 22,
        SidecarCompletionItemKind::Event => 23,
        SidecarCompletionItemKind::Interface => 8,
    }
}

fn prsm_label_for_sidecar_item(item: &UnityCompletionItem) -> String {
    match item.kind {
        SidecarCompletionItemKind::Class
        | SidecarCompletionItemKind::Struct
        | SidecarCompletionItemKind::Interface
        | SidecarCompletionItemKind::Enum => item.label.clone(),
        _ => lower_camel_case(&item.label),
    }
}

fn prsm_completion_snippet_for_sidecar_item(item: &UnityCompletionItem, label: &str) -> Option<String> {
    if let Some(insert_text) = item.insert_text.as_ref() {
        return Some(insert_text.clone());
    }

    match item.kind {
        SidecarCompletionItemKind::Method | SidecarCompletionItemKind::Constructor => {
            let signature = item.signature.as_deref().unwrap_or_default();
            if signature.contains("()") {
                Some(format!("{}()", label))
            } else {
                Some(format!("{}($1)", label))
            }
        }
        _ => None,
    }
}

fn lower_camel_case(value: &str) -> String {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };

    format!("{}{}", first.to_ascii_lowercase(), chars.as_str())
}

fn generated_context_for_target(target: &CSharpLookupTarget, query_context: &QueryContext) -> Option<GeneratedContext> {
    Some(GeneratedContext {
        generated_owner_type: Some(target.type_name.clone()),
        generated_file: expected_generated_csharp_path(&target.file_path, query_context),
    })
}

fn should_query_sidecar_for_type(type_name: &str, project_index: &ProjectIndex) -> bool {
    let display_name = display_type_name(type_name);
    if matches!(
        display_name.as_str(),
        "Int" | "Float" | "Double" | "Bool" | "String" | "Long" | "Byte" | "Unit"
    ) {
        return false;
    }

    if lsp_support::core_type_is_unity(&display_name) {
        return true;
    }

    type_name.contains('.')
        && !project_index
            .files
            .iter()
            .any(|file| file.declaration.name.eq_ignore_ascii_case(&display_name))
}

fn format_sidecar_hover_section(title: &str, hover: &UnityHoverResult) -> String {
    let mut details = vec![format!("**Symbol:** {}", hover.display_name)];
    if let Some(namespace) = hover.namespace.as_ref() {
        details.push(format!("**Namespace:** {}", namespace));
    }
    if let Some(assembly) = hover.assembly.as_ref() {
        details.push(format!("**Assembly:** {}", assembly));
    }
    if let Some(docs_url) = hover.docs_url.as_ref() {
        details.push(format!("**Docs:** [{}]({})", hover.display_name, docs_url));
    }

    let mut sections = vec![format!("**{}**", title), details.join("  \n")];
    if let Some(signature) = hover.signature.as_ref() {
        sections.push(format!("```csharp\n{}\n```", signature));
    }
    if let Some(documentation) = hover.documentation.as_ref() {
        sections.push(documentation.clone());
    }

    sections.join("\n\n")
}

fn generated_csharp_hover_section_for_symbol(
    symbol: &IndexedSymbol,
    query_context: &QueryContext,
) -> Option<String> {
    generated_csharp_hover_section(csharp_lookup_target_from_symbol(symbol)?, query_context)
}

fn generated_csharp_hover_section_for_definition(
    definition: &HirDefinition,
    query_context: &QueryContext,
) -> Option<String> {
    generated_csharp_hover_section(csharp_lookup_target_from_definition(definition)?, query_context)
}

fn generated_csharp_hover_section(
    target: CSharpLookupTarget,
    query_context: &QueryContext,
) -> Option<String> {
    let generated_path = expected_generated_csharp_path(&target.file_path, query_context)?;
    let file_label = if generated_path.exists() {
        "**File:**"
    } else {
        "**Expected File:**"
    };

    Some(
        [
            "**Generated C#**".to_string(),
            [
                format!(
                    "**Lookup:** {}",
                    format_csharp_lookup(&target.type_name, target.member_name.as_deref())
                ),
                format!("{} {}", file_label, format_path(&generated_path)),
            ]
            .join("  \n"),
        ]
        .join("\n\n"),
    )
}

fn expected_generated_csharp_path(source_file_path: &Path, query_context: &QueryContext) -> Option<PathBuf> {
    let original_path = query_context.original_path(source_file_path);
    let extension = original_path.extension()?.to_string_lossy();
    if !extension.eq_ignore_ascii_case("prsm") {
        return None;
    }

    if let Some(output_dir) = query_context.output_dir.as_ref() {
        let stem = original_path.file_stem()?.to_string_lossy().to_string();
        return Some(output_dir.join(format!("{}.cs", stem)));
    }

    Some(original_path.with_extension("cs"))
}

fn csharp_lookup_target_from_symbol(symbol: &IndexedSymbol) -> Option<CSharpLookupTarget> {
    if symbol.kind.is_top_level() {
        return Some(CSharpLookupTarget {
            type_name: last_qualified_segment(&symbol.qualified_name),
            member_name: None,
            file_path: symbol.file_path.clone(),
        });
    }

    let type_name = symbol
        .container_name
        .as_deref()
        .map(last_qualified_segment)
        .or_else(|| container_type_name(&symbol.qualified_name))?;

    Some(CSharpLookupTarget {
        type_name,
        member_name: Some(csharp_member_name(symbol.kind.as_str(), &symbol.name)),
        file_path: symbol.file_path.clone(),
    })
}

fn csharp_lookup_target_from_definition(definition: &HirDefinition) -> Option<CSharpLookupTarget> {
    match definition.kind {
        HirDefinitionKind::Type => Some(CSharpLookupTarget {
            type_name: definition.name.clone(),
            member_name: None,
            file_path: definition.file_path.clone(),
        }),
        HirDefinitionKind::Field
        | HirDefinitionKind::Function
        | HirDefinitionKind::Coroutine
        | HirDefinitionKind::Lifecycle
        | HirDefinitionKind::EnumEntry => Some(CSharpLookupTarget {
            type_name: container_type_name(&definition.qualified_name)?,
            member_name: Some(csharp_member_name(definition.kind.as_str(), &definition.name)),
            file_path: definition.file_path.clone(),
        }),
        HirDefinitionKind::Parameter | HirDefinitionKind::Local => None,
    }
}

fn csharp_member_name(kind: &str, member_name: &str) -> String {
    if kind == "lifecycle" {
        return lifecycle_csharp_name(member_name)
            .map(str::to_string)
            .unwrap_or_else(|| capitalize_ascii(member_name));
    }

    member_name.to_string()
}

fn lifecycle_csharp_name(member_name: &str) -> Option<&'static str> {
    match member_name.to_ascii_lowercase().as_str() {
        "awake" => Some("Awake"),
        "start" => Some("Start"),
        "update" => Some("Update"),
        "fixedupdate" => Some("FixedUpdate"),
        "lateupdate" => Some("LateUpdate"),
        "onenable" => Some("OnEnable"),
        "ondisable" => Some("OnDisable"),
        "ondestroy" => Some("OnDestroy"),
        "ontriggerenter" => Some("OnTriggerEnter"),
        "ontriggerexit" => Some("OnTriggerExit"),
        "ontriggerstay" => Some("OnTriggerStay"),
        "oncollisionenter" => Some("OnCollisionEnter"),
        "oncollisionexit" => Some("OnCollisionExit"),
        "oncollisionstay" => Some("OnCollisionStay"),
        _ => None,
    }
}

fn container_type_name(qualified_name: &str) -> Option<String> {
    let segments = qualified_name
        .split('.')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    if segments.len() < 2 {
        return None;
    }

    Some(segments[segments.len() - 2].to_string())
}

fn last_qualified_segment(name: &str) -> String {
    name.split('.')
        .filter(|segment| !segment.is_empty())
        .last()
        .unwrap_or(name)
        .to_string()
}

fn capitalize_ascii(value: &str) -> String {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };

    format!("{}{}", first.to_ascii_uppercase(), chars.as_str())
}

fn format_csharp_lookup(type_name: &str, member_name: Option<&str>) -> String {
    member_name
        .map(|member_name| format!("{}.{}", type_name, member_name))
        .unwrap_or_else(|| type_name.to_string())
}

fn unity_hover_section_for_symbol(
    symbol: &IndexedSymbol,
    definition: Option<&HirDefinition>,
    project_index: &ProjectIndex,
) -> Option<String> {
    let Some((declaration, member)) = find_symbol_summary(project_index, symbol) else {
        return definition.and_then(unity_hover_section_for_definition);
    };

    if member.is_none() {
        if let Some(base_type) = declaration.base_type.as_ref() {
            if let Some(section) = format_unity_hover_section("Base Type", base_type) {
                return Some(section);
            }
        }
    }

    definition.and_then(unity_hover_section_for_definition)
}

fn unity_hover_section_for_definition(definition: &HirDefinition) -> Option<String> {
    format_unity_hover_section("Type", &definition.ty.display_name())
}

fn unity_hover_section_for_reference(reference: &IndexedReference) -> Option<String> {
    if reference.kind.as_str() != "type" {
        return None;
    }

    format_unity_hover_section("Type", &reference.name)
}

fn format_unity_hover_section(label: &str, type_name: &str) -> Option<String> {
    let docs_url = lsp_support::unity_docs_url_for_type(type_name)?;
    let display_name = display_type_name(type_name);
    let lookup_name = match lsp_support::core_type_namespace(type_name) {
        Some(namespace) if !namespace.is_empty() => format!("{}.{}", namespace, display_name),
        _ => display_name.clone(),
    };

    Some(
        [
            "**Unity API**".to_string(),
            format!("**{}:** {}", label, lookup_name),
            format!("**Docs:** [{}]({})", display_name, docs_url),
        ]
        .join("\n\n"),
    )
}

fn display_type_name(type_name: &str) -> String {
    let trimmed = type_name.trim().trim_end_matches('?');
    let without_namespace = trimmed
        .rsplit_once('.')
        .map(|(_, tail)| tail)
        .unwrap_or(trimmed);
    without_namespace
        .split('<')
        .next()
        .unwrap_or(without_namespace)
        .trim()
        .to_string()
}

fn format_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn compare_symbols(left: &IndexedSymbol, right: &IndexedSymbol) -> std::cmp::Ordering {
    left.file_path
        .cmp(&right.file_path)
        .then(left.span.start.line.cmp(&right.span.start.line))
        .then(left.span.start.col.cmp(&right.span.start.col))
        .then(left.qualified_name.cmp(&right.qualified_name))
}

fn build_document_symbols(
    project_index: &ProjectIndex,
    symbols: &[&IndexedSymbol],
    runtime_path: &Path,
    query_context: &QueryContext,
) -> Vec<Value> {
    if let Some(file_summary) = find_file_summary(project_index, runtime_path) {
        return vec![document_symbol_from_file_summary(file_summary)];
    }

    let target_path = query_context.original_path(runtime_path);
    let mut children_by_container: HashMap<String, Vec<&IndexedSymbol>> = HashMap::new();

    for symbol in symbols {
        if query_context.original_path(&symbol.file_path) != target_path {
            continue;
        }
        if let Some(container_name) = &symbol.container_name {
            children_by_container
                .entry(container_name.clone())
                .or_default()
                .push(*symbol);
        }
    }

    symbols
        .iter()
        .copied()
        .filter(|symbol| query_context.original_path(&symbol.file_path) == target_path)
        .filter(|symbol| symbol.container_name.is_none())
        .map(|symbol| {
            let children = children_by_container
                .get(&symbol.qualified_name)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|child| document_symbol_json(child, Vec::new()))
                .collect::<Vec<_>>();
            document_symbol_json(symbol, children)
        })
        .collect()
}

fn document_symbol_from_file_summary(file_summary: &FileSummary) -> Value {
    let declaration = &file_summary.declaration;
    let children = declaration
        .members
        .iter()
        .map(|member| {
            json!({
                "name": member.name,
                "detail": member.signature,
                "kind": symbol_kind_number(member_kind_name(member)),
                "range": lsp_range_json(member.span),
                "selectionRange": lsp_range_json(member.span),
                "children": [],
            })
        })
        .collect::<Vec<_>>();

    json!({
        "name": declaration.name,
        "detail": declaration.signature,
        "kind": symbol_kind_number(declaration_kind_name(declaration)),
        "range": lsp_range_json(declaration_range(declaration)),
        "selectionRange": lsp_range_json(declaration.span),
        "children": children,
    })
}

fn document_symbol_json(symbol: &IndexedSymbol, children: Vec<Value>) -> Value {
    json!({
        "name": symbol.name,
        "detail": symbol.signature,
        "kind": symbol_kind_number(symbol.kind.as_str()),
        "range": lsp_range_json(symbol.span),
        "selectionRange": lsp_range_json(symbol.span),
        "children": children,
    })
}

fn workspace_symbol_container_name(symbol: &IndexedSymbol, file_path: &Path) -> Option<String> {
    symbol.container_name.clone().or_else(|| {
        file_path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
    })
}

fn symbol_hover_metadata(symbol: &IndexedSymbol, project_index: &ProjectIndex) -> Vec<String> {
    let Some((declaration, member)) = find_symbol_summary(project_index, symbol) else {
        return Vec::new();
    };

    let mut details = Vec::new();
    if let Some(container_name) = &symbol.container_name {
        details.push(format!("**Container:** {}", container_name));
    }

    if member.is_none() {
        if let Some(base_type) = declaration.base_type.as_ref() {
            details.push(format!("**Base:** {}", base_type));
        }
        if !declaration.interfaces.is_empty() {
            details.push(format!("**Implements:** {}", declaration.interfaces.join(", ")));
        }
        if !declaration.members.is_empty() {
            details.push(format!("**Members:** {}", declaration.members.len()));
        }
    }

    details
}

fn find_symbol_summary<'a>(
    project_index: &'a ProjectIndex,
    symbol: &IndexedSymbol,
) -> Option<(&'a DeclarationSummary, Option<&'a MemberSummary>)> {
    let file_summary = project_index
        .files
        .iter()
        .find(|file_summary| file_summary.path == symbol.file_path)?;

    if symbol.container_name.is_none() {
        return Some((&file_summary.declaration, None));
    }

    let member = file_summary
        .declaration
        .members
        .iter()
        .find(|member| member.name == symbol.name);
    Some((&file_summary.declaration, member))
}

fn find_file_summary<'a>(project_index: &'a ProjectIndex, runtime_path: &Path) -> Option<&'a FileSummary> {
    project_index
        .files
        .iter()
        .find(|file_summary| file_summary.path == runtime_path)
}

fn declaration_kind_name(declaration: &DeclarationSummary) -> &'static str {
    match declaration.kind {
        crate::project_index::DeclarationKind::Component => "component",
        crate::project_index::DeclarationKind::Asset => "asset",
        crate::project_index::DeclarationKind::Class => "class",
        crate::project_index::DeclarationKind::DataClass => "data class",
        crate::project_index::DeclarationKind::Enum => "enum",
        crate::project_index::DeclarationKind::Attribute => "attribute",
    }
}

fn member_kind_name(member: &MemberSummary) -> &'static str {
    match member.kind {
        crate::project_index::MemberKind::Field => "field",
        crate::project_index::MemberKind::SerializeField => "serialize-field",
        crate::project_index::MemberKind::RequiredComponent => "required-component",
        crate::project_index::MemberKind::OptionalComponent => "optional-component",
        crate::project_index::MemberKind::ChildComponent => "child-component",
        crate::project_index::MemberKind::ParentComponent => "parent-component",
        crate::project_index::MemberKind::Function => "function",
        crate::project_index::MemberKind::Coroutine => "coroutine",
        crate::project_index::MemberKind::Lifecycle => "lifecycle",
        crate::project_index::MemberKind::EnumEntry => "enum-entry",
    }
}

fn declaration_range(declaration: &DeclarationSummary) -> crate::lexer::token::Span {
    let mut range = declaration.span;
    for member in &declaration.members {
        if compare_positions(member.span.end, range.end).is_gt() {
            range.end = member.span.end;
        }
    }
    range
}

fn workspace_symbol_score(symbol: &IndexedSymbol, query: &str) -> Option<u8> {
    if query.is_empty() {
        return Some(4);
    }

    let name = symbol.name.to_ascii_lowercase();
    let qualified_name = symbol.qualified_name.to_ascii_lowercase();
    let signature = symbol.signature.to_ascii_lowercase();

    if name == query {
        return Some(0);
    }
    if qualified_name == query {
        return Some(1);
    }
    if name.starts_with(query) {
        return Some(2);
    }
    if qualified_name.contains(query) {
        return Some(3);
    }
    if signature.contains(query) {
        return Some(4);
    }

    None
}

fn push_rename_location(
    seen: &mut HashSet<String>,
    locations: &mut Vec<RenameLocation>,
    file_path: PathBuf,
    span: crate::lexer::token::Span,
) {
    let key = location_key(&file_path, span);
    if !seen.insert(key) {
        return;
    }

    locations.push(RenameLocation { file_path, span });
}

fn validate_rename_target(new_name: &str) -> Option<String> {
    let trimmed = new_name.trim();
    if trimmed.is_empty() {
        return Some("Rename target cannot be empty.".into());
    }
    if trimmed != new_name {
        return Some("Rename target cannot start or end with whitespace.".into());
    }
    if !is_valid_identifier(trimmed) {
        return Some("Rename target must be a valid PrSM identifier.".into());
    }
    if is_reserved_keyword(trimmed) {
        return Some(format!("'{}' is a reserved PrSM keyword.", trimmed));
    }
    None
}

fn is_valid_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }

    chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn is_reserved_keyword(value: &str) -> bool {
    const RESERVED_KEYWORDS: &[&str] = &[
        "component",
        "asset",
        "class",
        "data",
        "enum",
        "attribute",
        "serialize",
        "require",
        "optional",
        "child",
        "parent",
        "val",
        "var",
        "const",
        "fixed",
        "public",
        "private",
        "protected",
        "func",
        "override",
        "return",
        "coroutine",
        "wait",
        "start",
        "stop",
        "stopall",
        "awake",
        "update",
        "fixedupdate",
        "lateupdate",
        "onenable",
        "ondisable",
        "ondestroy",
        "ontriggerenter",
        "ontriggerexit",
        "ontriggerstay",
        "oncollisionenter",
        "oncollisionexit",
        "oncollisionstay",
        "if",
        "else",
        "when",
        "for",
        "while",
        "in",
        "until",
        "downto",
        "step",
        "break",
        "continue",
        "is",
        "listen",
        "intrinsic",
        "using",
        "null",
        "this",
        "true",
        "false",
        "nextframe",
        "fixedframe",
    ];

    RESERVED_KEYWORDS.contains(&value.to_ascii_lowercase().as_str())
}

fn shows_mutability(kind: HirDefinitionKind) -> bool {
    matches!(kind, HirDefinitionKind::Field | HirDefinitionKind::Local)
}

fn overlay_path_for(overlay_root: &Path, original_path: &Path) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    original_path.hash(&mut hasher);
    let hash = hasher.finish();
    let stem = original_path
        .file_stem()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| "document".into());
    overlay_root.join(format!("{}_{}.prsm", stem, hash))
}

fn normalize_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

fn file_uri_to_path(uri: &str) -> Option<PathBuf> {
    let url = Url::parse(uri).ok()?;
    url.to_file_path().ok()
}

fn path_to_uri_string(path: &Path) -> Result<String, String> {
    Url::from_file_path(path)
        .map(|url| url.to_string())
        .map_err(|_| format!("Failed to convert path to file URI: {}", path.display()))
}

fn location_json(path: &Path, span: crate::lexer::token::Span) -> Result<Value, String> {
    Ok(json!({
        "uri": path_to_uri_string(path)?,
        "range": lsp_range_json(span),
    }))
}

fn lsp_range_json(span: crate::lexer::token::Span) -> Value {
    let start_line = span.start.line.saturating_sub(1);
    let start_col = span.start.col.saturating_sub(1);
    let mut end_line = span.end.line.saturating_sub(1);
    let mut end_col = span.end.col.saturating_sub(1);

    if end_line < start_line || (end_line == start_line && end_col <= start_col) {
        end_line = start_line;
        end_col = start_col + 1;
    }

    json!({
        "start": {
            "line": start_line,
            "character": start_col,
        },
        "end": {
            "line": end_line,
            "character": end_col,
        }
    })
}

fn diagnostic_json(diagnostic: &Diagnostic, line_lengths: &[usize]) -> Value {
    let safe_lengths = if line_lengths.is_empty() {
        vec![0usize]
    } else {
        line_lengths.to_vec()
    };
    let max_line = safe_lengths.len().saturating_sub(1);

    let start_line = clamp_index(diagnostic.span.start.line.saturating_sub(1) as usize, max_line);
    let start_col = clamp_index(diagnostic.span.start.col.saturating_sub(1) as usize, safe_lengths[start_line]);

    let mut end_line = clamp_index(diagnostic.span.end.line.saturating_sub(1) as usize, max_line);
    let mut end_col = clamp_index(diagnostic.span.end.col.saturating_sub(1) as usize, safe_lengths[end_line]);

    if end_line < start_line || (end_line == start_line && end_col <= start_col) {
        end_line = start_line;
        end_col = (start_col + 1).min(safe_lengths[start_line]);
        if end_col <= start_col && safe_lengths[start_line] == 0 {
            end_col = start_col;
        }
    }

    json!({
        "range": {
            "start": {
                "line": start_line,
                "character": start_col,
            },
            "end": {
                "line": end_line,
                "character": end_col,
            }
        },
        "severity": diagnostic_severity_number(diagnostic.severity),
        "code": diagnostic.code,
        "source": "prism",
        "message": diagnostic.message,
    })
}

fn clamp_index(value: usize, max: usize) -> usize {
    value.min(max)
}

#[cfg(test)]
mod tests {
    use super::{merge_completion_items, parse_sidecar_args, prsm_label_for_sidecar_item};
    use crate::roslyn_sidecar_protocol::{SidecarCompletionItemKind, UnityCompletionItem};
    use serde_json::json;

    #[test]
    fn lsp_parse_sidecar_args_json_array() {
        let args = parse_sidecar_args(r#"["C:/Program Files/PrSM/prism-roslyn-sidecar.dll","--stdio"]"#)
            .expect("args should parse");
        assert_eq!(args, vec!["C:/Program Files/PrSM/prism-roslyn-sidecar.dll", "--stdio"]);
    }

    #[test]
    fn lsp_merge_completion_items_prefers_sidecar_entries() {
        let merged = merge_completion_items(
            vec![json!({ "label": "setActive", "detail": "via sidecar" })],
            vec![
                json!({ "label": "setActive", "detail": "fallback" }),
                json!({ "label": "transform", "detail": "fallback" }),
            ],
        );

        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0]["detail"], json!("via sidecar"));
        assert_eq!(merged[1]["label"], json!("transform"));
    }

    #[test]
    fn lsp_sidecar_method_labels_are_lower_camel_case() {
        let label = prsm_label_for_sidecar_item(&UnityCompletionItem {
            label: "SetActive".to_string(),
            kind: SidecarCompletionItemKind::Method,
            detail: None,
            documentation: None,
            signature: None,
            insert_text: None,
            namespace: None,
            assembly: None,
            is_static: false,
        });

        assert_eq!(label, "setActive");
    }
}

fn compare_positions(
    left: crate::lexer::token::Position,
    right: crate::lexer::token::Position,
) -> std::cmp::Ordering {
    left.line.cmp(&right.line).then(left.col.cmp(&right.col))
}

fn diagnostic_severity_number(severity: Severity) -> u32 {
    match severity {
        Severity::Error => 1,
        Severity::Warning => 2,
    }
}

fn symbol_kind_number(kind: &str) -> u32 {
    match kind {
        "component" | "class" | "data class" => 5,
        "asset" => 19,
        "attribute" => 11,
        "enum" => 10,
        "enum-entry" => 22,
        "function" | "coroutine" | "lifecycle" => 6,
        "field" | "serialize-field" | "required-component" | "optional-component" | "child-component" | "parent-component" => 8,
        _ => 13,
    }
}

fn format_location(path: &Path, span: crate::lexer::token::Span) -> String {
    format!(
        "{}:{}:{}",
        path.to_string_lossy().replace('\\', "/"),
        span.start.line,
        span.start.col,
    )
}

fn location_key(path: &Path, span: crate::lexer::token::Span) -> String {
    format!(
        "{}:{}:{}:{}:{}",
        path.to_string_lossy(),
        span.start.line,
        span.start.col,
        span.end.line,
        span.end.col,
    )
}