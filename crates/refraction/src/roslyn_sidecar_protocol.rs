use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const JSONRPC_VERSION: &str = "2.0";
pub const SIDECAR_PROTOCOL_VERSION: u32 = 1;

pub const METHOD_HEALTH_PING: &str = "health/ping";
pub const METHOD_SIDECAR_INITIALIZE: &str = "sidecar/initialize";
pub const METHOD_SIDECAR_LOAD_PROJECT: &str = "sidecar/loadProject";
pub const METHOD_SIDECAR_SHUTDOWN: &str = "sidecar/shutdown";
pub const METHOD_UNITY_COMPLETE_MEMBERS: &str = "unity/completeMembers";
pub const METHOD_UNITY_GET_DEFINITION: &str = "unity/getDefinition";
pub const METHOD_UNITY_GET_HOVER: &str = "unity/getHover";
pub const METHOD_UNITY_GET_TYPE: &str = "unity/getType";
pub const METHOD_UNITY_RESOLVE_GENERATED_SYMBOL: &str = "unity/resolveGeneratedSymbol";
pub const METHOD_WORKSPACE_RELOAD: &str = "workspace/reload";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcId {
    Number(u64),
    String(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcRequest<T> {
    pub jsonrpc: String,
    pub id: JsonRpcId,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<T>,
}

impl<T> JsonRpcRequest<T> {
    pub fn new(id: JsonRpcId, method: impl Into<String>, params: T) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            method: method.into(),
            params: Some(params),
        }
    }

    pub fn without_params(id: JsonRpcId, method: impl Into<String>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            method: method.into(),
            params: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcNotification<T> {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<T>,
}

impl<T> JsonRpcNotification<T> {
    pub fn new(method: impl Into<String>, params: T) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: method.into(),
            params: Some(params),
        }
    }

    pub fn without_params(method: impl Into<String>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: method.into(),
            params: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcSuccess<T> {
    pub jsonrpc: String,
    pub id: JsonRpcId,
    pub result: T,
}

impl<T> JsonRpcSuccess<T> {
    pub fn new(id: JsonRpcId, result: T) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            result,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonRpcErrorResponse {
    pub jsonrpc: String,
    pub id: JsonRpcId,
    pub error: JsonRpcError,
}

impl JsonRpcErrorResponse {
    pub fn new(id: JsonRpcId, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            error: JsonRpcError {
                code,
                message: message.into(),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SidecarCapabilities {
    pub metadata_hover: bool,
    pub metadata_completion: bool,
    pub generated_symbol_lookup: bool,
    pub xml_documentation: bool,
    pub workspace_reload: bool,
}

impl Default for SidecarCapabilities {
    fn default() -> Self {
        Self {
            metadata_hover: true,
            metadata_completion: true,
            generated_symbol_lookup: true,
            xml_documentation: true,
            workspace_reload: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthPingParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthPingResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<String>,
    pub protocol_version: u32,
    pub sidecar_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sidecar_version: Option<String>,
    pub capabilities: SidecarCapabilities,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SidecarInitializeParams {
    pub protocol_version: u32,
    pub client_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SidecarInitializeResult {
    pub protocol_version: u32,
    pub sidecar_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sidecar_version: Option<String>,
    pub capabilities: SidecarCapabilities,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SidecarLoadProjectParams {
    pub workspace_root: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_file: Option<PathBuf>,
    pub unity_project_root: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_dir: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub generated_files: Vec<PathBuf>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub metadata_references: Vec<PathBuf>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub package_assemblies: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SidecarLoadProjectResult {
    pub project_id: String,
    pub loaded_documents: usize,
    pub metadata_reference_count: usize,
    pub generated_document_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SidecarShutdownResult {
    pub acknowledged: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkspaceReloadReason {
    ProjectConfigChanged,
    GeneratedSourcesChanged,
    MetadataReferencesChanged,
    PackageManifestChanged,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceReloadParams {
    pub reason: WorkspaceReloadReason,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changed_files: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceReloadResult {
    pub project_id: String,
    pub reloaded: bool,
    pub changed_document_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated_owner_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated_file: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SidecarCompletionItemKind {
    Class,
    Struct,
    Interface,
    Enum,
    Constructor,
    Method,
    Property,
    Field,
    Event,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SidecarSymbolKind {
    Class,
    Struct,
    Interface,
    Enum,
    Delegate,
    Method,
    Property,
    Field,
    Event,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SidecarSymbolSource {
    Metadata,
    Generated,
    Source,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SidecarLocation {
    pub file_path: PathBuf,
    pub line: u32,
    pub col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnityCompleteMembersParams {
    pub type_name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub prefix: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<GeneratedContext>,
    #[serde(default = "default_true")]
    pub include_instance_members: bool,
    #[serde(default)]
    pub include_static_members: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnityCompletionItem {
    pub label: String,
    pub kind: SidecarCompletionItemKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insert_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assembly: Option<String>,
    #[serde(default)]
    pub is_static: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnityCompleteMembersResult {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<UnityCompletionItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnityGetHoverParams {
    pub type_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<GeneratedContext>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnityHoverResult {
    pub display_name: String,
    pub kind: SidecarSymbolKind,
    pub source: SidecarSymbolSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assembly: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docs_url: Option<String>,
    #[serde(default)]
    pub is_static: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnityGetTypeParams {
    pub type_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace_hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnityTypeResult {
    pub display_name: String,
    pub kind: SidecarSymbolKind,
    pub source: SidecarSymbolSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assembly: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_type: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub interfaces: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docs_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnityGetDefinitionParams {
    pub type_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<GeneratedContext>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnityDefinitionResult {
    pub source: SidecarSymbolSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<SidecarLocation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assembly: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docs_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnityResolveGeneratedSymbolParams {
    pub generated_file: PathBuf,
    pub type_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedGeneratedSymbol {
    pub display_name: String,
    pub kind: SidecarSymbolKind,
    pub source: SidecarSymbolSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<SidecarLocation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assembly: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docs_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnityResolveGeneratedSymbolResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<ResolvedGeneratedSymbol>,
}

const fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn serialize_complete_members_request_matches_contract() {
        let request = JsonRpcRequest::new(
            JsonRpcId::Number(1),
            METHOD_UNITY_COMPLETE_MEMBERS,
            UnityCompleteMembersParams {
                type_name: "Button".to_string(),
                prefix: "on".to_string(),
                context: Some(GeneratedContext {
                    generated_owner_type: Some("PlayerUI".to_string()),
                    generated_file: Some(PathBuf::from("C:/Project/Assets/Generated/PrSM/PlayerUI.cs")),
                }),
                include_instance_members: true,
                include_static_members: false,
            },
        );

        let value = serde_json::to_value(request).expect("request should serialize");
        assert_eq!(value["jsonrpc"], json!(JSONRPC_VERSION));
        assert_eq!(value["method"], json!(METHOD_UNITY_COMPLETE_MEMBERS));
        assert_eq!(value["params"]["type_name"], json!("Button"));
        assert_eq!(value["params"]["prefix"], json!("on"));
        assert_eq!(value["params"]["context"]["generated_owner_type"], json!("PlayerUI"));
    }

    #[test]
    fn deserialize_hover_success_response() {
        let response = json!({
            "jsonrpc": "2.0",
            "id": 7,
            "result": {
                "display_name": "UnityEngine.Transform.position",
                "kind": "property",
                "source": "metadata",
                "namespace": "UnityEngine",
                "signature": "public Vector3 position { get; set; }",
                "documentation": "The world space position of the Transform.",
                "assembly": "UnityEngine.CoreModule",
                "docs_url": "https://docs.unity3d.com/ScriptReference/Transform-position.html",
                "is_static": false
            }
        });

        let parsed: JsonRpcSuccess<UnityHoverResult> =
            serde_json::from_value(response).expect("response should deserialize");

        assert_eq!(parsed.id, JsonRpcId::Number(7));
        assert_eq!(parsed.result.kind, SidecarSymbolKind::Property);
        assert_eq!(parsed.result.source, SidecarSymbolSource::Metadata);
        assert_eq!(parsed.result.namespace.as_deref(), Some("UnityEngine"));
    }

    #[test]
    fn roundtrip_load_project_params_preserves_paths() {
        let params = SidecarLoadProjectParams {
            workspace_root: PathBuf::from("C:/Project"),
            project_file: Some(PathBuf::from("C:/Project/.prsmproject")),
            unity_project_root: PathBuf::from("C:/Project"),
            output_dir: Some(PathBuf::from("C:/Project/Assets/Generated/PrSM")),
            generated_files: vec![PathBuf::from("C:/Project/Assets/Generated/PrSM/Player.cs")],
            metadata_references: vec![
                PathBuf::from("C:/Unity/Editor/Data/Managed/UnityEngine/UnityEngine.CoreModule.dll"),
                PathBuf::from("C:/Unity/Editor/Data/Managed/UnityEngine/UnityEngine.UI.dll"),
            ],
            package_assemblies: vec![PathBuf::from("C:/Project/Library/ScriptAssemblies/Assembly-CSharp.dll")],
        };

        let json = serde_json::to_string(&params).expect("params should serialize");
        let parsed: SidecarLoadProjectParams = serde_json::from_str(&json).expect("params should deserialize");

        assert_eq!(parsed, params);
    }
}