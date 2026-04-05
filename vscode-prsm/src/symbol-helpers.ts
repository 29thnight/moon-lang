import { PrismIndexedSymbol } from './prism-cli';

export interface PrSMSymbolNode {
    symbol: PrismIndexedSymbol;
    children: PrSMSymbolNode[];
}

export function buildPrSMDocumentSymbolTree(symbols: PrismIndexedSymbol[]): PrSMSymbolNode[] {
    const normalized = [...symbols].sort(compareSymbols);
    const childrenByContainer = new Map<string, PrismIndexedSymbol[]>();

    for (const symbol of normalized) {
        if (!symbol.container_name) {
            continue;
        }

        const children = childrenByContainer.get(symbol.container_name) ?? [];
        children.push(symbol);
        childrenByContainer.set(symbol.container_name, children);
    }

    return normalized
        .filter(symbol => !symbol.container_name)
        .map(symbol => ({
            symbol,
            children: (childrenByContainer.get(symbol.qualified_name) ?? []).map(child => ({
                symbol: child,
                children: [],
            })),
        }));
}

export function filterPrSMWorkspaceSymbols(symbols: PrismIndexedSymbol[], query: string): PrismIndexedSymbol[] {
    const normalizedQuery = query.trim().toLowerCase();
    return symbols
        .map(symbol => ({ symbol, score: getScore(symbol, normalizedQuery) }))
        .filter((entry): entry is { symbol: PrismIndexedSymbol; score: number } => entry.score !== null)
        .sort((left, right) => {
            if (left.score !== right.score) {
                return left.score - right.score;
            }

            return compareSymbols(left.symbol, right.symbol);
        })
        .map(entry => entry.symbol);
}

function getScore(symbol: PrismIndexedSymbol, query: string): number | null {
    if (!query) {
        return 4;
    }

    const name = symbol.name.toLowerCase();
    const qualifiedName = symbol.qualified_name.toLowerCase();
    const signature = symbol.signature.toLowerCase();

    if (name === query) {
        return 0;
    }
    if (qualifiedName === query) {
        return 1;
    }
    if (name.startsWith(query)) {
        return 2;
    }
    if (qualifiedName.includes(query)) {
        return 3;
    }
    if (signature.includes(query)) {
        return 4;
    }

    return null;
}

function compareSymbols(left: PrismIndexedSymbol, right: PrismIndexedSymbol): number {
    return left.file.localeCompare(right.file)
        || left.line - right.line
        || left.col - right.col
        || left.qualified_name.localeCompare(right.qualified_name);
}