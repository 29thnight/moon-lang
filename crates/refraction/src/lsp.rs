use crate::ast::{Annotation, Arg, Block, Decl, ElseBranch, Expr, File, FuncBody, LambdaBody, Member, Param, Stmt, StringPart, TypeRef, WaitForm, WhenBody, WhenBranch, WhenPattern};
use crate::diagnostics::{Diagnostic, Severity};
use crate::driver;
use crate::hir::{HirDefinition, HirDefinitionKind};
use crate::lexer::lexer::Lexer;
use crate::lexer::token::{Position, Span};
use crate::lsp_support;
use crate::parser::parser::Parser;
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
const TEXT_DOCUMENT_CODE_ACTION: &str = "textDocument/codeAction";
const TEXT_DOCUMENT_RENAME: &str = "textDocument/rename";
const TEXT_DOCUMENT_PREPARE_RENAME: &str = "textDocument/prepareRename";
const TEXT_DOCUMENT_DOCUMENT_SYMBOL: &str = "textDocument/documentSymbol";
const WORKSPACE_SYMBOL: &str = "workspace/symbol";

const INVALID_PARAMS: i32 = -32602;
const METHOD_NOT_FOUND: i32 = -32601;
const CODE_ACTION_KIND_REFACTOR_REWRITE: &str = "refactor.rewrite";
const CODE_ACTION_KIND_SOURCE_ORGANIZE_IMPORTS: &str = "source.organizeImports";
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
        "codeActionProvider": {
            "codeActionKinds": [
                "refactor.rewrite",
                "refactor.extract",
                "refactor.inline",
                "source.organizeImports"
            ]
        },
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
        cached_index: None,
        cached_hir: None,
        dirty_files: HashSet::new(),
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
    /// Cached project index — rebuilt incrementally when dirty files change.
    cached_index: Option<ProjectIndex>,
    /// Cached HIR project — rebuilt incrementally when dirty files change.
    cached_hir: Option<crate::hir::HirProject>,
    /// Files modified since the last index/HIR rebuild.
    dirty_files: HashSet<PathBuf>,
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
struct TextDocumentRange {
    file_path: PathBuf,
    uri: String,
    span: Span,
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExplicitTypeArgCodeAction {
    title: String,
    insert_at: Position,
    insert_text: String,
}

#[derive(Debug, Clone)]
struct LspCallableSignature {
    params: Vec<TypeRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OrganizeUsingsCodeAction {
    range: Span,
    new_text: String,
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
            TEXT_DOCUMENT_CODE_ACTION => self.handle_code_action(request),
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
        self.ensure_hir(&query_context.source_files);
        let hir_project = self.cached_hir.as_ref().unwrap();

        if let Some(definition) = hir_project.find_definition_for_position(&runtime_path, position.line, position.col) {
            return self.send_ok(
                request.id,
                location_json(
                    &query_context.original_path(&definition.file_path),
                    definition.span,
                )?,
            );
        }

        let project_index = self.cached_index.as_ref().unwrap();
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
        self.ensure_hir(&query_context.source_files);
        let hir_project = self.cached_hir.as_ref().unwrap();
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
        self.ensure_hir(&query_context.source_files);
        // Take caches out temporarily to avoid borrow conflicts with &mut self sidecar calls.
        let index = self.cached_index.take().unwrap();
        let hir = self.cached_hir.take().unwrap();
        let fallback_items = lsp_support::completion_items(
            &document_text,
            position.line,
            position.col,
            &runtime_path,
            &index,
            &hir,
        );
        // Put caches back before sidecar call.
        self.cached_index = Some(index);
        self.cached_hir = Some(hir);
        let index = self.cached_index.take().unwrap();
        let hir = self.cached_hir.take().unwrap();
        let items = self
            .sidecar_completion_items(
                &position.file_path,
                &document_text,
                position.line,
                position.col,
                &runtime_path,
                &index,
                &hir,
            )
            .map(|sidecar_items| merge_completion_items(sidecar_items, fallback_items.clone()))
            .unwrap_or(fallback_items);
        self.cached_index = Some(index);
        self.cached_hir = Some(hir);

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
        self.ensure_hir(&query_context.source_files);
        // Compute hover data in a limited scope to release borrows for sidecar call.
        let (symbol_at, reference_at, resolved_symbol, definition) = {
            let project_index = self.cached_index.as_ref().unwrap();
            let hir_project = self.cached_hir.as_ref().unwrap();
            let sym = project_index.find_symbol_at(&runtime_path, position.line, position.col).cloned();
            let refr = project_index.find_reference_at(&runtime_path, position.line, position.col).cloned();
            let resolved = refr.as_ref().and_then(|r| project_index.resolve_reference_target(r)).cloned();
            let def = hir_project
                .find_definition_for_position(&runtime_path, position.line, position.col)
                .or_else(|| resolved.as_ref().and_then(|s| hir_project.find_definition_by_qualified_name(&s.qualified_name)))
                .or_else(|| sym.as_ref().and_then(|s| hir_project.find_definition_by_qualified_name(&s.qualified_name)))
                .cloned();
            (sym, refr, resolved, def)
        };
        let index = self.cached_index.take().unwrap();
        let sidecar_section = self.sidecar_hover_section(
            &position.file_path,
            symbol_at.as_ref(),
            reference_at.as_ref(),
            resolved_symbol.as_ref(),
            definition.as_ref(),
            &index,
            &query_context,
        );
        self.cached_index = Some(index);
        let hover_markdown = build_hover_markdown(
            symbol_at.as_ref(),
            reference_at.as_ref(),
            resolved_symbol.as_ref(),
            definition.as_ref(),
            self.cached_index.as_ref().unwrap(),
            sidecar_section,
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

    fn handle_code_action(&mut self, request: Request) -> Result<(), String> {
        let target = match parse_text_document_range(&request.params) {
            Ok(target) => target,
            Err(error) => return self.send_error(request.id, INVALID_PARAMS, error),
        };
        let requested_kinds = parse_requested_code_action_kinds(&request.params);

        let Some(document_text) = self.read_document_text(&target.file_path) else {
            return self.send_ok(request.id, json!([]));
        };

        let actions = collect_code_actions_json(
            &document_text,
            &target.uri,
            target.span,
            requested_kinds.as_deref(),
        );
        self.send_ok(request.id, Value::Array(actions))
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
        self.ensure_index(&query_context.source_files);
        let project_index = self.cached_index.as_ref().unwrap();
        let mut symbols = project_index
            .query_symbols(&project_index::SymbolQuery::default())
            .into_iter()
            .filter(|symbol| query_context.original_path(&symbol.file_path) == normalize_path(&file_path))
            .collect::<Vec<_>>();
        symbols.sort_by(|left, right| compare_symbols(left, right));

        let document_symbols = build_document_symbols(project_index, &symbols, &runtime_path, &query_context);
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
        self.ensure_index(&query_context.source_files);
        let project_index = self.cached_index.as_ref().unwrap();
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
            let normalized = normalize_path(&file_path);
            self.mark_dirty(&normalized);
            self.publish_diagnostics(&normalized)?;
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
            let normalized = normalize_path(&file_path);
            self.mark_dirty(&normalized);
            self.publish_diagnostics(&normalized)?;
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
            let normalized = normalize_path(&file_path);
            self.mark_dirty(&normalized);
            self.publish_diagnostics(&normalized)?;
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

    fn build_rename_plan(&mut self, file_path: &Path, line: u32, col: u32) -> Result<RenamePlan, String> {
        let query_context = self.build_query_context(Some(file_path));
        let runtime_path = query_context.runtime_path(file_path);
        self.ensure_hir(&query_context.source_files);
        let hir_project = self.cached_hir.as_ref().unwrap();
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
        Some(format_sidecar_hover_section("Generated C#", &hover))
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
        Some(format_sidecar_hover_section("Generated C#", &hover))
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

    /// Mark a file as dirty so the next index/HIR request triggers a rebuild.
    fn mark_dirty(&mut self, file_path: &Path) {
        self.dirty_files.insert(file_path.to_path_buf());
    }

    /// Ensure cached project index is fresh. Rebuilds incrementally if dirty files exist.
    /// Call this before accessing `self.cached_index`.
    fn ensure_index(&mut self, source_files: &[PathBuf]) {
        if self.cached_index.is_none() || !self.dirty_files.is_empty() {
            // For correctness, do a full rebuild when dirty. The index is fast enough
            // for typical project sizes (<100 files). True per-file incremental can be
            // added later if profiling shows this is a bottleneck.
            self.cached_index = Some(project_index::build_project_index(source_files));
            self.cached_hir = None; // HIR must also be rebuilt when index changes.
            self.dirty_files.clear();
        }
    }

    /// Ensure cached HIR project is fresh. Requires index to be current.
    fn ensure_hir(&mut self, source_files: &[PathBuf]) {
        self.ensure_index(source_files);
        if self.cached_hir.is_none() {
            self.cached_hir = Some(driver::build_hir_project(source_files));
        }
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

fn parse_text_document_range(params: &Value) -> Result<TextDocumentRange, String> {
    let uri = params
        .get("textDocument")
        .and_then(|text_document| text_document.get("uri"))
        .and_then(Value::as_str)
        .ok_or_else(|| "Missing text document URI.".to_string())?
        .to_string();
    let file_path = file_uri_to_path(&uri).ok_or_else(|| format!("Invalid file URI: {}", uri))?;
    let start = params
        .get("range")
        .and_then(|range| range.get("start"))
        .ok_or_else(|| "Missing text document range start.".to_string())?;
    let end = params
        .get("range")
        .and_then(|range| range.get("end"))
        .ok_or_else(|| "Missing text document range end.".to_string())?;

    let span = Span {
        start: Position {
            line: start
                .get("line")
                .and_then(Value::as_u64)
                .map(|value| value as u32 + 1)
                .ok_or_else(|| "Missing text document range start line.".to_string())?,
            col: start
                .get("character")
                .and_then(Value::as_u64)
                .map(|value| value as u32 + 1)
                .ok_or_else(|| "Missing text document range start character.".to_string())?,
        },
        end: Position {
            line: end
                .get("line")
                .and_then(Value::as_u64)
                .map(|value| value as u32 + 1)
                .ok_or_else(|| "Missing text document range end line.".to_string())?,
            col: end
                .get("character")
                .and_then(Value::as_u64)
                .map(|value| value as u32 + 1)
                .ok_or_else(|| "Missing text document range end character.".to_string())?,
        },
    };

    Ok(TextDocumentRange {
        file_path: normalize_path(&file_path),
        uri,
        span,
    })
}

fn parse_requested_code_action_kinds(params: &Value) -> Option<Vec<String>> {
    let kinds = params
        .get("context")
        .and_then(|context| context.get("only"))
        .and_then(Value::as_array)
        .map(|kinds| {
            kinds
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if kinds.is_empty() {
        None
    } else {
        Some(kinds)
    }
}

fn collect_code_actions_json(
    source: &str,
    uri: &str,
    selection_span: Span,
    requested_kinds: Option<&[String]>,
) -> Vec<Value> {
    let mut actions = Vec::new();

    if requested_code_action_allows(requested_kinds, CODE_ACTION_KIND_REFACTOR_REWRITE) {
        actions.extend(collect_explicit_type_arg_actions_json(source, uri, selection_span));
    }

    if requested_code_action_allows(requested_kinds, CODE_ACTION_KIND_SOURCE_ORGANIZE_IMPORTS) {
        if let Some(action) = collect_organize_usings_action_for_source(source) {
            actions.push(organize_usings_code_action_json(uri, action));
        }
    }

    actions
}

fn requested_code_action_allows(requested_kinds: Option<&[String]>, offered_kind: &str) -> bool {
    requested_kinds.map_or(true, |requested_kinds| {
        requested_kinds
            .iter()
            .any(|requested_kind| code_action_kind_matches(requested_kind, offered_kind))
    })
}

fn code_action_kind_matches(requested_kind: &str, offered_kind: &str) -> bool {
    requested_kind == offered_kind
        || offered_kind
            .strip_prefix(requested_kind)
            .map(|suffix| suffix.starts_with('.'))
            .unwrap_or(false)
}

fn collect_explicit_type_arg_actions_json(source: &str, uri: &str, selection_span: Span) -> Vec<Value> {
    collect_explicit_type_arg_actions_for_source(source, selection_span)
        .into_iter()
        .map(|action| explicit_type_arg_code_action_json(uri, action))
        .collect()
}

fn collect_organize_usings_action_for_source(source: &str) -> Option<OrganizeUsingsCodeAction> {
    let (file, has_parse_errors) = parse_prsm_file(source);
    let first_using = file.usings.first()?;
    let using_block_range = Span {
        start: first_using.span.start,
        end: decl_start_position(&file.decl),
    };
    let current_using_text = source_text_for_span(source, using_block_range)?;
    if using_block_contains_comments(current_using_text) {
        return None;
    }

    let prune_unused = !has_parse_errors && !file_contains_intrinsic_code(&file);
    let new_text = organized_usings_text(&file, detect_line_ending(source), prune_unused);
    if current_using_text == new_text {
        return None;
    }

    Some(OrganizeUsingsCodeAction {
        range: using_block_range,
        new_text,
    })
}

fn collect_explicit_type_arg_actions_for_source(
    source: &str,
    selection_span: Span,
) -> Vec<ExplicitTypeArgCodeAction> {
    let (file, _) = parse_prsm_file(source);
    let Some(members) = decl_members(&file.decl) else {
        return Vec::new();
    };

    let callable_signatures = collect_lsp_callable_signatures(members);
    let mut actions = Vec::new();
    for member in members {
        collect_member_explicit_type_arg_actions(
            member,
            &callable_signatures,
            selection_span,
            &mut actions,
        );
    }

    let mut deduped = Vec::new();
    let mut seen = HashSet::new();
    for action in actions {
        let key = format!(
            "{}:{}:{}:{}",
            action.insert_at.line,
            action.insert_at.col,
            action.insert_text,
            action.title,
        );
        if seen.insert(key) {
            deduped.push(action);
        }
    }
    deduped
}

fn parse_prsm_file(source: &str) -> (File, bool) {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let file = parser.parse_file();
    let has_errors = !parser.errors().is_empty();
    (file, has_errors)
}

fn organized_usings_text(file: &File, line_ending: &str, prune_unused: bool) -> String {
    let used_namespaces = if prune_unused {
        used_namespaces_for_file(file)
    } else {
        HashSet::new()
    };

    let mut using_paths = file
        .usings
        .iter()
        .map(|using| using.path.trim().to_string())
        .filter(|using| !using.is_empty())
        .filter(|using| !prune_unused || should_keep_using_path(using, &used_namespaces))
        .collect::<Vec<_>>();
    using_paths.sort();
    using_paths.dedup();

    if using_paths.is_empty() {
        return String::new();
    }

    let mut text = using_paths
        .into_iter()
        .map(|using| format!("using {}", using))
        .collect::<Vec<_>>()
        .join(line_ending);
    text.push_str(line_ending);
    text.push_str(line_ending);
    text
}

fn should_keep_using_path(path: &str, used_namespaces: &HashSet<String>) -> bool {
    !supports_unused_using_cleanup(path) || used_namespaces.contains(path)
}

fn supports_unused_using_cleanup(path: &str) -> bool {
    matches!(
        path,
        "UnityEngine"
            | "UnityEngine.Events"
            | "UnityEngine.InputSystem"
            | "UnityEngine.SceneManagement"
            | "UnityEngine.UI"
    )
}

fn used_namespaces_for_file(file: &File) -> HashSet<String> {
    let mut used_namespaces = HashSet::new();
    collect_used_namespaces_for_decl(&file.decl, &mut used_namespaces);
    used_namespaces
}

fn collect_used_namespaces_for_decl(decl: &Decl, used_namespaces: &mut HashSet<String>) {
    match decl {
        Decl::Component {
            base_class,
            interfaces,
            members,
            ..
        } => {
            mark_namespace_for_known_type_name(base_class, used_namespaces);
            for interface in interfaces {
                mark_namespace_for_known_type_name(interface, used_namespaces);
            }
            for member in members {
                collect_used_namespaces_for_member(member, used_namespaces);
            }
        }
        Decl::Asset {
            base_class,
            members,
            ..
        } => {
            mark_namespace_for_known_type_name(base_class, used_namespaces);
            for member in members {
                collect_used_namespaces_for_member(member, used_namespaces);
            }
        }
        Decl::Class {
            super_class,
            interfaces,
            members,
            ..
        } => {
            if let Some(super_class) = super_class {
                mark_namespace_for_known_type_name(super_class, used_namespaces);
            }
            for interface in interfaces {
                mark_namespace_for_known_type_name(interface, used_namespaces);
            }
            for member in members {
                collect_used_namespaces_for_member(member, used_namespaces);
            }
        }
        Decl::DataClass { fields, .. } | Decl::Attribute { fields, .. } => {
            collect_used_namespaces_for_params(fields, used_namespaces);
        }
        Decl::Struct { fields, members, .. } => {
            collect_used_namespaces_for_params(fields, used_namespaces);
            for member in members {
                collect_used_namespaces_for_member(member, used_namespaces);
            }
        }
        Decl::Extension { members, .. } => {
            for member in members {
                collect_used_namespaces_for_member(member, used_namespaces);
            }
        }
        Decl::Interface { .. } => {}
        Decl::TypeAlias { target, .. } => {
            collect_used_namespaces_for_type_ref(target, used_namespaces);
        }
        Decl::Enum { params, entries, .. } => {
            for param in params {
                collect_used_namespaces_for_type_ref(&param.ty, used_namespaces);
            }
            for entry in entries {
                for arg in &entry.args {
                    collect_used_namespaces_for_expr(arg, used_namespaces);
                }
            }
        }
    }
}

fn collect_used_namespaces_for_member(member: &Member, used_namespaces: &mut HashSet<String>) {
    match member {
        Member::SerializeField {
            annotations,
            ty,
            init,
            ..
        } => {
            collect_used_namespaces_for_annotations(annotations, used_namespaces);
            collect_used_namespaces_for_type_ref(ty, used_namespaces);
            if let Some(init) = init {
                collect_used_namespaces_for_expr(init, used_namespaces);
            }
        }
        Member::Field { ty, init, .. } => {
            if let Some(ty) = ty {
                collect_used_namespaces_for_type_ref(ty, used_namespaces);
            }
            if let Some(init) = init {
                collect_used_namespaces_for_expr(init, used_namespaces);
            }
        }
        Member::Require { ty, .. }
        | Member::Optional { ty, .. }
        | Member::Child { ty, .. }
        | Member::Parent { ty, .. } => collect_used_namespaces_for_type_ref(ty, used_namespaces),
        Member::Func {
            params,
            return_ty,
            body,
            ..
        } => {
            collect_used_namespaces_for_params(params, used_namespaces);
            if let Some(return_ty) = return_ty {
                collect_used_namespaces_for_type_ref(return_ty, used_namespaces);
            }
            match body {
                FuncBody::Block(block) => collect_used_namespaces_for_block(block, used_namespaces),
                FuncBody::ExprBody(expr) => collect_used_namespaces_for_expr(expr, used_namespaces),
            }
        }
        Member::Coroutine { params, body, .. } | Member::Lifecycle { params, body, .. } => {
            collect_used_namespaces_for_params(params, used_namespaces);
            collect_used_namespaces_for_block(body, used_namespaces);
        }
        Member::IntrinsicFunc {
            params,
            return_ty,
            ..
        } => {
            collect_used_namespaces_for_params(params, used_namespaces);
            if let Some(return_ty) = return_ty {
                collect_used_namespaces_for_type_ref(return_ty, used_namespaces);
            }
        }
        Member::IntrinsicCoroutine { params, .. } => {
            collect_used_namespaces_for_params(params, used_namespaces);
        }
        Member::Pool { item_type, .. } => {
            collect_used_namespaces_for_type_ref(item_type, used_namespaces);
        }
        Member::Property { ty, getter, setter, .. } => {
            collect_used_namespaces_for_type_ref(ty, used_namespaces);
            if let Some(body) = getter {
                match body {
                    FuncBody::Block(block) => collect_used_namespaces_for_block(block, used_namespaces),
                    FuncBody::ExprBody(expr) => collect_used_namespaces_for_expr(expr, used_namespaces),
                }
            }
            if let Some(setter) = setter {
                collect_used_namespaces_for_block(&setter.body, used_namespaces);
            }
        }
        Member::Event { ty, .. } => {
            collect_used_namespaces_for_type_ref(ty, used_namespaces);
        }
        Member::StateMachine { states, .. } => {
            for s in states {
                if let Some(b) = &s.enter {
                    collect_used_namespaces_for_block(b, used_namespaces);
                }
                if let Some(b) = &s.exit {
                    collect_used_namespaces_for_block(b, used_namespaces);
                }
            }
        }
        Member::Command { params, execute, undo, can_execute, .. } => {
            collect_used_namespaces_for_params(params, used_namespaces);
            collect_used_namespaces_for_block(execute, used_namespaces);
            if let Some(u) = undo {
                collect_used_namespaces_for_block(u, used_namespaces);
            }
            if let Some(ce) = can_execute {
                collect_used_namespaces_for_expr(ce, used_namespaces);
            }
        }
        Member::BindProperty { ty, init, .. } => {
            collect_used_namespaces_for_type_ref(ty, used_namespaces);
            if let Some(e) = init {
                collect_used_namespaces_for_expr(e, used_namespaces);
            }
        }
    }
}

fn collect_used_namespaces_for_annotations(
    annotations: &[Annotation],
    used_namespaces: &mut HashSet<String>,
) {
    for annotation in annotations {
        mark_namespace_for_known_type_name(&annotation.name, used_namespaces);
        for arg in &annotation.args {
            collect_used_namespaces_for_expr(arg, used_namespaces);
        }
    }
}

fn collect_used_namespaces_for_params(params: &[Param], used_namespaces: &mut HashSet<String>) {
    for param in params {
        collect_used_namespaces_for_type_ref(&param.ty, used_namespaces);
        if let Some(default) = &param.default {
            collect_used_namespaces_for_expr(default, used_namespaces);
        }
    }
}

fn collect_used_namespaces_for_block(block: &Block, used_namespaces: &mut HashSet<String>) {
    for stmt in &block.stmts {
        collect_used_namespaces_for_stmt(stmt, used_namespaces);
    }
}

fn collect_used_namespaces_for_stmt(stmt: &Stmt, used_namespaces: &mut HashSet<String>) {
    match stmt {
        Stmt::ValDecl { ty, init, .. } => {
            if let Some(ty) = ty {
                collect_used_namespaces_for_type_ref(ty, used_namespaces);
            }
            collect_used_namespaces_for_expr(init, used_namespaces);
        }
        Stmt::VarDecl { ty, init, .. } => {
            if let Some(ty) = ty {
                collect_used_namespaces_for_type_ref(ty, used_namespaces);
            }
            if let Some(init) = init {
                collect_used_namespaces_for_expr(init, used_namespaces);
            }
        }
        Stmt::Assignment { target, value, .. } => {
            collect_used_namespaces_for_expr(target, used_namespaces);
            collect_used_namespaces_for_expr(value, used_namespaces);
        }
        Stmt::Expr { expr, .. } => collect_used_namespaces_for_expr(expr, used_namespaces),
        Stmt::If {
            cond,
            then_block,
            else_branch,
            ..
        } => {
            collect_used_namespaces_for_expr(cond, used_namespaces);
            collect_used_namespaces_for_block(then_block, used_namespaces);
            if let Some(else_branch) = else_branch {
                match else_branch {
                    ElseBranch::ElseBlock(block) => {
                        collect_used_namespaces_for_block(block, used_namespaces)
                    }
                    ElseBranch::ElseIf(stmt) => collect_used_namespaces_for_stmt(stmt, used_namespaces),
                }
            }
        }
        Stmt::When { subject, branches, .. } => {
            if let Some(subject) = subject {
                collect_used_namespaces_for_expr(subject, used_namespaces);
            }
            for branch in branches {
                collect_used_namespaces_for_when_branch(branch, used_namespaces);
            }
        }
        Stmt::For {
            for_pattern,
            iterable,
            body,
            ..
        } => {
            if let Some(for_pattern) = for_pattern {
                mark_namespace_for_known_type_name(&for_pattern.type_name, used_namespaces);
            }
            collect_used_namespaces_for_expr(iterable, used_namespaces);
            collect_used_namespaces_for_block(body, used_namespaces);
        }
        Stmt::DestructureVal { pattern, init, .. } => {
            mark_namespace_for_known_type_name(&pattern.type_name, used_namespaces);
            collect_used_namespaces_for_expr(init, used_namespaces);
        }
        Stmt::While { cond, body, .. } => {
            collect_used_namespaces_for_expr(cond, used_namespaces);
            collect_used_namespaces_for_block(body, used_namespaces);
        }
        Stmt::Return { value, .. } => {
            if let Some(value) = value {
                collect_used_namespaces_for_expr(value, used_namespaces);
            }
        }
        Stmt::Wait { form, .. } => match form {
            WaitForm::Duration(expr) | WaitForm::Until(expr) | WaitForm::While(expr) => {
                collect_used_namespaces_for_expr(expr, used_namespaces)
            }
            WaitForm::NextFrame | WaitForm::FixedFrame => {}
        },
        Stmt::Start { call, .. } => collect_used_namespaces_for_expr(call, used_namespaces),
        Stmt::Stop { target, .. } => collect_used_namespaces_for_expr(target, used_namespaces),
        Stmt::Listen {
            event,
            body,
            ..
        } => {
            collect_used_namespaces_for_expr(event, used_namespaces);
            collect_used_namespaces_for_block(body, used_namespaces);
        }
        Stmt::StopAll { .. }
        | Stmt::Unlisten { .. }
        | Stmt::IntrinsicBlock { .. }
        | Stmt::Break { .. }
        | Stmt::Continue { .. } => {}
        Stmt::Try { try_block, catches, finally_block, .. } => {
            collect_used_namespaces_for_block(try_block, used_namespaces);
            for c in catches {
                collect_used_namespaces_for_type_ref(&c.ty, used_namespaces);
                collect_used_namespaces_for_block(&c.body, used_namespaces);
            }
            if let Some(fb) = finally_block {
                collect_used_namespaces_for_block(fb, used_namespaces);
            }
        }
        Stmt::Throw { expr, .. } => {
            collect_used_namespaces_for_expr(expr, used_namespaces);
        }
        Stmt::Use { ty, init, body, .. } => {
            if let Some(ty) = ty {
                collect_used_namespaces_for_type_ref(ty, used_namespaces);
            }
            collect_used_namespaces_for_expr(init, used_namespaces);
            if let Some(body) = body {
                collect_used_namespaces_for_block(body, used_namespaces);
            }
        }
        Stmt::BindTo { target, .. } => {
            collect_used_namespaces_for_expr(target, used_namespaces);
        }
        // Language 5, Sprint 1: yield / yield break / preprocessor block.
        Stmt::Yield { value, .. } => {
            collect_used_namespaces_for_expr(value, used_namespaces);
        }
        Stmt::YieldBreak { .. } => {}
        Stmt::Preprocessor { arms, else_arm, .. } => {
            for arm in arms {
                for s in &arm.body {
                    collect_used_namespaces_for_stmt(s, used_namespaces);
                }
            }
            if let Some(else_stmts) = else_arm {
                for s in else_stmts {
                    collect_used_namespaces_for_stmt(s, used_namespaces);
                }
            }
        }
    }
}

fn collect_used_namespaces_for_when_branch(
    branch: &WhenBranch,
    used_namespaces: &mut HashSet<String>,
) {
    match &branch.pattern {
        WhenPattern::Expression(expr) => collect_used_namespaces_for_expr(expr, used_namespaces),
        WhenPattern::Is(ty) => collect_used_namespaces_for_type_ref(ty, used_namespaces),
        WhenPattern::Else => {}
        WhenPattern::Binding { path, .. } => {
            if let Some(type_name) = path.first() {
                mark_namespace_for_known_type_name(type_name, used_namespaces);
            }
        }
        WhenPattern::Or { patterns, .. } => {
            for p in patterns {
                match p {
                    WhenPattern::Expression(e) => collect_used_namespaces_for_expr(e, used_namespaces),
                    WhenPattern::Is(ty) => collect_used_namespaces_for_type_ref(ty, used_namespaces),
                    _ => {}
                }
            }
        }
        WhenPattern::Range { start, end, .. } => {
            collect_used_namespaces_for_expr(start, used_namespaces);
            collect_used_namespaces_for_expr(end, used_namespaces);
        }
    }

    if let Some(guard) = &branch.guard {
        collect_used_namespaces_for_expr(guard, used_namespaces);
    }

    match &branch.body {
        WhenBody::Block(block) => collect_used_namespaces_for_block(block, used_namespaces),
        WhenBody::Expr(expr) => collect_used_namespaces_for_expr(expr, used_namespaces),
    }
}

fn collect_used_namespaces_for_expr(expr: &Expr, used_namespaces: &mut HashSet<String>) {
    match expr {
        Expr::IntLit(_, _)
        | Expr::FloatLit(_, _)
        | Expr::DurationLit(_, _)
        | Expr::StringLit(_, _)
        | Expr::BoolLit(_, _)
        | Expr::Null(_)
        | Expr::Ident(_, _)
        | Expr::This(_)
        // Language 5, Sprint 2: `nameof(x)` references no namespace beyond
        // the surrounding type — it's emitted verbatim.
        | Expr::NameOf { .. } => {}
        Expr::StringInterp { parts, .. } => {
            for part in parts {
                if let StringPart::Expr(expr) = part {
                    collect_used_namespaces_for_expr(expr, used_namespaces);
                }
            }
        }
        Expr::Binary { left, right, .. } | Expr::Elvis { left, right, .. } => {
            collect_used_namespaces_for_expr(left, used_namespaces);
            collect_used_namespaces_for_expr(right, used_namespaces);
        }
        Expr::Unary { operand, .. } | Expr::NonNullAssert { expr: operand, .. } => {
            collect_used_namespaces_for_expr(operand, used_namespaces);
        }
        Expr::MemberAccess { receiver, name, .. } | Expr::SafeCall { receiver, name, .. } => {
            collect_used_namespaces_for_expr(receiver, used_namespaces);
            collect_used_namespaces_for_receiver(receiver, Some(name), used_namespaces);
        }
        Expr::SafeMethodCall {
            receiver,
            name,
            type_args,
            args,
            ..
        } => {
            collect_used_namespaces_for_expr(receiver, used_namespaces);
            collect_used_namespaces_for_receiver(receiver, Some(name), used_namespaces);
            for type_arg in type_args {
                collect_used_namespaces_for_type_ref(type_arg, used_namespaces);
            }
            collect_used_namespaces_for_args(args, used_namespaces);
        }
        Expr::Call {
            receiver,
            name,
            type_args,
            args,
            ..
        } => {
            if let Some(receiver) = receiver {
                collect_used_namespaces_for_expr(receiver, used_namespaces);
                collect_used_namespaces_for_receiver(receiver, Some(name), used_namespaces);
            } else {
                collect_used_namespaces_for_free_call(name, used_namespaces);
            }
            for type_arg in type_args {
                collect_used_namespaces_for_type_ref(type_arg, used_namespaces);
            }
            collect_used_namespaces_for_args(args, used_namespaces);
        }
        Expr::IndexAccess {
            receiver,
            index,
            ..
        } => {
            collect_used_namespaces_for_expr(receiver, used_namespaces);
            collect_used_namespaces_for_expr(index, used_namespaces);
        }
        Expr::IfExpr {
            cond,
            then_block,
            else_block,
            ..
        } => {
            collect_used_namespaces_for_expr(cond, used_namespaces);
            collect_used_namespaces_for_block(then_block, used_namespaces);
            collect_used_namespaces_for_block(else_block, used_namespaces);
        }
        Expr::WhenExpr {
            subject,
            branches,
            ..
        } => {
            if let Some(subject) = subject {
                collect_used_namespaces_for_expr(subject, used_namespaces);
            }
            for branch in branches {
                collect_used_namespaces_for_when_branch(branch, used_namespaces);
            }
        }
        Expr::Range {
            start,
            end,
            step,
            ..
        } => {
            collect_used_namespaces_for_expr(start, used_namespaces);
            collect_used_namespaces_for_expr(end, used_namespaces);
            if let Some(step) = step {
                collect_used_namespaces_for_expr(step, used_namespaces);
            }
        }
        Expr::Is { expr, ty, .. } => {
            collect_used_namespaces_for_expr(expr, used_namespaces);
            collect_used_namespaces_for_type_ref(ty, used_namespaces);
        }
        Expr::Lambda { body, .. } => match body {
            LambdaBody::Block(block) => collect_used_namespaces_for_block(block, used_namespaces),
            LambdaBody::Expr(e) => collect_used_namespaces_for_expr(e, used_namespaces),
        },
        Expr::IntrinsicExpr { ty, .. } => collect_used_namespaces_for_type_ref(ty, used_namespaces),
        Expr::SafeCastExpr { expr, target_type, .. } => {
            collect_used_namespaces_for_expr(expr, used_namespaces);
            collect_used_namespaces_for_type_ref(target_type, used_namespaces);
        }
        Expr::ForceCastExpr { expr, target_type, .. } => {
            collect_used_namespaces_for_expr(expr, used_namespaces);
            collect_used_namespaces_for_type_ref(target_type, used_namespaces);
        }
        Expr::Tuple { elements, .. } => {
            for e in elements {
                collect_used_namespaces_for_expr(e, used_namespaces);
            }
        }
        Expr::ListLit { elements, .. } => {
            // The implicit `List<T>` lives in System.Collections.Generic.
            used_namespaces.insert("System.Collections.Generic".to_string());
            for e in elements {
                collect_used_namespaces_for_expr(e, used_namespaces);
            }
        }
        Expr::MapLit { entries, .. } => {
            // The implicit `Dictionary<K, V>` lives in System.Collections.Generic.
            used_namespaces.insert("System.Collections.Generic".to_string());
            for (k, v) in entries {
                collect_used_namespaces_for_expr(k, used_namespaces);
                collect_used_namespaces_for_expr(v, used_namespaces);
            }
        }
        Expr::Await { expr: inner, .. } => {
            // Phase 5: await is just a prefix on the inner expression.
            collect_used_namespaces_for_expr(inner, used_namespaces);
        }
    }
}

fn collect_used_namespaces_for_args(args: &[Arg], used_namespaces: &mut HashSet<String>) {
    for arg in args {
        collect_used_namespaces_for_expr(&arg.value, used_namespaces);
    }
}

fn collect_used_namespaces_for_receiver(
    receiver: &Expr,
    member_name: Option<&str>,
    used_namespaces: &mut HashSet<String>,
) {
    let Expr::Ident(name, _) = receiver else {
        return;
    };

    if name.eq_ignore_ascii_case("input") {
        if member_name.is_some_and(|member_name| member_name.eq_ignore_ascii_case("action")) {
            used_namespaces.insert("UnityEngine.InputSystem".to_string());
        } else {
            mark_namespace_for_known_type_name("Input", used_namespaces);
        }
        return;
    }

    if name.eq_ignore_ascii_case("quat") {
        mark_namespace_for_known_type_name("Quaternion", used_namespaces);
        return;
    }

    mark_namespace_for_known_type_name(name, used_namespaces);
}

fn collect_used_namespaces_for_free_call(name: &str, used_namespaces: &mut HashSet<String>) {
    match name {
        "vec2" => mark_namespace_for_known_type_name("Vector2", used_namespaces),
        "vec3" => mark_namespace_for_known_type_name("Vector3", used_namespaces),
        "color" => mark_namespace_for_known_type_name("Color", used_namespaces),
        "Destroy" => mark_namespace_for_known_type_name("Object", used_namespaces),
        "log" | "warn" | "error" | "print" => {
            mark_namespace_for_known_type_name("Debug", used_namespaces)
        }
        _ => {}
    }
}

fn collect_used_namespaces_for_type_ref(ty: &TypeRef, used_namespaces: &mut HashSet<String>) {
    match ty {
        TypeRef::Simple { name, .. } => mark_namespace_for_known_type_name(name, used_namespaces),
        TypeRef::Generic {
            name,
            type_args,
            ..
        } => {
            mark_namespace_for_known_type_name(name, used_namespaces);
            for type_arg in type_args {
                collect_used_namespaces_for_type_ref(type_arg, used_namespaces);
            }
        }
        TypeRef::Qualified { .. } => {}
        TypeRef::Tuple { types, .. } => {
            for t in types {
                collect_used_namespaces_for_type_ref(t, used_namespaces);
            }
        }
        TypeRef::Function { param_types, return_type, .. } => {
            for t in param_types {
                collect_used_namespaces_for_type_ref(t, used_namespaces);
            }
            collect_used_namespaces_for_type_ref(return_type, used_namespaces);
        }
    }
}

fn mark_namespace_for_known_type_name(type_name: &str, used_namespaces: &mut HashSet<String>) {
    let trimmed = type_name.trim().trim_end_matches('?');
    if trimmed.is_empty() || trimmed.contains('.') || trimmed.contains('<') {
        return;
    }

    let display_name = display_type_name(trimmed);
    if let Some(namespace) = lsp_support::core_type_namespace(&display_name) {
        used_namespaces.insert(namespace.to_string());
    }
}

fn file_contains_intrinsic_code(file: &File) -> bool {
    decl_contains_intrinsic_code(&file.decl)
}

fn decl_contains_intrinsic_code(decl: &Decl) -> bool {
    match decl {
        Decl::Component { members, .. }
        | Decl::Asset { members, .. }
        | Decl::Class { members, .. } => members.iter().any(member_contains_intrinsic_code),
        Decl::Struct { members, .. }
        | Decl::Extension { members, .. } => members.iter().any(member_contains_intrinsic_code),
        Decl::DataClass { .. } | Decl::Enum { .. } | Decl::Attribute { .. } | Decl::Interface { .. } | Decl::TypeAlias { .. } => false,
    }
}

fn member_contains_intrinsic_code(member: &Member) -> bool {
    match member {
        Member::IntrinsicFunc { .. } | Member::IntrinsicCoroutine { .. } => true,
        Member::Func { body, .. } => match body {
            FuncBody::Block(block) => block_contains_intrinsic_code(block),
            FuncBody::ExprBody(expr) => expr_contains_intrinsic_code(expr),
        },
        Member::Coroutine { body, .. } | Member::Lifecycle { body, .. } => {
            block_contains_intrinsic_code(body)
        }
        Member::SerializeField { init, .. } | Member::Field { init, .. } => {
            init.as_ref().is_some_and(expr_contains_intrinsic_code)
        }
        Member::Require { .. }
        | Member::Optional { .. }
        | Member::Child { .. }
        | Member::Parent { .. }
        | Member::Pool { .. }
        | Member::Property { .. }
        | Member::Event { .. }
        | Member::BindProperty { .. }
        | Member::StateMachine { .. } => false,
        Member::Command { execute, undo, can_execute, .. } => {
            block_contains_intrinsic_code(execute)
                || undo.as_ref().is_some_and(block_contains_intrinsic_code)
                || can_execute.as_ref().is_some_and(expr_contains_intrinsic_code)
        }
    }
}

fn block_contains_intrinsic_code(block: &Block) -> bool {
    block.stmts.iter().any(stmt_contains_intrinsic_code)
}

fn stmt_contains_intrinsic_code(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::IntrinsicBlock { .. } => true,
        Stmt::ValDecl { init, .. } => expr_contains_intrinsic_code(init),
        Stmt::VarDecl { init, .. } => init.as_ref().is_some_and(expr_contains_intrinsic_code),
        Stmt::Assignment { target, value, .. } => {
            expr_contains_intrinsic_code(target) || expr_contains_intrinsic_code(value)
        }
        Stmt::Expr { expr, .. }
        | Stmt::Start { call: expr, .. }
        | Stmt::Stop { target: expr, .. } => expr_contains_intrinsic_code(expr),
        Stmt::If {
            cond,
            then_block,
            else_branch,
            ..
        } => {
            expr_contains_intrinsic_code(cond)
                || block_contains_intrinsic_code(then_block)
                || else_branch.as_ref().is_some_and(else_branch_contains_intrinsic_code)
        }
        Stmt::When { subject, branches, .. } => {
            subject.as_ref().is_some_and(expr_contains_intrinsic_code)
                || branches.iter().any(when_branch_contains_intrinsic_code)
        }
        Stmt::For { iterable, body, .. } => {
            expr_contains_intrinsic_code(iterable) || block_contains_intrinsic_code(body)
        }
        Stmt::DestructureVal { init, .. } => expr_contains_intrinsic_code(init),
        Stmt::While { cond, body, .. } => {
            expr_contains_intrinsic_code(cond) || block_contains_intrinsic_code(body)
        }
        Stmt::Return { value, .. } => value.as_ref().is_some_and(expr_contains_intrinsic_code),
        Stmt::Wait { form, .. } => match form {
            WaitForm::Duration(expr) | WaitForm::Until(expr) | WaitForm::While(expr) => {
                expr_contains_intrinsic_code(expr)
            }
            WaitForm::NextFrame | WaitForm::FixedFrame => false,
        },
        Stmt::Listen {
            event,
            body,
            ..
        } => expr_contains_intrinsic_code(event) || block_contains_intrinsic_code(body),
        Stmt::StopAll { .. }
        | Stmt::Unlisten { .. }
        | Stmt::Break { .. }
        | Stmt::Continue { .. } => false,
        Stmt::Try { try_block, catches, finally_block, .. } => {
            block_contains_intrinsic_code(try_block)
                || catches.iter().any(|c| block_contains_intrinsic_code(&c.body))
                || finally_block.as_ref().is_some_and(block_contains_intrinsic_code)
        }
        Stmt::Throw { expr, .. } => expr_contains_intrinsic_code(expr),
        Stmt::Use { init, body, .. } => {
            expr_contains_intrinsic_code(init)
                || body.as_ref().is_some_and(block_contains_intrinsic_code)
        }
        Stmt::BindTo { target, .. } => expr_contains_intrinsic_code(target),
        // Language 5, Sprint 1: yield bodies and preprocessor blocks may
        // themselves contain intrinsic code; recurse into both.
        Stmt::Yield { value, .. } => expr_contains_intrinsic_code(value),
        Stmt::YieldBreak { .. } => false,
        Stmt::Preprocessor { arms, else_arm, .. } => {
            arms.iter().any(|arm| arm.body.iter().any(stmt_contains_intrinsic_code))
                || else_arm
                    .as_ref()
                    .is_some_and(|stmts| stmts.iter().any(stmt_contains_intrinsic_code))
        }
    }
}

fn else_branch_contains_intrinsic_code(else_branch: &ElseBranch) -> bool {
    match else_branch {
        ElseBranch::ElseBlock(block) => block_contains_intrinsic_code(block),
        ElseBranch::ElseIf(stmt) => stmt_contains_intrinsic_code(stmt),
    }
}

fn when_branch_contains_intrinsic_code(branch: &WhenBranch) -> bool {
    branch.guard.as_ref().is_some_and(expr_contains_intrinsic_code)
        || matches!(&branch.pattern, WhenPattern::Expression(expr) if expr_contains_intrinsic_code(expr))
        || match &branch.body {
            WhenBody::Block(block) => block_contains_intrinsic_code(block),
            WhenBody::Expr(expr) => expr_contains_intrinsic_code(expr),
        }
}

fn expr_contains_intrinsic_code(expr: &Expr) -> bool {
    match expr {
        Expr::IntrinsicExpr { .. } => true,
        Expr::StringInterp { parts, .. } => parts.iter().any(|part| match part {
            StringPart::Literal(_) => false,
            StringPart::Expr(expr) => expr_contains_intrinsic_code(expr),
        }),
        Expr::Binary { left, right, .. } | Expr::Elvis { left, right, .. } => {
            expr_contains_intrinsic_code(left) || expr_contains_intrinsic_code(right)
        }
        Expr::Unary { operand, .. } | Expr::NonNullAssert { expr: operand, .. } => {
            expr_contains_intrinsic_code(operand)
        }
        Expr::MemberAccess { receiver, .. } | Expr::SafeCall { receiver, .. } => {
            expr_contains_intrinsic_code(receiver)
        }
        Expr::SafeMethodCall {
            receiver,
            args,
            ..
        }
        | Expr::Call {
            receiver: Some(receiver),
            args,
            ..
        } => {
            expr_contains_intrinsic_code(receiver)
                || args.iter().any(|arg| expr_contains_intrinsic_code(&arg.value))
        }
        Expr::Call {
            receiver: None,
            args,
            ..
        } => args.iter().any(|arg| expr_contains_intrinsic_code(&arg.value)),
        Expr::IndexAccess {
            receiver,
            index,
            ..
        } => expr_contains_intrinsic_code(receiver) || expr_contains_intrinsic_code(index),
        Expr::IfExpr {
            cond,
            then_block,
            else_block,
            ..
        } => {
            expr_contains_intrinsic_code(cond)
                || block_contains_intrinsic_code(then_block)
                || block_contains_intrinsic_code(else_block)
        }
        Expr::WhenExpr {
            subject,
            branches,
            ..
        } => {
            subject.as_ref().is_some_and(|subject| expr_contains_intrinsic_code(subject))
                || branches.iter().any(when_branch_contains_intrinsic_code)
        }
        Expr::Range {
            start,
            end,
            step,
            ..
        } => {
            expr_contains_intrinsic_code(start)
                || expr_contains_intrinsic_code(end)
                || step.as_ref().is_some_and(|step| expr_contains_intrinsic_code(step))
        }
        Expr::Is { expr, .. } => expr_contains_intrinsic_code(expr),
        Expr::Lambda { body, .. } => match body {
            LambdaBody::Block(block) => block_contains_intrinsic_code(block),
            LambdaBody::Expr(_) => false,
        },
        Expr::SafeCastExpr { expr, .. } | Expr::ForceCastExpr { expr, .. } => {
            expr_contains_intrinsic_code(expr)
        }
        Expr::Tuple { elements, .. } => elements.iter().any(expr_contains_intrinsic_code),
        Expr::ListLit { elements, .. } => elements.iter().any(expr_contains_intrinsic_code),
        Expr::MapLit { entries, .. } => entries.iter().any(|(k, v)| {
            expr_contains_intrinsic_code(k) || expr_contains_intrinsic_code(v)
        }),
        Expr::Await { expr: inner, .. } => expr_contains_intrinsic_code(inner),
        Expr::IntLit(_, _)
        | Expr::FloatLit(_, _)
        | Expr::DurationLit(_, _)
        | Expr::StringLit(_, _)
        | Expr::BoolLit(_, _)
        | Expr::Null(_)
        | Expr::Ident(_, _)
        | Expr::This(_)
        // Language 5, Sprint 2: nameof can never embed an intrinsic block.
        | Expr::NameOf { .. } => false,
    }
}

fn detect_line_ending(source: &str) -> &'static str {
    if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

fn using_block_contains_comments(text: &str) -> bool {
    text.contains("//") || text.contains("/*")
}

fn source_text_for_span<'a>(source: &'a str, span: Span) -> Option<&'a str> {
    let start = offset_for_position(source, span.start)?;
    let end = offset_for_position(source, span.end)?;
    source.get(start..end)
}

fn offset_for_position(source: &str, position: Position) -> Option<usize> {
    if position.line == 1 && position.col == 1 {
        return Some(0);
    }

    let mut current_line = 1u32;
    let mut current_col = 1u32;
    for (index, ch) in source.char_indices() {
        if current_line == position.line && current_col == position.col {
            return Some(index);
        }

        if ch == '\n' {
            current_line += 1;
            current_col = 1;
        } else {
            current_col += 1;
        }
    }

    if current_line == position.line && current_col == position.col {
        Some(source.len())
    } else {
        None
    }
}

fn decl_members(decl: &Decl) -> Option<&[Member]> {
    match decl {
        Decl::Component { members, .. }
        | Decl::Asset { members, .. }
        | Decl::Class { members, .. } => Some(members.as_slice()),
        Decl::Struct { members, .. } => Some(members.as_slice()),
        Decl::Extension { members, .. } => Some(members.as_slice()),
        Decl::DataClass { .. } | Decl::Enum { .. } | Decl::Attribute { .. } | Decl::Interface { .. } | Decl::TypeAlias { .. } => None,
    }
}

fn decl_start_position(decl: &Decl) -> Position {
    match decl {
        Decl::Component { span, .. }
        | Decl::Asset { span, .. }
        | Decl::Class { span, .. }
        | Decl::DataClass { span, .. }
        | Decl::Enum { span, .. }
        | Decl::Attribute { span, .. }
        | Decl::Interface { span, .. }
        | Decl::TypeAlias { span, .. }
        | Decl::Struct { span, .. }
        | Decl::Extension { span, .. } => span.start,
    }
}

fn collect_lsp_callable_signatures(members: &[Member]) -> HashMap<String, LspCallableSignature> {
    let mut signatures = HashMap::new();

    for member in members {
        match member {
            Member::Func { name, params, .. }
            | Member::Coroutine { name, params, .. }
            | Member::IntrinsicFunc { name, params, .. }
            | Member::IntrinsicCoroutine { name, params, .. } => {
                signatures.insert(
                    name.clone(),
                    LspCallableSignature {
                        params: params.iter().map(|param| param.ty.clone()).collect(),
                    },
                );
            }
            _ => {}
        }
    }

    signatures
}

fn collect_member_explicit_type_arg_actions(
    member: &Member,
    callable_signatures: &HashMap<String, LspCallableSignature>,
    selection_span: Span,
    actions: &mut Vec<ExplicitTypeArgCodeAction>,
) {
    match member {
        Member::Func { return_ty, body, .. } => match body {
            FuncBody::Block(block) => collect_block_explicit_type_arg_actions(
                block,
                callable_signatures,
                return_ty.as_ref(),
                selection_span,
                actions,
            ),
            FuncBody::ExprBody(expr) => collect_expr_explicit_type_arg_actions(
                expr,
                return_ty.as_ref(),
                callable_signatures,
                selection_span,
                actions,
            ),
        },
        Member::Coroutine { body, .. } | Member::Lifecycle { body, .. } => {
            collect_block_explicit_type_arg_actions(body, callable_signatures, None, selection_span, actions)
        }
        _ => {}
    }
}

fn collect_block_explicit_type_arg_actions(
    block: &Block,
    callable_signatures: &HashMap<String, LspCallableSignature>,
    expected_return_ty: Option<&TypeRef>,
    selection_span: Span,
    actions: &mut Vec<ExplicitTypeArgCodeAction>,
) {
    for stmt in &block.stmts {
        collect_stmt_explicit_type_arg_actions(
            stmt,
            callable_signatures,
            expected_return_ty,
            selection_span,
            actions,
        );
    }
}

fn collect_value_block_explicit_type_arg_actions(
    block: &Block,
    callable_signatures: &HashMap<String, LspCallableSignature>,
    expected_type: Option<&TypeRef>,
    selection_span: Span,
    actions: &mut Vec<ExplicitTypeArgCodeAction>,
) {
    if block.stmts.is_empty() {
        return;
    }

    let last_index = block.stmts.len() - 1;
    for stmt in &block.stmts[..last_index] {
        collect_stmt_explicit_type_arg_actions(
            stmt,
            callable_signatures,
            expected_type,
            selection_span,
            actions,
        );
    }

    match &block.stmts[last_index] {
        Stmt::Expr { expr, .. } => collect_expr_explicit_type_arg_actions(
            expr,
            expected_type,
            callable_signatures,
            selection_span,
            actions,
        ),
        Stmt::Return { value: Some(expr), .. } => collect_expr_explicit_type_arg_actions(
            expr,
            expected_type,
            callable_signatures,
            selection_span,
            actions,
        ),
        other => collect_stmt_explicit_type_arg_actions(
            other,
            callable_signatures,
            expected_type,
            selection_span,
            actions,
        ),
    }
}

fn collect_stmt_explicit_type_arg_actions(
    stmt: &Stmt,
    callable_signatures: &HashMap<String, LspCallableSignature>,
    expected_return_ty: Option<&TypeRef>,
    selection_span: Span,
    actions: &mut Vec<ExplicitTypeArgCodeAction>,
) {
    match stmt {
        Stmt::ValDecl { ty, init, .. } | Stmt::VarDecl { ty, init: Some(init), .. } => {
            collect_expr_explicit_type_arg_actions(
                init,
                ty.as_ref(),
                callable_signatures,
                selection_span,
                actions,
            );
        }
        Stmt::VarDecl { init: None, .. }
        | Stmt::StopAll { .. }
        | Stmt::IntrinsicBlock { .. }
        | Stmt::Break { .. }
        | Stmt::Continue { .. }
        | Stmt::Unlisten { .. } => {}
        Stmt::Assignment { target, value, .. } => {
            collect_expr_explicit_type_arg_actions(target, None, callable_signatures, selection_span, actions);
            collect_expr_explicit_type_arg_actions(value, None, callable_signatures, selection_span, actions);
        }
        Stmt::Expr { expr, .. } => {
            collect_expr_explicit_type_arg_actions(expr, None, callable_signatures, selection_span, actions);
        }
        Stmt::If { cond, then_block, else_branch, .. } => {
            collect_expr_explicit_type_arg_actions(cond, None, callable_signatures, selection_span, actions);
            collect_block_explicit_type_arg_actions(
                then_block,
                callable_signatures,
                expected_return_ty,
                selection_span,
                actions,
            );
            if let Some(else_branch) = else_branch {
                match else_branch {
                    ElseBranch::ElseBlock(block) => collect_block_explicit_type_arg_actions(
                        block,
                        callable_signatures,
                        expected_return_ty,
                        selection_span,
                        actions,
                    ),
                    ElseBranch::ElseIf(stmt) => collect_stmt_explicit_type_arg_actions(
                        stmt,
                        callable_signatures,
                        expected_return_ty,
                        selection_span,
                        actions,
                    ),
                }
            }
        }
        Stmt::When { subject, branches, .. } => {
            if let Some(subject) = subject {
                collect_expr_explicit_type_arg_actions(subject, None, callable_signatures, selection_span, actions);
            }
            for branch in branches {
                if let WhenPattern::Expression(expr) = &branch.pattern {
                    collect_expr_explicit_type_arg_actions(expr, None, callable_signatures, selection_span, actions);
                }
                if let Some(guard) = &branch.guard {
                    collect_expr_explicit_type_arg_actions(guard, None, callable_signatures, selection_span, actions);
                }
                match &branch.body {
                    WhenBody::Block(block) => collect_block_explicit_type_arg_actions(
                        block,
                        callable_signatures,
                        expected_return_ty,
                        selection_span,
                        actions,
                    ),
                    WhenBody::Expr(expr) => collect_expr_explicit_type_arg_actions(
                        expr,
                        None,
                        callable_signatures,
                        selection_span,
                        actions,
                    ),
                }
            }
        }
        Stmt::For { iterable, body, .. } => {
            collect_expr_explicit_type_arg_actions(iterable, None, callable_signatures, selection_span, actions);
            collect_block_explicit_type_arg_actions(
                body,
                callable_signatures,
                expected_return_ty,
                selection_span,
                actions,
            );
        }
        Stmt::DestructureVal { init, .. } => {
            collect_expr_explicit_type_arg_actions(init, None, callable_signatures, selection_span, actions);
        }
        Stmt::While { cond, body, .. } => {
            collect_expr_explicit_type_arg_actions(cond, None, callable_signatures, selection_span, actions);
            collect_block_explicit_type_arg_actions(
                body,
                callable_signatures,
                expected_return_ty,
                selection_span,
                actions,
            );
        }
        Stmt::Return { value: Some(expr), .. } => {
            collect_expr_explicit_type_arg_actions(
                expr,
                expected_return_ty,
                callable_signatures,
                selection_span,
                actions,
            );
        }
        Stmt::Return { value: None, .. } => {}
        Stmt::Wait { form, .. } => match form {
            WaitForm::Duration(expr) | WaitForm::Until(expr) | WaitForm::While(expr) => {
                collect_expr_explicit_type_arg_actions(expr, None, callable_signatures, selection_span, actions);
            }
            WaitForm::NextFrame | WaitForm::FixedFrame => {}
        },
        Stmt::Start { call, .. } => {
            collect_expr_explicit_type_arg_actions(call, None, callable_signatures, selection_span, actions);
        }
        Stmt::Stop { target, .. } => {
            collect_expr_explicit_type_arg_actions(target, None, callable_signatures, selection_span, actions);
        }
        Stmt::Listen { event, body, .. } => {
            collect_expr_explicit_type_arg_actions(event, None, callable_signatures, selection_span, actions);
            collect_block_explicit_type_arg_actions(body, callable_signatures, None, selection_span, actions);
        }
        Stmt::Try { try_block, catches, finally_block, .. } => {
            collect_block_explicit_type_arg_actions(try_block, callable_signatures, expected_return_ty, selection_span, actions);
            for c in catches {
                collect_block_explicit_type_arg_actions(&c.body, callable_signatures, expected_return_ty, selection_span, actions);
            }
            if let Some(fb) = finally_block {
                collect_block_explicit_type_arg_actions(fb, callable_signatures, expected_return_ty, selection_span, actions);
            }
        }
        Stmt::Throw { expr, .. } => {
            collect_expr_explicit_type_arg_actions(expr, None, callable_signatures, selection_span, actions);
        }
        Stmt::Use { ty, init, body, .. } => {
            collect_expr_explicit_type_arg_actions(
                init,
                ty.as_ref(),
                callable_signatures,
                selection_span,
                actions,
            );
            if let Some(body) = body {
                collect_block_explicit_type_arg_actions(
                    body,
                    callable_signatures,
                    expected_return_ty,
                    selection_span,
                    actions,
                );
            }
        }
        Stmt::BindTo { target, .. } => {
            collect_expr_explicit_type_arg_actions(
                target,
                None,
                callable_signatures,
                selection_span,
                actions,
            );
        }
        // Language 5, Sprint 1: yield + preprocessor walks.
        Stmt::Yield { value, .. } => {
            collect_expr_explicit_type_arg_actions(
                value,
                None,
                callable_signatures,
                selection_span,
                actions,
            );
        }
        Stmt::YieldBreak { .. } => {}
        Stmt::Preprocessor { arms, else_arm, .. } => {
            for arm in arms {
                for s in &arm.body {
                    collect_stmt_explicit_type_arg_actions(
                        s,
                        callable_signatures,
                        expected_return_ty,
                        selection_span,
                        actions,
                    );
                }
            }
            if let Some(else_stmts) = else_arm {
                for s in else_stmts {
                    collect_stmt_explicit_type_arg_actions(
                        s,
                        callable_signatures,
                        expected_return_ty,
                        selection_span,
                        actions,
                    );
                }
            }
        }
    }
}

fn collect_expr_explicit_type_arg_actions(
    expr: &Expr,
    expected_type: Option<&TypeRef>,
    callable_signatures: &HashMap<String, LspCallableSignature>,
    selection_span: Span,
    actions: &mut Vec<ExplicitTypeArgCodeAction>,
) {
    match expr {
        Expr::IntLit(_, _)
        | Expr::FloatLit(_, _)
        | Expr::DurationLit(_, _)
        | Expr::StringLit(_, _)
        | Expr::BoolLit(_, _)
        | Expr::Null(_)
        | Expr::Ident(_, _)
        | Expr::This(_)
        | Expr::IntrinsicExpr { .. }
        // Language 5, Sprint 2: nameof has no type arguments to surface.
        | Expr::NameOf { .. } => {}
        Expr::StringInterp { parts, .. } => {
            for part in parts {
                if let StringPart::Expr(expr) = part {
                    collect_expr_explicit_type_arg_actions(
                        expr,
                        None,
                        callable_signatures,
                        selection_span,
                        actions,
                    );
                }
            }
        }
        Expr::Binary { left, right, .. } => {
            collect_expr_explicit_type_arg_actions(left, None, callable_signatures, selection_span, actions);
            collect_expr_explicit_type_arg_actions(right, None, callable_signatures, selection_span, actions);
        }
        Expr::Unary { operand, .. } => {
            collect_expr_explicit_type_arg_actions(operand, None, callable_signatures, selection_span, actions);
        }
        Expr::MemberAccess { receiver, .. } | Expr::SafeCall { receiver, .. } => {
            collect_expr_explicit_type_arg_actions(receiver, None, callable_signatures, selection_span, actions);
        }
        Expr::SafeMethodCall {
            receiver,
            name,
            name_span,
            type_args,
            args,
            ..
        } => {
            maybe_push_explicit_type_arg_action(
                Some(receiver.as_ref()),
                name,
                *name_span,
                type_args,
                expected_type,
                selection_span,
                actions,
            );
            collect_expr_explicit_type_arg_actions(receiver, None, callable_signatures, selection_span, actions);
            let signature = lookup_lsp_callable_signature(Some(receiver.as_ref()), name, callable_signatures);
            for (index, arg) in args.iter().enumerate() {
                let arg_expected_type = signature.and_then(|signature| signature.params.get(index));
                collect_expr_explicit_type_arg_actions(
                    &arg.value,
                    arg_expected_type,
                    callable_signatures,
                    selection_span,
                    actions,
                );
            }
        }
        Expr::NonNullAssert { expr, .. } => {
            collect_expr_explicit_type_arg_actions(
                expr,
                expected_type,
                callable_signatures,
                selection_span,
                actions,
            );
        }
        Expr::Elvis { left, right, .. } => {
            collect_expr_explicit_type_arg_actions(
                left,
                expected_type,
                callable_signatures,
                selection_span,
                actions,
            );
            collect_expr_explicit_type_arg_actions(
                right,
                expected_type,
                callable_signatures,
                selection_span,
                actions,
            );
        }
        Expr::Call {
            receiver,
            name,
            name_span,
            type_args,
            args,
            ..
        } => {
            maybe_push_explicit_type_arg_action(
                receiver.as_deref(),
                name,
                *name_span,
                type_args,
                expected_type,
                selection_span,
                actions,
            );
            if let Some(receiver) = receiver {
                collect_expr_explicit_type_arg_actions(receiver, None, callable_signatures, selection_span, actions);
            }
            let signature = lookup_lsp_callable_signature(receiver.as_deref(), name, callable_signatures);
            for (index, arg) in args.iter().enumerate() {
                let arg_expected_type = signature.and_then(|signature| signature.params.get(index));
                collect_expr_explicit_type_arg_actions(
                    &arg.value,
                    arg_expected_type,
                    callable_signatures,
                    selection_span,
                    actions,
                );
            }
        }
        Expr::IndexAccess { receiver, index, .. } => {
            collect_expr_explicit_type_arg_actions(receiver, None, callable_signatures, selection_span, actions);
            collect_expr_explicit_type_arg_actions(index, None, callable_signatures, selection_span, actions);
        }
        Expr::IfExpr { cond, then_block, else_block, .. } => {
            collect_expr_explicit_type_arg_actions(cond, None, callable_signatures, selection_span, actions);
            collect_value_block_explicit_type_arg_actions(
                then_block,
                callable_signatures,
                expected_type,
                selection_span,
                actions,
            );
            collect_value_block_explicit_type_arg_actions(
                else_block,
                callable_signatures,
                expected_type,
                selection_span,
                actions,
            );
        }
        Expr::WhenExpr { subject, branches, .. } => {
            if let Some(subject) = subject {
                collect_expr_explicit_type_arg_actions(subject, None, callable_signatures, selection_span, actions);
            }
            for branch in branches {
                if let WhenPattern::Expression(expr) = &branch.pattern {
                    collect_expr_explicit_type_arg_actions(expr, None, callable_signatures, selection_span, actions);
                }
                if let Some(guard) = &branch.guard {
                    collect_expr_explicit_type_arg_actions(guard, None, callable_signatures, selection_span, actions);
                }
                match &branch.body {
                    WhenBody::Expr(expr) => collect_expr_explicit_type_arg_actions(
                        expr,
                        expected_type,
                        callable_signatures,
                        selection_span,
                        actions,
                    ),
                    WhenBody::Block(block) => collect_value_block_explicit_type_arg_actions(
                        block,
                        callable_signatures,
                        expected_type,
                        selection_span,
                        actions,
                    ),
                }
            }
        }
        Expr::Range { start, end, step, .. } => {
            collect_expr_explicit_type_arg_actions(start, None, callable_signatures, selection_span, actions);
            collect_expr_explicit_type_arg_actions(end, None, callable_signatures, selection_span, actions);
            if let Some(step) = step {
                collect_expr_explicit_type_arg_actions(step, None, callable_signatures, selection_span, actions);
            }
        }
        Expr::Is { expr, .. } => {
            collect_expr_explicit_type_arg_actions(expr, None, callable_signatures, selection_span, actions);
        }
        Expr::Lambda { body, .. } => match body {
            LambdaBody::Block(block) => {
                collect_block_explicit_type_arg_actions(block, callable_signatures, None, selection_span, actions);
            }
            LambdaBody::Expr(e) => {
                collect_expr_explicit_type_arg_actions(e, None, callable_signatures, selection_span, actions);
            }
        },
        Expr::SafeCastExpr { expr, .. } | Expr::ForceCastExpr { expr, .. } => {
            collect_expr_explicit_type_arg_actions(expr, None, callable_signatures, selection_span, actions);
        }
        Expr::Tuple { elements, .. } => {
            for e in elements {
                collect_expr_explicit_type_arg_actions(e, None, callable_signatures, selection_span, actions);
            }
        }
        Expr::ListLit { elements, .. } => {
            for e in elements {
                collect_expr_explicit_type_arg_actions(e, None, callable_signatures, selection_span, actions);
            }
        }
        Expr::MapLit { entries, .. } => {
            for (k, v) in entries {
                collect_expr_explicit_type_arg_actions(k, None, callable_signatures, selection_span, actions);
                collect_expr_explicit_type_arg_actions(v, None, callable_signatures, selection_span, actions);
            }
        }
        Expr::Await { expr: inner, .. } => {
            collect_expr_explicit_type_arg_actions(inner, None, callable_signatures, selection_span, actions);
        }
    }
}

fn maybe_push_explicit_type_arg_action(
    receiver: Option<&Expr>,
    name: &str,
    name_span: Span,
    type_args: &[TypeRef],
    expected_type: Option<&TypeRef>,
    selection_span: Span,
    actions: &mut Vec<ExplicitTypeArgCodeAction>,
) {
    if name.is_empty()
        || !type_args.is_empty()
        || !supports_expected_type_inference_lsp(receiver, name)
        || !selection_targets_span(selection_span, name_span)
    {
        return;
    }

    let Some(expected_type) = expected_type else {
        return;
    };

    let explicit_type = format_type_ref_source(&strip_nullable_type_ref(expected_type));
    if explicit_type.is_empty() || explicit_type == "Unit" {
        return;
    }

    actions.push(ExplicitTypeArgCodeAction {
        title: format!("Add explicit type argument <{}>", explicit_type),
        insert_at: name_span.end,
        insert_text: format!("<{}>", explicit_type),
    });
}

fn lookup_lsp_callable_signature<'a>(
    receiver: Option<&Expr>,
    name: &str,
    callable_signatures: &'a HashMap<String, LspCallableSignature>,
) -> Option<&'a LspCallableSignature> {
    match receiver {
        None => callable_signatures.get(name),
        Some(Expr::This(_)) => callable_signatures.get(name),
        _ => None,
    }
}

fn supports_expected_type_inference_lsp(receiver: Option<&Expr>, name: &str) -> bool {
    match receiver {
        None => matches!(name, "get" | "require" | "find" | "child" | "parent" | "loadJson"),
        Some(_) => matches!(
            name,
            "getComponent" | "getComponentInChildren" | "getComponentInParent" | "findFirstObjectByType"
        ),
    }
}

fn strip_nullable_type_ref(ty: &TypeRef) -> TypeRef {
    match ty {
        TypeRef::Simple { name, span, .. } => TypeRef::Simple {
            name: name.clone(),
            nullable: false,
            span: *span,
        },
        TypeRef::Generic { name, type_args, span, .. } => TypeRef::Generic {
            name: name.clone(),
            type_args: type_args.clone(),
            nullable: false,
            span: *span,
        },
        TypeRef::Qualified {
            qualifier,
            name,
            span,
            ..
        } => TypeRef::Qualified {
            qualifier: qualifier.clone(),
            name: name.clone(),
            nullable: false,
            span: *span,
        },
        TypeRef::Tuple { types, span, .. } => TypeRef::Tuple {
            types: types.clone(),
            nullable: false,
            span: *span,
        },
        TypeRef::Function { param_types, return_type, span, .. } => TypeRef::Function {
            param_types: param_types.clone(),
            return_type: return_type.clone(),
            nullable: false,
            span: *span,
        },
    }
}

fn format_type_ref_source(ty: &TypeRef) -> String {
    match ty {
        TypeRef::Simple { name, nullable, .. } => format_nullable_type_ref_source(name.clone(), *nullable),
        TypeRef::Generic {
            name,
            type_args,
            nullable,
            ..
        } => format_nullable_type_ref_source(
            format!(
                "{}<{}>",
                name,
                type_args
                    .iter()
                    .map(format_type_ref_source)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            *nullable,
        ),
        TypeRef::Qualified {
            qualifier,
            name,
            nullable,
            ..
        } => format_nullable_type_ref_source(format!("{}.{}", qualifier, name), *nullable),
        TypeRef::Tuple { types, nullable, .. } => {
            let inner: Vec<String> = types.iter().map(format_type_ref_source).collect();
            format_nullable_type_ref_source(format!("({})", inner.join(", ")), *nullable)
        }
        TypeRef::Function { param_types, return_type, nullable, .. } => {
            let inner: Vec<String> = param_types.iter().map(format_type_ref_source).collect();
            format_nullable_type_ref_source(
                format!("({}) => {}", inner.join(", "), format_type_ref_source(return_type)),
                *nullable,
            )
        }
    }
}

fn format_nullable_type_ref_source(base: String, nullable: bool) -> String {
    if nullable {
        format!("{}?", base)
    } else {
        base
    }
}

fn selection_targets_span(selection_span: Span, target_span: Span) -> bool {
    if positions_equal(selection_span.start, selection_span.end) {
        return point_within_span(selection_span.start, target_span);
    }

    spans_overlap(selection_span, target_span)
}

fn positions_equal(left: Position, right: Position) -> bool {
    left.line == right.line && left.col == right.col
}

fn point_within_span(position: Position, span: Span) -> bool {
    compare_positions(position, span.start) != std::cmp::Ordering::Less
        && compare_positions(position, span.end) != std::cmp::Ordering::Greater
}

fn spans_overlap(left: Span, right: Span) -> bool {
    compare_positions(left.end, right.start) != std::cmp::Ordering::Less
        && compare_positions(right.end, left.start) != std::cmp::Ordering::Less
}

fn explicit_type_arg_code_action_json(uri: &str, action: ExplicitTypeArgCodeAction) -> Value {
    let mut changes = HashMap::new();
    changes.insert(
        uri.to_string(),
        vec![json!({
            "range": lsp_text_edit_range_json(action.insert_at, action.insert_at),
            "newText": action.insert_text,
        })],
    );

    json!({
        "title": action.title,
        "kind": CODE_ACTION_KIND_REFACTOR_REWRITE,
        "isPreferred": true,
        "edit": {
            "changes": changes,
        },
    })
}

fn organize_usings_code_action_json(uri: &str, action: OrganizeUsingsCodeAction) -> Value {
    let mut changes = HashMap::new();
    changes.insert(
        uri.to_string(),
        vec![json!({
            "range": lsp_range_json(action.range),
            "newText": action.new_text,
        })],
    );

    json!({
        "title": "Organize using declarations",
        "kind": CODE_ACTION_KIND_SOURCE_ORGANIZE_IMPORTS,
        "edit": {
            "changes": changes,
        },
    })
}

fn build_hover_markdown(
    symbol_at: Option<&IndexedSymbol>,
    reference_at: Option<&IndexedReference>,
    resolved_symbol: Option<&IndexedSymbol>,
    definition: Option<&HirDefinition>,
    project_index: &ProjectIndex,
    sidecar_hover_section: Option<String>,
) -> Option<String> {
    if let Some(symbol) = symbol_at {
        return Some(join_hover_sections([
            Some(format_prsm_hover_signature(&symbol.signature)),
            sidecar_hover_section
                .clone()
                .or_else(|| unity_hover_section_for_symbol(symbol, definition, project_index)),
        ]));
    }

    if let (Some(reference), Some(symbol)) = (reference_at, resolved_symbol) {
        return Some(join_hover_sections([
            Some(format_prsm_hover_signature(&symbol.signature)),
            sidecar_hover_section
                .clone()
                .or_else(|| unity_hover_section_for_symbol(symbol, definition, project_index))
                .or_else(|| unity_hover_section_for_reference(reference)),
        ]));
    }

    if let Some(definition) = definition {
        return Some(join_hover_sections([
            Some(format_prsm_hover_signature(&hover_signature_for_definition(definition))),
            sidecar_hover_section
                .clone()
                .or_else(|| unity_hover_section_for_definition(definition)),
        ]));
    }

    reference_at.map(|reference| {
        let supplemental = sidecar_hover_section.or_else(|| unity_hover_section_for_reference(reference));
        let unresolved_message = if supplemental.is_some() {
            None
        } else {
            Some("_Definition not found in the current PrSM project index._".to_string())
        };

        join_hover_sections([
            Some(format_prsm_hover_signature(&reference.name)),
            supplemental,
            unresolved_message,
        ])
    })
}

fn join_hover_sections(sections: impl IntoIterator<Item = Option<String>>) -> String {
    sections
        .into_iter()
        .flatten()
        .filter(|section| !section.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn format_prsm_hover_signature(signature: &str) -> String {
    format!("```prsm\n{}\n```", signature.trim())
}

fn hover_signature_for_definition(definition: &HirDefinition) -> String {
    format!("{} {}", definition.kind.as_str(), definition.qualified_name)
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
    let mut sections = vec![format!("**[{}]**", title)];
    if let Some(signature) = hover.signature.as_ref().map(|value| value.trim()).filter(|value| !value.is_empty()) {
        sections.push(format!("```csharp\n{}\n```", signature));
    }
    if let Some(documentation) = hover.documentation.as_ref().map(|value| value.trim()).filter(|value| !value.is_empty()) {
        sections.push(documentation.to_string());
    }
    if let Some(docs_url) = hover.docs_url.as_ref() {
        sections.push(format!("[Docs]({})", docs_url));
    }
    if sections.len() == 1 {
        sections.push(hover.display_name.clone());
    }

    sections.join("\n\n")
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
            "**[Unity API]**".to_string(),
            match label {
                "Type" => lookup_name,
                _ => format!("{} ({})", lookup_name, label),
            },
            format!("[Docs]({})", docs_url),
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
        crate::project_index::DeclarationKind::Interface => "interface",
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

fn lsp_text_edit_range_json(start: Position, end: Position) -> Value {
    json!({
        "start": {
            "line": start.line.saturating_sub(1),
            "character": start.col.saturating_sub(1),
        },
        "end": {
            "line": end.line.saturating_sub(1),
            "character": end.col.saturating_sub(1),
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
    use super::{
        collect_explicit_type_arg_actions_for_source, collect_organize_usings_action_for_source,
        format_sidecar_hover_section, merge_completion_items, parse_sidecar_args,
        prsm_label_for_sidecar_item, requested_code_action_allows,
    };
    use crate::lexer::token::{Position, Span};
    use crate::roslyn_sidecar_protocol::{SidecarCompletionItemKind, SidecarSymbolKind, SidecarSymbolSource, UnityCompletionItem, UnityHoverResult};
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

    #[test]
    fn lsp_sidecar_hover_sections_use_compact_layout() {
        let markdown = format_sidecar_hover_section(
            "Generated C#",
            &UnityHoverResult {
                display_name: "Player".to_string(),
                kind: SidecarSymbolKind::Class,
                source: SidecarSymbolSource::Generated,
                namespace: Some("Game".to_string()),
                signature: Some("public class Player : MonoBehaviour".to_string()),
                documentation: Some("Unity script".to_string()),
                assembly: Some("Game.Assembly".to_string()),
                docs_url: Some("https://example.com/player".to_string()),
                is_static: false,
            },
        );

        assert!(markdown.contains("**[Generated C#]**"));
        assert!(markdown.contains("public class Player : MonoBehaviour"));
        assert!(markdown.contains("Unity script"));
        assert!(markdown.contains("[Docs](https://example.com/player)"));
        assert!(!markdown.contains("**Symbol:**"));
        assert!(!markdown.contains("**Assembly:**"));
    }

    #[test]
    fn lsp_code_actions_add_explicit_type_arg_for_typed_initializer() {
        let actions = explicit_type_arg_actions_from_marked_source(
            r#"component Player : MonoBehaviour {
    awake {
        val weapon: WeaponData = |get|()
    }
}"#,
        );

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].title, "Add explicit type argument <WeaponData>");
        assert_eq!(actions[0].insert_text, "<WeaponData>");
    }

    #[test]
    fn lsp_code_actions_add_explicit_type_arg_for_expr_body_return() {
        let actions = explicit_type_arg_actions_from_marked_source(
            r#"component Player : MonoBehaviour {
    func load(): WeaponData = |get|()
}"#,
        );

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].insert_text, "<WeaponData>");
    }

    #[test]
    fn lsp_code_actions_add_explicit_type_arg_for_argument_context() {
        let actions = explicit_type_arg_actions_from_marked_source(
            r#"component Player : MonoBehaviour {
    func equip() {
        setCurrent(|get|())
    }

    func setCurrent(weapon: WeaponData) {
    }
}"#,
        );

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].insert_text, "<WeaponData>");
    }

    #[test]
    fn lsp_code_actions_skip_calls_with_existing_type_args() {
        let actions = explicit_type_arg_actions_from_marked_source(
            r#"component Player : MonoBehaviour {
    func awake() {
        val weapon: WeaponData = |get|<WeaponData>()
    }
}"#,
        );

        assert!(actions.is_empty());
    }

