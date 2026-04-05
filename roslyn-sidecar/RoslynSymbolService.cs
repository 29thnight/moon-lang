using System.Text;
using System.Xml.Linq;
using Microsoft.CodeAnalysis;
using Microsoft.CodeAnalysis.CSharp.Syntax;

namespace Prism.RoslynSidecar;

internal sealed class RoslynSymbolService
{
    private const string UnityDocsBaseUrl = "https://docs.unity3d.com/6000.3/Documentation/ScriptReference";

    private readonly RoslynProjectContext _context;

    private static readonly SymbolDisplayFormat SignatureFormat = new(
        globalNamespaceStyle: SymbolDisplayGlobalNamespaceStyle.Omitted,
        typeQualificationStyle: SymbolDisplayTypeQualificationStyle.NameAndContainingTypesAndNamespaces,
        genericsOptions: SymbolDisplayGenericsOptions.IncludeTypeParameters,
        memberOptions: SymbolDisplayMemberOptions.IncludeParameters
            | SymbolDisplayMemberOptions.IncludeContainingType
            | SymbolDisplayMemberOptions.IncludeType,
        parameterOptions: SymbolDisplayParameterOptions.IncludeType
            | SymbolDisplayParameterOptions.IncludeName
            | SymbolDisplayParameterOptions.IncludeOptionalBrackets,
        miscellaneousOptions: SymbolDisplayMiscellaneousOptions.UseSpecialTypes);

    public RoslynSymbolService(RoslynProjectContext context)
    {
        _context = context;
    }

    public UnityCompleteMembersResult CompleteMembers(UnityCompleteMembersParams @params)
    {
        var matches = ResolveTypes(@params.TypeName)
            .SelectMany(type => EnumerateCompletionItems(type, @params))
            .GroupBy(item => item.Label, StringComparer.OrdinalIgnoreCase)
            .Select(group => group.First())
            .OrderBy(item => item.Label, StringComparer.OrdinalIgnoreCase)
            .ToList();

        return new UnityCompleteMembersResult
        {
            Items = matches,
        };
    }

    public UnityHoverResult GetHover(UnityGetHoverParams @params)
    {
        var type = ResolveTypeOrThrow(@params.TypeName);
        if (!string.IsNullOrWhiteSpace(@params.MemberName))
        {
            var member = ResolveMemberOrThrow(type, @params.MemberName!);
            return CreateHoverResult(member);
        }

        return CreateHoverResult(type);
    }

    public UnityTypeResult GetType(UnityGetTypeParams @params)
    {
        var type = ResolveTypeOrThrow(@params.TypeName, @params.NamespaceHint);
        return new UnityTypeResult
        {
            DisplayName = type.ToDisplayString(),
            Kind = MapSymbolKind(type),
            Source = SymbolSource(type),
            Namespace = type.ContainingNamespace?.ToDisplayString(),
            Assembly = type.ContainingAssembly?.Name,
            BaseType = type.BaseType?.ToDisplayString(),
            Interfaces = type.Interfaces.Select(item => item.ToDisplayString()).ToList(),
            Documentation = DocumentationText(type),
            DocsUrl = TryBuildDocsUrl(type),
        };
    }

    public UnityDefinitionResult GetDefinition(UnityGetDefinitionParams @params)
    {
        var type = ResolveTypeOrThrow(@params.TypeName);
        var symbol = string.IsNullOrWhiteSpace(@params.MemberName)
            ? (ISymbol)type
            : ResolveMemberOrThrow(type, @params.MemberName!);

        return CreateDefinitionResult(symbol);
    }

