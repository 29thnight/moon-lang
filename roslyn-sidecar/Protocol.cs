using System.Text.Json;
using System.Text.Json.Serialization;

namespace Prism.RoslynSidecar;

internal static class SidecarMethods
{
    public const string HealthPing = "health/ping";
    public const string SidecarInitialize = "sidecar/initialize";
    public const string SidecarLoadProject = "sidecar/loadProject";
    public const string SidecarShutdown = "sidecar/shutdown";
    public const string UnityCompleteMembers = "unity/completeMembers";
    public const string UnityGetDefinition = "unity/getDefinition";
    public const string UnityGetHover = "unity/getHover";
    public const string UnityGetType = "unity/getType";
    public const string UnityResolveGeneratedSymbol = "unity/resolveGeneratedSymbol";
    public const string WorkspaceReload = "workspace/reload";
}

internal static class SidecarProtocol
{
    public const string JsonRpcVersion = "2.0";
    public const int ProtocolVersion = 1;
}

internal static class SidecarErrorCodes
{
    public const int InvalidParams = -32602;
    public const int MethodNotFound = -32601;
    public const int ProjectNotLoaded = -32002;
    public const int NotImplemented = -32004;
}

internal sealed class RpcIdConverter : JsonConverter<RpcId>
{
    public override RpcId Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        if (reader.TokenType == JsonTokenType.Number && reader.TryGetUInt64(out var number))
        {
            return RpcId.FromNumber(number);
        }

        if (reader.TokenType == JsonTokenType.String)
        {
            return RpcId.FromString(reader.GetString() ?? string.Empty);
        }

        throw new JsonException("JSON-RPC id must be a number or string.");
    }

    public override void Write(Utf8JsonWriter writer, RpcId value, JsonSerializerOptions options)
    {
        if (value.Number is ulong number)
        {
            writer.WriteNumberValue(number);
            return;
        }

        if (value.Text is string text)
        {
            writer.WriteStringValue(text);
            return;
        }

        writer.WriteNullValue();
    }
}

[JsonConverter(typeof(RpcIdConverter))]
internal sealed record RpcId
{
    public ulong? Number { get; init; }
    public string? Text { get; init; }

    public static RpcId FromNumber(ulong number) => new() { Number = number };

    public static RpcId FromString(string text) => new() { Text = text };
}

internal sealed class JsonRpcRequest
{
    public string Jsonrpc { get; init; } = SidecarProtocol.JsonRpcVersion;
    public RpcId Id { get; init; } = RpcId.FromNumber(0);
    public string Method { get; init; } = string.Empty;
    public JsonElement? Params { get; init; }

    public T? DeserializeParams<T>(JsonSerializerOptions options)
    {
        if (Params is not JsonElement value || value.ValueKind is JsonValueKind.Null or JsonValueKind.Undefined)
        {
            return default;
        }

        return value.Deserialize<T>(options);
    }
}

internal sealed class JsonRpcSuccessResponse<T>
{
    public string Jsonrpc { get; init; } = SidecarProtocol.JsonRpcVersion;
    public RpcId Id { get; init; } = RpcId.FromNumber(0);
    public T Result { get; init; } = default!;
}

internal sealed class JsonRpcErrorResponse
{
    public string Jsonrpc { get; init; } = SidecarProtocol.JsonRpcVersion;
    public RpcId Id { get; init; } = RpcId.FromNumber(0);
    public JsonRpcError Error { get; init; } = new();
}

internal sealed class JsonRpcError
{
    public int Code { get; init; }
    public string Message { get; init; } = string.Empty;
}

internal sealed class SidecarCapabilities
{
    public bool MetadataHover { get; init; } = true;
    public bool MetadataCompletion { get; init; } = true;
    public bool GeneratedSymbolLookup { get; init; } = true;
    public bool XmlDocumentation { get; init; } = true;
    public bool WorkspaceReload { get; init; } = true;
}

internal sealed class HealthPingParams
{
    public string? Nonce { get; init; }
}

internal sealed class HealthPingResult
{
    public string? Nonce { get; init; }
    public int ProtocolVersion { get; init; }
    public string SidecarName { get; init; } = string.Empty;
    public string? SidecarVersion { get; init; }
    public SidecarCapabilities Capabilities { get; init; } = new();
}

internal sealed class SidecarInitializeParams
{
    public int ProtocolVersion { get; init; }
    public string ClientName { get; init; } = string.Empty;
    public string? ClientVersion { get; init; }
}

internal sealed class SidecarInitializeResult
{
    public int ProtocolVersion { get; init; }
    public string SidecarName { get; init; } = string.Empty;
    public string? SidecarVersion { get; init; }
    public SidecarCapabilities Capabilities { get; init; } = new();
}

internal sealed class SidecarLoadProjectParams
{
    public string WorkspaceRoot { get; init; } = string.Empty;
    public string? ProjectFile { get; init; }
    public string UnityProjectRoot { get; init; } = string.Empty;
    public string? OutputDir { get; init; }
    public List<string> GeneratedFiles { get; init; } = [];
    public List<string> MetadataReferences { get; init; } = [];
    public List<string> PackageAssemblies { get; init; } = [];
}