    #[test]
    fn lsp_code_actions_offer_organize_usings_when_sort_or_dedupe_needed() {
        let action = collect_organize_usings_action_for_source(
            r#"using UnityEngine.UI
using UnityEngine
using UnityEngine.UI
component Player : MonoBehaviour {
}"#,
        )
        .expect("organize imports action should exist");

        assert_eq!(action.new_text, "using UnityEngine\n\n");
    }

    #[test]
    fn lsp_code_actions_keep_used_unity_type_namespaces() {
        let action = collect_organize_usings_action_for_source(
            r#"using UnityEngine.Events
using UnityEngine.UI
using UnityEngine
component Player : MonoBehaviour {
    func bind(button: Button) {
    }
}"#,
        )
        .expect("organize imports action should exist");

        assert_eq!(
            action.new_text,
            "using UnityEngine\nusing UnityEngine.UI\n\n"
        );
    }

    #[test]
    fn lsp_code_actions_keep_input_system_for_input_action_sugar() {
        let action = collect_organize_usings_action_for_source(
            r#"using UnityEngine.InputSystem
using UnityEngine.UI
using UnityEngine
component Player : MonoBehaviour {
    update {
        if input.action("Jump").pressed {
        }
    }
}"#,
        )
        .expect("organize imports action should exist");

        assert_eq!(
            action.new_text,
            "using UnityEngine\nusing UnityEngine.InputSystem\n\n"
        );
    }