    public UnityResolveGeneratedSymbolResult ResolveGeneratedSymbol(UnityResolveGeneratedSymbolParams @params)
    {
        var normalized = Path.GetFullPath(@params.GeneratedFile);
        if (!_context.TryGetSyntaxTree(normalized, out var syntaxTree))
        {
            return new UnityResolveGeneratedSymbolResult();
        }

        var semanticModel = _context.Compilation.GetSemanticModel(syntaxTree);
        var root = syntaxTree.GetRoot();
        ISymbol? symbol = null;

        if (!string.IsNullOrWhiteSpace(@params.MemberName))
        {
            var declaration = root
                .DescendantNodes()
                .OfType<MemberDeclarationSyntax>()
                .FirstOrDefault(node => node switch
                {
                    MethodDeclarationSyntax method => string.Equals(method.Identifier.ValueText, @params.MemberName, StringComparison.Ordinal),
                    PropertyDeclarationSyntax property => string.Equals(property.Identifier.ValueText, @params.MemberName, StringComparison.Ordinal),
                    FieldDeclarationSyntax field => field.Declaration.Variables.Any(variable => string.Equals(variable.Identifier.ValueText, @params.MemberName, StringComparison.Ordinal)),
                    EventDeclarationSyntax @event => string.Equals(@event.Identifier.ValueText, @params.MemberName, StringComparison.Ordinal),
                    EventFieldDeclarationSyntax eventField => eventField.Declaration.Variables.Any(variable => string.Equals(variable.Identifier.ValueText, @params.MemberName, StringComparison.Ordinal)),
                    _ => false,
                });

            if (declaration is not null)
            {
                symbol = semanticModel.GetDeclaredSymbol(declaration);
            }
        }

        symbol ??= ResolveTypeOrThrow(@params.TypeName);

        return new UnityResolveGeneratedSymbolResult
        {
            Symbol = new ResolvedGeneratedSymbol
            {
                DisplayName = symbol.ToDisplayString(),
                Kind = MapSymbolKind(symbol),
                Source = SymbolSource(symbol),
                Location = FirstSourceLocation(symbol),
                Assembly = symbol.ContainingAssembly?.Name,
                DocsUrl = TryBuildDocsUrl(symbol),
            },
        };
    }

    private IEnumerable<INamedTypeSymbol> ResolveTypes(string typeName, string? namespaceHint = null)
    {
        var normalized = NormalizeTypeName(typeName);
        var qualifiedCandidates = new List<string>();
        if (!string.IsNullOrWhiteSpace(namespaceHint) && !normalized.Contains('.'))
        {
            qualifiedCandidates.Add($"{namespaceHint}.{normalized}");
        }
        if (normalized.Contains('.'))
        {
            qualifiedCandidates.Add(normalized);
        }

        foreach (var candidate in qualifiedCandidates)
        {
            var symbol = _context.Compilation.GetTypeByMetadataName(candidate);
            if (symbol is not null)
            {
                yield return symbol;
            }
        }

        var seen = new HashSet<string>(StringComparer.Ordinal);
        foreach (var assembly in _context.Assemblies())
        {
            foreach (var symbol in EnumerateTypes(assembly.GlobalNamespace, normalized))
            {
                var key = symbol.ToDisplayString(SymbolDisplayFormat.FullyQualifiedFormat);
                if (seen.Add(key))
                {
                    yield return symbol;
                }
            }
        }
    }

    private static IEnumerable<INamedTypeSymbol> EnumerateTypes(INamespaceSymbol namespaceSymbol, string typeName)
    {
        foreach (var type in namespaceSymbol.GetTypeMembers())
        {
            if (string.Equals(type.Name, typeName, StringComparison.Ordinal))
            {
                yield return type;
            }
        }

        foreach (var childNamespace in namespaceSymbol.GetNamespaceMembers())
        {
            foreach (var type in EnumerateTypes(childNamespace, typeName))
            {
                yield return type;
            }
        }
    }

    private INamedTypeSymbol ResolveTypeOrThrow(string typeName, string? namespaceHint = null)
    {
        var symbol = ResolveTypes(typeName, namespaceHint)
            .OrderByDescending(type => type.ContainingNamespace?.ToDisplayString().StartsWith("Unity", StringComparison.Ordinal) == true)
            .ThenBy(type => type.ToDisplayString(), StringComparer.Ordinal)
            .FirstOrDefault();

        return symbol ?? throw new InvalidOperationException($"Type '{typeName}' was not found in the Roslyn compilation context.");
    }

    private static ISymbol ResolveMemberOrThrow(INamedTypeSymbol type, string memberName)
    {
        var symbol = type
            .GetMembers()
            .Where(member => !member.IsImplicitlyDeclared)
            .FirstOrDefault(member => string.Equals(member.Name, memberName, StringComparison.Ordinal))
            ?? type
                .GetMembers()
                .Where(member => !member.IsImplicitlyDeclared)
                .FirstOrDefault(member => string.Equals(member.Name, memberName, StringComparison.OrdinalIgnoreCase));

        return symbol ?? throw new InvalidOperationException($"Member '{memberName}' was not found on type '{type.ToDisplayString()}'.");
    }