internal sealed class SidecarLoadProjectResult
{
    public string ProjectId { get; init; } = string.Empty;
    public int LoadedDocuments { get; init; }
    public int MetadataReferenceCount { get; init; }
    public int GeneratedDocumentCount { get; init; }
}

internal sealed class SidecarShutdownResult
{
    public bool Acknowledged { get; init; }
}

internal enum WorkspaceReloadReason
{
    ProjectConfigChanged,
    GeneratedSourcesChanged,
    MetadataReferencesChanged,
    PackageManifestChanged,
    Manual,
}

internal sealed class WorkspaceReloadParams
{
    public WorkspaceReloadReason Reason { get; init; }
    public List<string> ChangedFiles { get; init; } = [];
}

internal sealed class WorkspaceReloadResult
{
    public string ProjectId { get; init; } = string.Empty;
    public bool Reloaded { get; init; }
    public int ChangedDocumentCount { get; init; }
}

internal sealed class GeneratedContext
{
    public string? GeneratedOwnerType { get; init; }
    public string? GeneratedFile { get; init; }
}

internal enum SidecarCompletionItemKind
{
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

internal enum SidecarSymbolKind
{
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

internal enum SidecarSymbolSource
{
    Metadata,
    Generated,
    Source,
}

internal sealed class SidecarLocation
{
    public string FilePath { get; init; } = string.Empty;
    public uint Line { get; init; }
    public uint Col { get; init; }
    public uint EndLine { get; init; }
    public uint EndCol { get; init; }
}

internal sealed class UnityCompleteMembersParams
{
    public string TypeName { get; init; } = string.Empty;
    public string Prefix { get; init; } = string.Empty;
    public GeneratedContext? Context { get; init; }
    public bool IncludeInstanceMembers { get; init; } = true;
    public bool IncludeStaticMembers { get; init; }
}

internal sealed class UnityCompletionItem
{
    public string Label { get; init; } = string.Empty;
    public SidecarCompletionItemKind Kind { get; init; }
    public string? Detail { get; init; }
    public string? Documentation { get; init; }
    public string? Signature { get; init; }
    public string? InsertText { get; init; }
    public string? Namespace { get; init; }
    public string? Assembly { get; init; }
    public bool IsStatic { get; init; }
}

internal sealed class UnityCompleteMembersResult
{
    public List<UnityCompletionItem> Items { get; init; } = [];
}

internal sealed class UnityGetHoverParams
{
    public string TypeName { get; init; } = string.Empty;
    public string? MemberName { get; init; }
    public GeneratedContext? Context { get; init; }
}

internal sealed class UnityHoverResult
{
    public string DisplayName { get; init; } = string.Empty;
    public SidecarSymbolKind Kind { get; init; }
    public SidecarSymbolSource Source { get; init; }
    public string? Namespace { get; init; }
    public string? Signature { get; init; }
    public string? Documentation { get; init; }
    public string? Assembly { get; init; }
    public string? DocsUrl { get; init; }
    public bool IsStatic { get; init; }
}

internal sealed class UnityGetTypeParams
{
    public string TypeName { get; init; } = string.Empty;
    public string? NamespaceHint { get; init; }
}

internal sealed class UnityTypeResult
{
    public string DisplayName { get; init; } = string.Empty;
    public SidecarSymbolKind Kind { get; init; }
    public SidecarSymbolSource Source { get; init; }
    public string? Namespace { get; init; }
    public string? Assembly { get; init; }
    public string? BaseType { get; init; }
    public List<string> Interfaces { get; init; } = [];
    public string? Documentation { get; init; }
    public string? DocsUrl { get; init; }
}

internal sealed class UnityGetDefinitionParams
{
    public string TypeName { get; init; } = string.Empty;
    public string? MemberName { get; init; }
    public GeneratedContext? Context { get; init; }
}

internal sealed class UnityDefinitionResult
{
    public SidecarSymbolSource Source { get; init; }
    public SidecarLocation? Location { get; init; }
    public string? DisplayName { get; init; }
    public string? Assembly { get; init; }
    public string? DocsUrl { get; init; }
}

internal sealed class UnityResolveGeneratedSymbolParams
{
    public string GeneratedFile { get; init; } = string.Empty;
    public string TypeName { get; init; } = string.Empty;
    public string? MemberName { get; init; }
}

internal sealed class ResolvedGeneratedSymbol
{
    public string DisplayName { get; init; } = string.Empty;
    public SidecarSymbolKind Kind { get; init; }
    public SidecarSymbolSource Source { get; init; }
    public SidecarLocation? Location { get; init; }
    public string? Assembly { get; init; }
    public string? DocsUrl { get; init; }
}

internal sealed class UnityResolveGeneratedSymbolResult
{
    public ResolvedGeneratedSymbol? Symbol { get; init; }
}

internal sealed class ProjectState
{
    public string ProjectId { get; init; } = string.Empty;
    public string WorkspaceRoot { get; init; } = string.Empty;
    public string? ProjectFile { get; init; }
    public string UnityProjectRoot { get; init; } = string.Empty;
    public string? OutputDir { get; init; }
    public List<string> GeneratedFiles { get; init; } = [];
    public List<string> MetadataReferences { get; init; } = [];
    public List<string> PackageAssemblies { get; init; } = [];
}