    #[test]
    fn lsp_code_actions_skip_unused_pruning_when_intrinsic_code_is_present() {
        let action = collect_organize_usings_action_for_source(
            r#"using UnityEngine.UI
using UnityEngine
component Player : MonoBehaviour {
    func bind() {
        intrinsic {
            Debug.Log("raw");
        }
    }
}"#,
        )
        .expect("organize imports action should exist");

        assert_eq!(
            action.new_text,
            "using UnityEngine\nusing UnityEngine.UI\n\n"
        );
    }

    #[test]
    fn lsp_code_actions_skip_organize_usings_when_already_normalized() {
        let action = collect_organize_usings_action_for_source(
            r#"using UnityEngine
using UnityEngine.UI

component Player : MonoBehaviour {
    func bind(button: Button) {
    }
}"#,
        );

        assert!(action.is_none());
    }

    #[test]
    fn lsp_code_actions_skip_organize_usings_when_comments_would_be_lost() {
        let action = collect_organize_usings_action_for_source(
            r#"using UnityEngine
// keep this note
using UnityEngine.UI
component Player : MonoBehaviour {
}"#,
        );

        assert!(action.is_none());
    }

    #[test]
    fn lsp_code_action_kind_matching_allows_parent_kinds() {
        assert!(requested_code_action_allows(
            Some(&["source".to_string()]),
            super::CODE_ACTION_KIND_SOURCE_ORGANIZE_IMPORTS,
        ));
        assert!(requested_code_action_allows(
            Some(&["refactor".to_string()]),
            super::CODE_ACTION_KIND_REFACTOR_REWRITE,
        ));
        assert!(!requested_code_action_allows(
            Some(&["quickfix".to_string()]),
            super::CODE_ACTION_KIND_SOURCE_ORGANIZE_IMPORTS,
        ));
    }

    fn explicit_type_arg_actions_from_marked_source(marked: &str) -> Vec<super::ExplicitTypeArgCodeAction> {
        let (source, selection) = source_and_selection_from_markers(marked);
        collect_explicit_type_arg_actions_for_source(&source, selection)
    }

    fn source_and_selection_from_markers(marked: &str) -> (String, Span) {
        let mut source = String::new();
        let mut start = None;
        let mut end = None;
        let mut line = 1u32;
        let mut col = 1u32;

        for ch in marked.chars() {
            if ch == '|' {
                if start.is_none() {
                    let position = Position { line, col };
                    start = Some(position);
                    end = Some(position);
                } else {
                    end = Some(Position { line, col });
                }
                continue;
            }

            source.push(ch);
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }

        (
            source,
            Span {
                start: start.expect("missing start marker"),
                end: end.expect("missing end marker"),
            },
        )
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