    private IEnumerable<UnityCompletionItem> EnumerateCompletionItems(INamedTypeSymbol type, UnityCompleteMembersParams @params)
    {
        foreach (var member in type.GetMembers().Where(member => !member.IsImplicitlyDeclared))
        {
            if (!ShouldIncludeMember(member, @params))
            {
                continue;
            }

            if (!MatchesPrefix(member.Name, @params.Prefix))
            {
                continue;
            }

            yield return CreateCompletionItem(member);
        }
    }

    private static bool ShouldIncludeMember(ISymbol member, UnityCompleteMembersParams @params)
    {
        var isStatic = member.IsStatic;
        if (isStatic && !@params.IncludeStaticMembers)
        {
            return false;
        }
        if (!isStatic && !@params.IncludeInstanceMembers)
        {
            return false;
        }

        return member switch
        {
            IMethodSymbol method => method.MethodKind is MethodKind.Ordinary or MethodKind.Constructor,
            IPropertySymbol => true,
            IFieldSymbol field => !field.IsConst || !field.IsImplicitlyDeclared,
            IEventSymbol => true,
            _ => false,
        };
    }

    private static bool MatchesPrefix(string label, string prefix)
    {
        if (string.IsNullOrEmpty(prefix))
        {
            return true;
        }

        return label.StartsWith(prefix, StringComparison.OrdinalIgnoreCase)
            || ToLowerCamelCase(label).StartsWith(prefix, StringComparison.OrdinalIgnoreCase);
    }

    private UnityCompletionItem CreateCompletionItem(ISymbol symbol)
    {
        return new UnityCompletionItem
        {
            Label = symbol.Name,
            Kind = MapCompletionKind(symbol),
            Detail = symbol.Kind.ToString(),
            Documentation = DocumentationText(symbol),
            Signature = SignatureText(symbol),
            InsertText = null,
            Namespace = symbol.ContainingNamespace?.ToDisplayString(),
            Assembly = symbol.ContainingAssembly?.Name,
            IsStatic = symbol.IsStatic,
        };
    }

    private UnityHoverResult CreateHoverResult(ISymbol symbol)
    {
        return new UnityHoverResult
        {
            DisplayName = symbol.ToDisplayString(),
            Kind = MapSymbolKind(symbol),
            Source = SymbolSource(symbol),
            Namespace = symbol.ContainingNamespace?.ToDisplayString(),
            Signature = SignatureText(symbol),
            Documentation = DocumentationText(symbol),
            Assembly = symbol.ContainingAssembly?.Name,
            DocsUrl = TryBuildDocsUrl(symbol),
            IsStatic = symbol.IsStatic,
        };
    }

    private UnityDefinitionResult CreateDefinitionResult(ISymbol symbol)
    {
        return new UnityDefinitionResult
        {
            Source = SymbolSource(symbol),
            Location = FirstSourceLocation(symbol),
            DisplayName = symbol.ToDisplayString(),
            Assembly = symbol.ContainingAssembly?.Name,
            DocsUrl = TryBuildDocsUrl(symbol),
        };
    }

    private static SidecarLocation? FirstSourceLocation(ISymbol symbol)
    {
        var location = symbol.Locations.FirstOrDefault(candidate => candidate.IsInSource);
        if (location is null || location.SourceTree is null)
        {
            return null;
        }

        var span = location.GetLineSpan();
        return new SidecarLocation
        {
            FilePath = location.SourceTree.FilePath,
            Line = (uint)span.StartLinePosition.Line + 1,
            Col = (uint)span.StartLinePosition.Character + 1,
            EndLine = (uint)span.EndLinePosition.Line + 1,
            EndCol = (uint)span.EndLinePosition.Character + 1,
        };
    }

    private static SidecarCompletionItemKind MapCompletionKind(ISymbol symbol)
    {
        return symbol switch
        {
            IMethodSymbol method when method.MethodKind == MethodKind.Constructor => SidecarCompletionItemKind.Constructor,
            IMethodSymbol => SidecarCompletionItemKind.Method,
            IPropertySymbol => SidecarCompletionItemKind.Property,
            IFieldSymbol => SidecarCompletionItemKind.Field,
            IEventSymbol => SidecarCompletionItemKind.Event,
            INamedTypeSymbol type => type.TypeKind switch
            {
                TypeKind.Class => SidecarCompletionItemKind.Class,
                TypeKind.Struct => SidecarCompletionItemKind.Struct,
                TypeKind.Interface => SidecarCompletionItemKind.Interface,
                TypeKind.Enum => SidecarCompletionItemKind.Enum,
                _ => SidecarCompletionItemKind.Class,
            },
            _ => SidecarCompletionItemKind.Method,
        };
    }

