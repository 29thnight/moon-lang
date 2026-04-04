import { MoonDefinitionEntry, MoonIndexResult, MoonIndexedReference, MoonIndexedSymbol, MoonSourceLocation } from './moonc-cli';
import { MoonCSharpHoverInfo } from './csharp-navigation';

export function getNavigationFallbackTarget(result: MoonIndexResult): MoonIndexedSymbol | null {
    return result.reference_at?.resolved_symbol ?? result.symbol_at ?? null;
}

export function getNavigationHoverText(
    result: MoonIndexResult,
    definition?: MoonDefinitionEntry | null,
    csharpInfo?: MoonCSharpHoverInfo | null,
): string | undefined {
    if (result.symbol_at) {
        return formatSymbolHover(result.symbol_at, definition, csharpInfo);
    }

    if (result.reference_at?.resolved_symbol) {
        return formatResolvedReferenceHover(result.reference_at, result.reference_at.resolved_symbol, definition, csharpInfo);
    }

    if (result.reference_at) {
        return formatUnresolvedReferenceHover(result.reference_at, csharpInfo);
    }

    return undefined;
}

function formatSymbolHover(
    symbol: MoonIndexedSymbol,
    definition?: MoonDefinitionEntry | null,
    csharpInfo?: MoonCSharpHoverInfo | null,
): string {
    const details = [
        '**Status:** Defined',
        `**Definition:** ${formatLocation(definition ?? symbol)}`,
    ];

    if (definition) {
        details.push(`**Type:** ${definition.type}`);
        if (showsMutability(definition)) {
            details.push(`**Mutable:** ${definition.mutable ? 'yes' : 'no'}`);
        }
    }

    return [
        `**${symbol.kind}** ${symbol.qualified_name}`,
        details.join('  \n'),
        '```moon',
        symbol.signature,
        '```',
        formatCSharpHover(csharpInfo),
    ].filter(Boolean).join('\n\n');
}

function formatResolvedReferenceHover(
    reference: MoonIndexedReference,
    symbol: MoonIndexedSymbol,
    definition?: MoonDefinitionEntry | null,
    csharpInfo?: MoonCSharpHoverInfo | null,
): string {
    const details = [
        '**Status:** Resolved',
        `**Target:** ${symbol.kind} ${symbol.qualified_name}`,
        `**Definition:** ${formatLocation(definition ?? symbol)}`,
    ];

    if (definition) {
        details.push(`**Type:** ${definition.type}`);
    }

    return [
        `**${reference.kind} reference** ${reference.name}`,
        details.join('  \n'),
        '```moon',
        symbol.signature,
        '```',
        formatCSharpHover(csharpInfo),
    ].filter(Boolean).join('\n\n');
}

function formatUnresolvedReferenceHover(reference: MoonIndexedReference, csharpInfo?: MoonCSharpHoverInfo | null): string {
    return [
        `**${reference.kind} reference** ${reference.name}`,
        '**Status:** Unresolved  \n**Definition:** Not found in the current Moon project index.',
        formatCSharpHover(csharpInfo),
    ].filter(Boolean).join('\n\n');
}

function formatCSharpHover(csharpInfo?: MoonCSharpHoverInfo | null): string | undefined {
    if (!csharpInfo) {
        return undefined;
    }

    const details = [
        `**Lookup:** ${formatCSharpLookup(csharpInfo)}`,
    ];

    if (csharpInfo.generatedFile) {
        details.push(`**File:** ${csharpInfo.generatedFile.replace(/\\/g, '/')}`);
    }

    return [
        '**Generated C#**',
        details.join('  \n'),
        csharpInfo.hoverText?.trim() || '_No additional C# hover details available._',
    ].join('\n\n');
}

function formatCSharpLookup(csharpInfo: MoonCSharpHoverInfo): string {
    return csharpInfo.memberName
        ? `${csharpInfo.typeName}.${csharpInfo.memberName}`
        : csharpInfo.typeName;
}

function formatLocation(location: MoonSourceLocation): string {
    return `${location.file.replace(/\\/g, '/')}:${location.line}:${location.col}`;
}

function showsMutability(definition: MoonDefinitionEntry): boolean {
    return definition.kind === 'field' || definition.kind === 'local';
}