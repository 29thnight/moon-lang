import { PrismDefinitionEntry, PrismIndexResult, PrismIndexedReference, PrismIndexedSymbol } from './prism-cli';
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

export function getNavigationCSharpHoverSection(
    csharpInfo?: PrismCSharpHoverInfo | null,
): string | undefined {
    return formatCSharpHover(csharpInfo);
}

function formatSymbolHover(
    symbol: PrismIndexedSymbol,
    _definition?: PrismDefinitionEntry | null,
    csharpInfo?: PrismCSharpHoverInfo | null,
): string {
    return [
        formatPrsmSignature(symbol.signature),
        formatCSharpHover(csharpInfo),
    ].filter(Boolean).join('\n\n');
}

function formatResolvedReferenceHover(
    reference: PrismIndexedReference,
    symbol: PrismIndexedSymbol,
    _definition?: PrismDefinitionEntry | null,
    csharpInfo?: PrismCSharpHoverInfo | null,
): string {
    return [
        formatPrsmSignature(symbol.signature),
        formatCSharpHover(csharpInfo),
    ].filter(Boolean).join('\n\n');
}

function formatUnresolvedReferenceHover(reference: PrismIndexedReference, csharpInfo?: PrismCSharpHoverInfo | null): string {
    const unresolvedMessage = csharpInfo ? undefined : '_Definition not found in the current PrSM project index._';

    return [
        formatPrsmSignature(reference.name),
        formatCSharpHover(csharpInfo),
        unresolvedMessage,
    ].filter(Boolean).join('\n\n');
}

function formatCSharpHover(csharpInfo?: PrismCSharpHoverInfo | null): string | undefined {
    if (!csharpInfo?.hoverText?.trim()) {
        return undefined;
    }

    const { codeBlocks, prose } = splitMarkdownCodeBlocks(csharpInfo.hoverText.trim());

    return [
        '**[Generated C#]**',
        codeBlocks.join('\n\n'),
        prose,
    ].join('\n\n');
}

function formatPrsmSignature(signature: string): string {
    return ['```prsm', signature, '```'].join('\n');
}

function splitMarkdownCodeBlocks(markdown: string): { codeBlocks: string[]; prose?: string } {
    const codeBlocks = markdown.match(/```[\s\S]*?```/g) ?? [];
    const prose = markdown
        .replace(/```[\s\S]*?```/g, '')
        .trim()
        .replace(/\n{3,}/g, '\n\n');

    return {
        codeBlocks,
        prose: prose || undefined,
    };
}