    private static SidecarSymbolKind MapSymbolKind(ISymbol symbol)
    {
        return symbol switch
        {
            IMethodSymbol => SidecarSymbolKind.Method,
            IPropertySymbol => SidecarSymbolKind.Property,
            IFieldSymbol => SidecarSymbolKind.Field,
            IEventSymbol => SidecarSymbolKind.Event,
            INamedTypeSymbol type => type.TypeKind switch
            {
                TypeKind.Class => SidecarSymbolKind.Class,
                TypeKind.Struct => SidecarSymbolKind.Struct,
                TypeKind.Interface => SidecarSymbolKind.Interface,
                TypeKind.Enum => SidecarSymbolKind.Enum,
                TypeKind.Delegate => SidecarSymbolKind.Delegate,
                _ => SidecarSymbolKind.Class,
            },
            _ => SidecarSymbolKind.Method,
        };
    }

    private static SidecarSymbolSource SymbolSource(ISymbol symbol)
    {
        if (symbol.Locations.Any(location => location.IsInSource))
        {
            return SidecarSymbolSource.Generated;
        }

        return symbol.Kind == SymbolKind.NamedType || symbol.Kind == SymbolKind.Method || symbol.Kind == SymbolKind.Property || symbol.Kind == SymbolKind.Field || symbol.Kind == SymbolKind.Event
            ? SidecarSymbolSource.Metadata
            : SidecarSymbolSource.Source;
    }

    private static string? SignatureText(ISymbol symbol)
    {
        return symbol.ToDisplayString(SignatureFormat);
    }

    private static string? DocumentationText(ISymbol symbol)
    {
        var xml = symbol.GetDocumentationCommentXml(expandIncludes: true, cancellationToken: default);
        return SummaryFromXml(xml);
    }

    private static string? SummaryFromXml(string? xml)
    {
        if (string.IsNullOrWhiteSpace(xml))
        {
            return null;
        }

        try
        {
            var document = XDocument.Parse($"<root>{xml}</root>");
            var summary = document.Root?.Element("summary");
            if (summary is null)
            {
                return null;
            }

            var builder = new StringBuilder();
            foreach (var text in summary.DescendantNodes().OfType<XText>())
            {
                builder.Append(text.Value);
            }

            var normalized = builder.ToString().Trim();
            return string.IsNullOrWhiteSpace(normalized) ? null : string.Join(" ", normalized.Split(default(string[]?), StringSplitOptions.RemoveEmptyEntries));
        }
        catch
        {
            return null;
        }
    }

    private static string? TryBuildDocsUrl(ISymbol symbol)
    {
        var containingType = symbol switch
        {
            INamedTypeSymbol namedType => namedType,
            _ => symbol.ContainingType,
        };

        if (containingType is null)
        {
            return null;
        }

        var namespaceName = containingType.ContainingNamespace?.ToDisplayString();
        if (string.IsNullOrWhiteSpace(namespaceName) || !namespaceName.StartsWith("Unity", StringComparison.Ordinal))
        {
            return null;
        }

        return symbol switch
        {
            INamedTypeSymbol => $"{UnityDocsBaseUrl}/{containingType.Name}.html",
            IMethodSymbol => $"{UnityDocsBaseUrl}/{containingType.Name}.{symbol.Name}.html",
            IPropertySymbol or IFieldSymbol or IEventSymbol => $"{UnityDocsBaseUrl}/{containingType.Name}-{symbol.Name}.html",
            _ => null,
        };
    }

    private static string NormalizeTypeName(string typeName)
    {
        var trimmed = typeName.Trim().TrimEnd('?');
        var genericIndex = trimmed.IndexOf('<');
        return genericIndex >= 0 ? trimmed[..genericIndex] : trimmed;
    }

    private static string ToLowerCamelCase(string value)
    {
        if (string.IsNullOrEmpty(value))
        {
            return value;
        }

        return char.ToLowerInvariant(value[0]) + value[1..];
    }
}