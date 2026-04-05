use crate::roslyn_sidecar_protocol::{
    HealthPingParams, HealthPingResult, JsonRpcErrorResponse, JsonRpcId, JsonRpcRequest, JsonRpcSuccess,
    SidecarInitializeParams, SidecarInitializeResult, SidecarLoadProjectParams, SidecarLoadProjectResult,
    SidecarShutdownResult, UnityCompleteMembersParams, UnityCompleteMembersResult, UnityDefinitionResult,
    UnityGetDefinitionParams, UnityGetHoverParams, UnityGetTypeParams, UnityHoverResult,
    UnityResolveGeneratedSymbolParams, UnityResolveGeneratedSymbolResult, UnityTypeResult,
    WorkspaceReloadParams, WorkspaceReloadResult, METHOD_HEALTH_PING, METHOD_SIDECAR_INITIALIZE,
    METHOD_SIDECAR_LOAD_PROJECT, METHOD_SIDECAR_SHUTDOWN, METHOD_UNITY_COMPLETE_MEMBERS,
    METHOD_UNITY_GET_DEFINITION, METHOD_UNITY_GET_HOVER, METHOD_UNITY_GET_TYPE,
    METHOD_UNITY_RESOLVE_GENERATED_SYMBOL, METHOD_WORKSPACE_RELOAD, JSONRPC_VERSION,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use std::fmt;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

#[derive(Debug)]
pub enum RoslynSidecarClientError {
    Io(std::io::Error),
    Json(serde_json::Error),
    MissingPipe(&'static str),
    Protocol(String),
    Remote { code: i32, message: String },
}

impl fmt::Display for RoslynSidecarClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "I/O error: {}", error),
            Self::Json(error) => write!(f, "JSON error: {}", error),
            Self::MissingPipe(name) => write!(f, "Sidecar process did not expose {}", name),
            Self::Protocol(message) => write!(f, "Protocol error: {}", message),
            Self::Remote { code, message } => write!(f, "Remote error {}: {}", code, message),
        }
    }
}

impl std::error::Error for RoslynSidecarClientError {}

impl From<std::io::Error> for RoslynSidecarClientError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for RoslynSidecarClientError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

#[derive(Debug, Clone, Default)]
pub struct RoslynSidecarCommand {
    program: PathBuf,
    args: Vec<String>,
    current_dir: Option<PathBuf>,
    env: Vec<(String, String)>,
}

