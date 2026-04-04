import { MoonIndexedSymbol } from './moonc-cli';

export interface MoonSymbolNode {
    symbol: MoonIndexedSymbol;
    children: MoonSymbolNode[];
}

export function buildMoonDocumentSymbolTree(symbols: MoonIndexedSymbol[]): MoonSymbolNode[] {
    const normalized = [...symbols].sort(compareSymbols);
    const childrenByContainer = new Map<string, MoonIndexedSymbol[]>();

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

export function filterMoonWorkspaceSymbols(symbols: MoonIndexedSymbol[], query: string): MoonIndexedSymbol[] {
    const normalizedQuery = query.trim().toLowerCase();
    return symbols
        .map(symbol => ({ symbol, score: getScore(symbol, normalizedQuery) }))
        .filter((entry): entry is { symbol: MoonIndexedSymbol; score: number } => entry.score !== null)
        .sort((left, right) => {
            if (left.score !== right.score) {
                return left.score - right.score;
            }

            return compareSymbols(left.symbol, right.symbol);
        })
        .map(entry => entry.symbol);
}

function getScore(symbol: MoonIndexedSymbol, query: string): number | null {
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

function compareSymbols(left: MoonIndexedSymbol, right: MoonIndexedSymbol): number {
    return left.file.localeCompare(right.file)
        || left.line - right.line
        || left.col - right.col
        || left.qualified_name.localeCompare(right.qualified_name);
}