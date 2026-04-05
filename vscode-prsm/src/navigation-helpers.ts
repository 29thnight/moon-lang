import { PrismDefinitionEntry, PrismIndexResult, PrismIndexedReference, PrismIndexedSymbol, PrismSourceLocation } from './prism-cli';
import { PrismCSharpHoverInfo } from './csharp-navigation';

export function getNavigationFallbackTarget(result: PrismIndexResult): PrismIndexedSymbol | null {
    return result.reference_at?.resolved_symbol ?? result.symbol_at ?? null;
}

export function getNavigationHoverText(
    result: PrismIndexResult,
    definition?: PrismDefinitionEntry | null,
    csharpInfo?: PrismCSharpHoverInfo | null,
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

export function getNavigationTypeHoverText(
    result: PrismIndexResult,
    csharpInfo?: PrismCSharpHoverInfo | null,
): string | undefined {
    const reference = result.reference_at;
    if (!reference || reference.kind.toLowerCase() !== 'type' || !csharpInfo) {
        return undefined;
    }

    return [
        `**type reference** ${reference.name}`,
        formatCSharpHover(csharpInfo),
    ].filter(Boolean).join('\n\n');
}

function formatSymbolHover(
    symbol: PrismIndexedSymbol,
    definition?: PrismDefinitionEntry | null,
    csharpInfo?: PrismCSharpHoverInfo | null,
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
        '```prsm',
        symbol.signature,
        '```',
        formatCSharpHover(csharpInfo),
    ].filter(Boolean).join('\n\n');
}

function formatResolvedReferenceHover(
    reference: PrismIndexedReference,
    symbol: PrismIndexedSymbol,
    definition?: PrismDefinitionEntry | null,
    csharpInfo?: PrismCSharpHoverInfo | null,
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
        '```prsm',
        symbol.signature,
        '```',
        formatCSharpHover(csharpInfo),
    ].filter(Boolean).join('\n\n');
}

function formatUnresolvedReferenceHover(reference: PrismIndexedReference, csharpInfo?: PrismCSharpHoverInfo | null): string {
    return [
        `**${reference.kind} reference** ${reference.name}`,
        '**Status:** Unresolved  \n**Definition:** Not found in the current PrSM project index.',
        formatCSharpHover(csharpInfo),
    ].filter(Boolean).join('\n\n');
}

function formatCSharpHover(csharpInfo?: PrismCSharpHoverInfo | null): string | undefined {
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

function formatCSharpLookup(csharpInfo: PrismCSharpHoverInfo): string {
    return csharpInfo.memberName
        ? `${csharpInfo.typeName}.${csharpInfo.memberName}`
        : csharpInfo.typeName;
}

function formatLocation(location: PrismSourceLocation): string {
    return `${location.file.replace(/\\/g, '/')}:${location.line}:${location.col}`;
}

function showsMutability(definition: PrismDefinitionEntry): boolean {
    return definition.kind === 'field' || definition.kind === 'local';
}