impl RoslynSidecarCommand {
    pub fn new(program: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            current_dir: None,
            env: Vec::new(),
        }
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    pub fn current_dir(mut self, current_dir: impl Into<PathBuf>) -> Self {
        self.current_dir = Some(current_dir.into());
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    pub fn spawn(self) -> Result<StdioRoslynSidecarClient, RoslynSidecarClientError> {
        let mut command = Command::new(&self.program);
        command.args(&self.args);
        command.stdin(Stdio::piped());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::inherit());

        if let Some(current_dir) = &self.current_dir {
            command.current_dir(current_dir);
        }

        for (key, value) in &self.env {
            command.env(key, value);
        }

        let mut child = command.spawn()?;
        let stdin = child
            .stdin
            .take()
            .ok_or(RoslynSidecarClientError::MissingPipe("stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or(RoslynSidecarClientError::MissingPipe("stdout"))?;

        Ok(RoslynSidecarClient::new(StdioSidecarTransport::new(child, stdin, stdout)))
    }
}

pub trait SidecarTransport {
    fn send(&mut self, payload: &[u8]) -> Result<(), RoslynSidecarClientError>;
    fn receive(&mut self) -> Result<Vec<u8>, RoslynSidecarClientError>;
}

#[derive(Debug)]
pub struct StdioSidecarTransport {
    child: Child,
    writer: BufWriter<ChildStdin>,
    reader: BufReader<ChildStdout>,
}

impl StdioSidecarTransport {
    pub fn new(child: Child, stdin: ChildStdin, stdout: ChildStdout) -> Self {
        Self {
            child,
            writer: BufWriter::new(stdin),
            reader: BufReader::new(stdout),
        }
    }

    pub fn process_id(&self) -> u32 {
        self.child.id()
    }
}

impl SidecarTransport for StdioSidecarTransport {
    fn send(&mut self, payload: &[u8]) -> Result<(), RoslynSidecarClientError> {
        write_framed_message(&mut self.writer, payload)
    }

    fn receive(&mut self) -> Result<Vec<u8>, RoslynSidecarClientError> {
        read_framed_message(&mut self.reader)
    }
}

impl Drop for StdioSidecarTransport {
    fn drop(&mut self) {
        if let Ok(None) = self.child.try_wait() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

pub type StdioRoslynSidecarClient = RoslynSidecarClient<StdioSidecarTransport>;

#[derive(Debug)]
pub struct RoslynSidecarClient<T> {
    transport: T,
    next_request_id: u64,
}

impl<T> RoslynSidecarClient<T>
where
    T: SidecarTransport,
{
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            next_request_id: 1,
        }
    }

    pub fn into_transport(self) -> T {
        self.transport
    }

    pub fn ping(&mut self, params: HealthPingParams) -> Result<HealthPingResult, RoslynSidecarClientError> {
        self.request(METHOD_HEALTH_PING, params)
    }

    pub fn initialize(
        &mut self,
        params: SidecarInitializeParams,
    ) -> Result<SidecarInitializeResult, RoslynSidecarClientError> {
        self.request(METHOD_SIDECAR_INITIALIZE, params)
    }

    pub fn load_project(
        &mut self,
        params: SidecarLoadProjectParams,
    ) -> Result<SidecarLoadProjectResult, RoslynSidecarClientError> {
        self.request(METHOD_SIDECAR_LOAD_PROJECT, params)
    }

    pub fn shutdown(&mut self) -> Result<SidecarShutdownResult, RoslynSidecarClientError> {
        self.request_without_params(METHOD_SIDECAR_SHUTDOWN)
    }

    pub fn reload_workspace(
        &mut self,
        params: WorkspaceReloadParams,
    ) -> Result<WorkspaceReloadResult, RoslynSidecarClientError> {
        self.request(METHOD_WORKSPACE_RELOAD, params)
    }

    pub fn complete_members(
        &mut self,
        params: UnityCompleteMembersParams,
    ) -> Result<UnityCompleteMembersResult, RoslynSidecarClientError> {
        self.request(METHOD_UNITY_COMPLETE_MEMBERS, params)
    }

    pub fn get_hover(
        &mut self,
        params: UnityGetHoverParams,
    ) -> Result<UnityHoverResult, RoslynSidecarClientError> {
        self.request(METHOD_UNITY_GET_HOVER, params)
    }

    pub fn get_type(&mut self, params: UnityGetTypeParams) -> Result<UnityTypeResult, RoslynSidecarClientError> {
        self.request(METHOD_UNITY_GET_TYPE, params)
    }

    pub fn get_definition(
        &mut self,
        params: UnityGetDefinitionParams,
    ) -> Result<UnityDefinitionResult, RoslynSidecarClientError> {
        self.request(METHOD_UNITY_GET_DEFINITION, params)
    }

    pub fn resolve_generated_symbol(
        &mut self,
        params: UnityResolveGeneratedSymbolParams,
    ) -> Result<UnityResolveGeneratedSymbolResult, RoslynSidecarClientError> {
        self.request(METHOD_UNITY_RESOLVE_GENERATED_SYMBOL, params)
    }

    fn request<P, R>(&mut self, method: &str, params: P) -> Result<R, RoslynSidecarClientError>
    where
        P: Serialize,
        R: DeserializeOwned,
    {
        let id = self.next_id();
        let request = JsonRpcRequest::new(id.clone(), method.to_string(), params);
        let payload = serde_json::to_vec(&request)?;
        self.transport.send(&payload)?;
        self.read_response(id)
    }

    fn request_without_params<R>(&mut self, method: &str) -> Result<R, RoslynSidecarClientError>
    where
        R: DeserializeOwned,
    {
        let id = self.next_id();
        let request = JsonRpcRequest::<Value>::without_params(id.clone(), method.to_string());
        let payload = serde_json::to_vec(&request)?;
        self.transport.send(&payload)?;
        self.read_response(id)
    }

    fn read_response<R>(&mut self, expected_id: JsonRpcId) -> Result<R, RoslynSidecarClientError>
    where
        R: DeserializeOwned,
    {
        let payload = self.transport.receive()?;
        let value: Value = serde_json::from_slice(&payload)?;

        match value.get("jsonrpc").and_then(Value::as_str) {
            Some(JSONRPC_VERSION) => {}
            Some(version) => {
                return Err(RoslynSidecarClientError::Protocol(format!(
                    "Expected jsonrpc {} but received {}",
                    JSONRPC_VERSION, version
                )))
            }
            None => {
                return Err(RoslynSidecarClientError::Protocol(
                    "Response missing jsonrpc field".to_string(),
                ))
            }
        }

        if value.get("error").is_some() {
            let response: JsonRpcErrorResponse = serde_json::from_value(value)?;
            if response.id != expected_id {
                return Err(RoslynSidecarClientError::Protocol(format!(
                    "Response id mismatch: expected {:?}, received {:?}",
                    expected_id, response.id
                )));
            }
            return Err(RoslynSidecarClientError::Remote {
                code: response.error.code,
                message: response.error.message,
            });
        }

        let response: JsonRpcSuccess<R> = serde_json::from_value(value)?;
        if response.id != expected_id {
            return Err(RoslynSidecarClientError::Protocol(format!(
                "Response id mismatch: expected {:?}, received {:?}",
                expected_id, response.id
            )));
        }

        Ok(response.result)
    }

    fn next_id(&mut self) -> JsonRpcId {
        let id = JsonRpcId::Number(self.next_request_id);
        self.next_request_id = self.next_request_id.saturating_add(1);
        id
    }
}

fn write_framed_message<W: Write>(writer: &mut W, payload: &[u8]) -> Result<(), RoslynSidecarClientError> {
    write!(writer, "Content-Length: {}\r\n\r\n", payload.len())?;
    writer.write_all(payload)?;
    writer.flush()?;
    Ok(())
}

fn read_framed_message<R>(reader: &mut R) -> Result<Vec<u8>, RoslynSidecarClientError>
where
    R: BufRead + Read,
{
    let mut content_length = None;

    loop {
        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line)?;
        if bytes_read == 0 {
            return Err(RoslynSidecarClientError::Protocol(
                "Unexpected EOF while reading message header".to_string(),
            ));
        }

        if line == "\r\n" {
            break;
        }

        let trimmed = line.trim_end_matches(['\r', '\n']);
        if let Some(value) = trimmed.strip_prefix("Content-Length:") {
            let parsed = value.trim().parse::<usize>().map_err(|error| {
                RoslynSidecarClientError::Protocol(format!("Invalid Content-Length header: {}", error))
            })?;
            content_length = Some(parsed);
        }
    }

    let length = content_length.ok_or_else(|| {
        RoslynSidecarClientError::Protocol("Missing Content-Length header".to_string())
    })?;

    let mut payload = vec![0; length];
    reader.read_exact(&mut payload)?;
    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::roslyn_sidecar_protocol::{SidecarCapabilities, SidecarSymbolKind, SidecarSymbolSource};
    use serde_json::json;
    use std::collections::VecDeque;
    use std::io::Cursor;

    #[derive(Debug, Default)]
    struct MockTransport {
        sent_payloads: Vec<Vec<u8>>,
        queued_responses: VecDeque<Vec<u8>>,
    }

    impl MockTransport {
        fn with_response(value: serde_json::Value) -> Self {
            Self {
                sent_payloads: Vec::new(),
                queued_responses: VecDeque::from([serde_json::to_vec(&value).expect("response should serialize")]),
            }
        }
    }

    impl SidecarTransport for MockTransport {
        fn send(&mut self, payload: &[u8]) -> Result<(), RoslynSidecarClientError> {
            self.sent_payloads.push(payload.to_vec());
            Ok(())
        }

        fn receive(&mut self) -> Result<Vec<u8>, RoslynSidecarClientError> {
            self.queued_responses.pop_front().ok_or_else(|| {
                RoslynSidecarClientError::Protocol("No queued response for mock transport".to_string())
            })
        }
    }

    #[test]
    fn write_and_read_framed_message_roundtrip() {
        let payload = br#"{"jsonrpc":"2.0","id":1,"method":"health/ping"}"#;
        let mut bytes = Vec::new();

        write_framed_message(&mut bytes, payload).expect("message should be framed");

        let mut reader = std::io::BufReader::new(Cursor::new(bytes));
        let parsed = read_framed_message(&mut reader).expect("message should be parsed");
        assert_eq!(parsed, payload);
    }

    #[test]
    fn initialize_sends_expected_method_and_payload() {
        let response = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "protocol_version": 1,
                "sidecar_name": "prism-roslyn-sidecar",
                "sidecar_version": "0.1.0",
                "capabilities": {
                    "metadata_hover": true,
                    "metadata_completion": true,
                    "generated_symbol_lookup": true,
                    "xml_documentation": true,
                    "workspace_reload": true
                }
            }
        });
        let transport = MockTransport::with_response(response);
        let mut client = RoslynSidecarClient::new(transport);

        let result = client
            .initialize(SidecarInitializeParams {
                protocol_version: 1,
                client_name: "prism-lsp".to_string(),
                client_version: Some("0.1.0".to_string()),
            })
            .expect("initialize should succeed");

        assert_eq!(result.protocol_version, 1);
        assert_eq!(result.capabilities, SidecarCapabilities::default());

        let transport = client.into_transport();
        let sent: Value = serde_json::from_slice(&transport.sent_payloads[0]).expect("request should parse as JSON");
        assert_eq!(sent["method"], json!(METHOD_SIDECAR_INITIALIZE));
        assert_eq!(sent["id"], json!(1));
        assert_eq!(sent["params"]["client_name"], json!("prism-lsp"));
    }

    #[test]
    fn remote_error_is_reported() {
        let response = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "error": {
                "code": -32001,
                "message": "Unity metadata references are not loaded"
            }
        });
        let transport = MockTransport::with_response(response);
        let mut client = RoslynSidecarClient::new(transport);

        let error = client
            .get_hover(UnityGetHoverParams {
                type_name: "Transform".to_string(),
                member_name: Some("position".to_string()),
                context: None,
            })
            .expect_err("hover should fail");

        match error {
            RoslynSidecarClientError::Remote { code, message } => {
                assert_eq!(code, -32001);
                assert!(message.contains("metadata references"));
            }
            other => panic!("unexpected error: {}", other),
        }
    }

    #[test]
    fn hover_response_deserializes_through_client() {
        let response = json!({
            "jsonrpc": "2.0",
            "id": 1,
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
        let transport = MockTransport::with_response(response);
        let mut client = RoslynSidecarClient::new(transport);

        let hover = client
            .get_hover(UnityGetHoverParams {
                type_name: "Transform".to_string(),
                member_name: Some("position".to_string()),
                context: None,
            })
            .expect("hover should succeed");

        assert_eq!(hover.kind, SidecarSymbolKind::Property);
        assert_eq!(hover.source, SidecarSymbolSource::Metadata);
        assert_eq!(hover.namespace.as_deref(), Some("UnityEngine"));
    }
}