using System.Reflection;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace Prism.RoslynSidecar;

internal sealed class Program
{
    private static readonly JsonSerializerOptions JsonOptions = CreateJsonOptions();

    private ProjectState? _projectState;
    private RoslynProjectContext? _roslynContext;

    private static async Task<int> Main()
    {
        var program = new Program();
        await program.RunAsync(CancellationToken.None);
        return 0;
    }

    private async Task RunAsync(CancellationToken cancellationToken)
    {
        using var input = Console.OpenStandardInput();
        using var output = Console.OpenStandardOutput();
        var connection = new JsonRpcConnection(input, output, JsonOptions);

        while (!cancellationToken.IsCancellationRequested)
        {
            var request = await connection.ReadRequestAsync(cancellationToken);
            if (request is null)
            {
                break;
            }

            if (!string.Equals(request.Jsonrpc, SidecarProtocol.JsonRpcVersion, StringComparison.Ordinal))
            {
                await connection.WriteErrorAsync(
                    request.Id,
                    SidecarErrorCodes.InvalidParams,
                    $"Unsupported jsonrpc version '{request.Jsonrpc}'.",
                    cancellationToken);
                continue;
            }

            var shouldContinue = await HandleRequestAsync(connection, request, cancellationToken);
            if (!shouldContinue)
            {
                break;
            }
        }
    }

    private async Task<bool> HandleRequestAsync(
        JsonRpcConnection connection,
        JsonRpcRequest request,
        CancellationToken cancellationToken)
    {
        try
        {
            switch (request.Method)
            {
                case SidecarMethods.HealthPing:
                    await connection.WriteSuccessAsync(
                        request.Id,
                        HandlePing(request.DeserializeParams<HealthPingParams>(JsonOptions)),
                        cancellationToken);
                    return true;

                case SidecarMethods.SidecarInitialize:
                    await connection.WriteSuccessAsync(
                        request.Id,
                        HandleInitialize(request.DeserializeParams<SidecarInitializeParams>(JsonOptions)),
                        cancellationToken);
                    return true;

                case SidecarMethods.SidecarLoadProject:
                    await connection.WriteSuccessAsync(
                        request.Id,
                        HandleLoadProject(request.DeserializeParams<SidecarLoadProjectParams>(JsonOptions)),
                        cancellationToken);
                    return true;

                case SidecarMethods.WorkspaceReload:
                    await connection.WriteSuccessAsync(
                        request.Id,
                        HandleWorkspaceReload(request.DeserializeParams<WorkspaceReloadParams>(JsonOptions)),
                        cancellationToken);
                    return true;

                case SidecarMethods.SidecarShutdown:
                    await connection.WriteSuccessAsync(
                        request.Id,
                        new SidecarShutdownResult { Acknowledged = true },
                        cancellationToken);
                    return false;

                case SidecarMethods.UnityCompleteMembers:
                    await connection.WriteSuccessAsync(
                        request.Id,
                        RequireSymbolService().CompleteMembers(
                            request.DeserializeParams<UnityCompleteMembersParams>(JsonOptions)
                            ?? throw new InvalidOperationException("Missing unity/completeMembers parameters.")),
                        cancellationToken);
                    return true;

                case SidecarMethods.UnityGetHover:
                    await connection.WriteSuccessAsync(
                        request.Id,
                        RequireSymbolService().GetHover(
                            request.DeserializeParams<UnityGetHoverParams>(JsonOptions)
                            ?? throw new InvalidOperationException("Missing unity/getHover parameters.")),
                        cancellationToken);
                    return true;

                case SidecarMethods.UnityGetType:
                    await connection.WriteSuccessAsync(
                        request.Id,
                        RequireSymbolService().GetType(
                            request.DeserializeParams<UnityGetTypeParams>(JsonOptions)
                            ?? throw new InvalidOperationException("Missing unity/getType parameters.")),
                        cancellationToken);
                    return true;

                case SidecarMethods.UnityGetDefinition:
                    await connection.WriteSuccessAsync(
                        request.Id,
                        RequireSymbolService().GetDefinition(
                            request.DeserializeParams<UnityGetDefinitionParams>(JsonOptions)
                            ?? throw new InvalidOperationException("Missing unity/getDefinition parameters.")),
                        cancellationToken);
                    return true;

                case SidecarMethods.UnityResolveGeneratedSymbol:
                    await connection.WriteSuccessAsync(
                        request.Id,
                        RequireSymbolService().ResolveGeneratedSymbol(
                            request.DeserializeParams<UnityResolveGeneratedSymbolParams>(JsonOptions)
                            ?? throw new InvalidOperationException("Missing unity/resolveGeneratedSymbol parameters.")),
                        cancellationToken);
                    return true;

                default:
                    await connection.WriteErrorAsync(
                        request.Id,
                        SidecarErrorCodes.MethodNotFound,
                        $"Unsupported sidecar method: {request.Method}",
                        cancellationToken);
                    return true;
            }
        }
        catch (JsonException ex)
        {
            await connection.WriteErrorAsync(request.Id, SidecarErrorCodes.InvalidParams, ex.Message, cancellationToken);
            return true;
        }
        catch (InvalidOperationException ex)
        {
            await connection.WriteErrorAsync(request.Id, SidecarErrorCodes.InvalidParams, ex.Message, cancellationToken);
            return true;
        }
    }

