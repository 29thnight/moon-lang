import * as path from 'path';
import * as vscode from 'vscode';
import { MoonIndexedSymbol, runMoonProjectSymbols } from './moonc-cli';
import { buildMoonDocumentSymbolTree, filterMoonWorkspaceSymbols, MoonSymbolNode } from './symbol-helpers';

export class MoonSymbolProvider implements vscode.DocumentSymbolProvider, vscode.WorkspaceSymbolProvider {
    private cachedSymbols: MoonIndexedSymbol[] | null = null;
    private inflight: Promise<MoonIndexedSymbol[]> | null = null;

    invalidate() {
        this.cachedSymbols = null;
        this.inflight = null;
    }

    async provideDocumentSymbols(
        document: vscode.TextDocument,
        token: vscode.CancellationToken,
    ): Promise<vscode.DocumentSymbol[] | undefined> {
        if (!vscode.workspace.isTrusted || document.uri.scheme !== 'file') {
            return undefined;
        }

        const symbols = await this.getProjectSymbols(document.uri.fsPath);
        if (token.isCancellationRequested) {
            return undefined;
        }

        const targetPath = path.normalize(document.uri.fsPath);
        const documentSymbols = symbols.filter(symbol => path.normalize(symbol.file) === targetPath);
        return buildMoonDocumentSymbolTree(documentSymbols).map(toDocumentSymbol);
    }

    async provideWorkspaceSymbols(
        query: string,
        token: vscode.CancellationToken,
    ): Promise<vscode.SymbolInformation[]> {
        if (!vscode.workspace.isTrusted) {
            return [];
        }

        const symbols = await this.getProjectSymbols();
        if (token.isCancellationRequested) {
            return [];
        }

        return filterMoonWorkspaceSymbols(symbols, query)
            .slice(0, 200)
            .map(symbol => new vscode.SymbolInformation(
                symbol.name,
                toVscodeSymbolKind(symbol.kind),
                symbol.container_name ?? '',
                new vscode.Location(vscode.Uri.file(symbol.file), toRange(symbol)),
            ));
    }

    private async getProjectSymbols(resourcePath?: string): Promise<MoonIndexedSymbol[]> {
        if (this.cachedSymbols) {
            return this.cachedSymbols;
        }

        if (!this.inflight) {
            this.inflight = runMoonProjectSymbols(resourcePath)
                .then(symbols => {
                    this.cachedSymbols = symbols;
                    return symbols;
                })
                .finally(() => {
                    this.inflight = null;
                });
        }

        return this.inflight;
    }
}

function toDocumentSymbol(node: MoonSymbolNode): vscode.DocumentSymbol {
    const symbol = new vscode.DocumentSymbol(
        node.symbol.name,
        node.symbol.signature,
        toVscodeSymbolKind(node.symbol.kind),
        toRange(node.symbol),
        toRange(node.symbol),
    );
    symbol.children = node.children.map(toDocumentSymbol);
    return symbol;
}

function toVscodeSymbolKind(kind: string): vscode.SymbolKind {
    switch (kind) {
        case 'component':
        case 'class':
        case 'data class':
            return vscode.SymbolKind.Class;
        case 'asset':
            return vscode.SymbolKind.Object;
        case 'attribute':
            return vscode.SymbolKind.Interface;
        case 'enum':
            return vscode.SymbolKind.Enum;
        case 'enum-entry':
            return vscode.SymbolKind.EnumMember;
        case 'function':
        case 'coroutine':
        case 'lifecycle':
            return vscode.SymbolKind.Method;
        case 'field':
        case 'serialize-field':
        case 'required-component':
        case 'optional-component':
        case 'child-component':
        case 'parent-component':
            return vscode.SymbolKind.Field;
        default:
            return vscode.SymbolKind.Variable;
    }
}

function toRange(symbol: MoonIndexedSymbol): vscode.Range {
    const startLine = Math.max(0, symbol.line - 1);
    const startCol = Math.max(0, symbol.col - 1);
    const endLine = Math.max(startLine, (symbol.end_line ?? symbol.line) - 1);
    const endCol = Math.max(startCol + 1, (symbol.end_col ?? symbol.col) - 1);
    return new vscode.Range(startLine, startCol, endLine, endCol);
}