using Microsoft.CodeAnalysis;
using Microsoft.CodeAnalysis.CSharp;
using Microsoft.CodeAnalysis.Text;

namespace Prism.RoslynSidecar;

internal sealed class RoslynProjectContext
{
    public RoslynProjectContext(ProjectState state, CSharpCompilation compilation, IReadOnlyDictionary<string, SyntaxTree> syntaxTreesByPath)
    {
        State = state;
        Compilation = compilation;
        SyntaxTreesByPath = syntaxTreesByPath;
    }

    public ProjectState State { get; }

    public CSharpCompilation Compilation { get; }

    public IReadOnlyDictionary<string, SyntaxTree> SyntaxTreesByPath { get; }

    public static RoslynProjectContext Load(ProjectState state)
    {
        var metadataReferences = BuildMetadataReferences(state);
        var syntaxTreesByPath = BuildSyntaxTrees(state.GeneratedFiles);

        var compilation = CSharpCompilation.Create(
            assemblyName: SanitizeAssemblyName(state.ProjectId),
            syntaxTrees: syntaxTreesByPath.Values,
            references: metadataReferences,
            options: new CSharpCompilationOptions(OutputKind.DynamicallyLinkedLibrary));

        return new RoslynProjectContext(state, compilation, syntaxTreesByPath);
    }

    public RoslynProjectContext Reload()
    {
        return Load(State);
    }

    public IEnumerable<IAssemblySymbol> Assemblies()
    {
        yield return Compilation.Assembly;

        foreach (var assembly in Compilation.SourceModule.ReferencedAssemblySymbols)
        {
            if (assembly is not null)
            {
                yield return assembly;
            }
        }
    }

    public bool TryGetSyntaxTree(string path, out SyntaxTree syntaxTree)
    {
        return SyntaxTreesByPath.TryGetValue(NormalizePath(path), out syntaxTree!);
    }

    private static IEnumerable<MetadataReference> BuildMetadataReferences(ProjectState state)
    {
        var seen = new HashSet<string>(StringComparer.OrdinalIgnoreCase);

        foreach (var path in EnumerateReferencePaths(state))
        {
            var normalized = NormalizePath(path);
            if (!File.Exists(normalized) || !seen.Add(normalized))
            {
                continue;
            }

            yield return MetadataReference.CreateFromFile(normalized);
        }
    }

    private static IEnumerable<string> EnumerateReferencePaths(ProjectState state)
    {
        foreach (var path in state.MetadataReferences)
        {
            yield return path;
        }

        foreach (var path in state.PackageAssemblies)
        {
            yield return path;
        }
    }

    private static Dictionary<string, SyntaxTree> BuildSyntaxTrees(IEnumerable<string> generatedFiles)
    {
        var syntaxTrees = new Dictionary<string, SyntaxTree>(StringComparer.OrdinalIgnoreCase);

        foreach (var path in generatedFiles)
        {
            var normalized = NormalizePath(path);
            if (!File.Exists(normalized) || syntaxTrees.ContainsKey(normalized))
            {
                continue;
            }

            var source = File.ReadAllText(normalized);
            var sourceText = SourceText.From(source);
            var syntaxTree = CSharpSyntaxTree.ParseText(sourceText, path: normalized);
            syntaxTrees[normalized] = syntaxTree;
        }

        return syntaxTrees;
    }
    private static string NormalizePath(string path)
    {
        return Path.GetFullPath(path)
            .TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar);
    }

    private static string SanitizeAssemblyName(string projectId)
    {
        var invalid = Path.GetInvalidFileNameChars();
        var chars = projectId.Select(ch => invalid.Contains(ch) ? '_' : ch).ToArray();
        return new string(chars);
    }
}