    private static HealthPingResult HandlePing(HealthPingParams? @params)
    {
        return new HealthPingResult
        {
            Nonce = @params?.Nonce,
            ProtocolVersion = SidecarProtocol.ProtocolVersion,
            SidecarName = "prism-roslyn-sidecar",
            SidecarVersion = Assembly.GetExecutingAssembly().GetName().Version?.ToString(),
            Capabilities = new SidecarCapabilities(),
        };
    }

    private static SidecarInitializeResult HandleInitialize(SidecarInitializeParams? @params)
    {
        if (@params is null)
        {
            throw new InvalidOperationException("Missing initialize parameters.");
        }

        if (@params.ProtocolVersion != SidecarProtocol.ProtocolVersion)
        {
            throw new InvalidOperationException(
                $"Unsupported protocol version '{@params.ProtocolVersion}'. Expected {SidecarProtocol.ProtocolVersion}.");
        }

        return new SidecarInitializeResult
        {
            ProtocolVersion = SidecarProtocol.ProtocolVersion,
            SidecarName = "prism-roslyn-sidecar",
            SidecarVersion = Assembly.GetExecutingAssembly().GetName().Version?.ToString(),
            Capabilities = new SidecarCapabilities(),
        };
    }

    private SidecarLoadProjectResult HandleLoadProject(SidecarLoadProjectParams? @params)
    {
        if (@params is null)
        {
            throw new InvalidOperationException("Missing loadProject parameters.");
        }

        if (string.IsNullOrWhiteSpace(@params.WorkspaceRoot))
        {
            throw new InvalidOperationException("workspace_root is required.");
        }

        _projectState = new ProjectState
        {
            ProjectId = NormalizeProjectId(@params.WorkspaceRoot),
            WorkspaceRoot = @params.WorkspaceRoot,
            ProjectFile = @params.ProjectFile,
            UnityProjectRoot = string.IsNullOrWhiteSpace(@params.UnityProjectRoot) ? @params.WorkspaceRoot : @params.UnityProjectRoot,
            OutputDir = @params.OutputDir,
            GeneratedFiles = [.. @params.GeneratedFiles],
            MetadataReferences = [.. @params.MetadataReferences],
            PackageAssemblies = [.. @params.PackageAssemblies],
        };
        _roslynContext = RoslynProjectContext.Load(_projectState);

        return new SidecarLoadProjectResult
        {
            ProjectId = _projectState.ProjectId,
            LoadedDocuments = _projectState.GeneratedFiles.Count,
            MetadataReferenceCount = _projectState.MetadataReferences.Count + _projectState.PackageAssemblies.Count,
            GeneratedDocumentCount = _projectState.GeneratedFiles.Count,
        };
    }

    private WorkspaceReloadResult HandleWorkspaceReload(WorkspaceReloadParams? @params)
    {
        if (_projectState is null)
        {
            throw new InvalidOperationException("Project is not loaded.");
        }

        _roslynContext = _roslynContext?.Reload() ?? RoslynProjectContext.Load(_projectState);

        var changedFiles = @params?.ChangedFiles?.Count ?? 0;
        return new WorkspaceReloadResult
        {
            ProjectId = _projectState.ProjectId,
            Reloaded = true,
            ChangedDocumentCount = changedFiles,
        };
    }

    private bool EnsureProjectIsLoaded() => _projectState is not null;

    private RoslynSymbolService RequireSymbolService()
    {
        if (_roslynContext is null)
        {
            throw new InvalidOperationException("Project is not loaded.");
        }

        return new RoslynSymbolService(_roslynContext);
    }

    private static string NormalizeProjectId(string workspaceRoot)
    {
        return Path.GetFullPath(workspaceRoot)
            .TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar)
            .Replace('\\', '/');
    }

    private static JsonSerializerOptions CreateJsonOptions()
    {
        var options = new JsonSerializerOptions
        {
            PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower,
            DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
            WriteIndented = false,
        };
        options.Converters.Add(new JsonStringEnumConverter(JsonNamingPolicy.CamelCase));
        options.Converters.Add(new RpcIdConverter());
        return options;
    }
}