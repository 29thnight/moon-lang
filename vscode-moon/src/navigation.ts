import * as vscode from 'vscode';
import { CSharpBridge } from './csharp-bridge';
import { getNavigationCSharpTarget } from './csharp-navigation';
import {
    MoonDefinitionEntry,
    MoonIndexResult,
    MoonIndexedReference,
    MoonIndexedSymbol,
    MoonSourceLocation,
    runMoonDefinitionForPosition,
    runMoonIndexForPosition,
    runMoonReferencesForPosition,
} from './moonc-cli';
import { getNavigationFallbackTarget, getNavigationHoverText } from './navigation-helpers';
import { getMoonRenamePlan, getMoonRenameSupportError, validateMoonRenameName } from './navigation-rename';

export class MoonNavigationProvider implements vscode.DefinitionProvider, vscode.HoverProvider, vscode.ReferenceProvider, vscode.RenameProvider {
    private readonly csharp = new CSharpBridge();

    async provideDefinition(
        document: vscode.TextDocument,
        position: vscode.Position,
        token: vscode.CancellationToken,
    ): Promise<vscode.Definition | undefined> {
        if (!vscode.workspace.isTrusted || document.uri.scheme !== 'file') {
            return undefined;
        }

        const line = position.line + 1;
        const col = position.character + 1;
        const definition = await runMoonDefinitionForPosition(document.uri.fsPath, line, col);
        if (token.isCancellationRequested) {
            return undefined;
        }

        if (definition) {
            return sourceLocationToDefinition(definition);
        }

        const indexResult = await runMoonIndexForPosition(document.uri.fsPath, line, col);
        if (token.isCancellationRequested || !indexResult) {
            return undefined;
        }

        const fallbackTarget = getNavigationFallbackTarget(indexResult);
        return fallbackTarget ? sourceLocationToDefinition(fallbackTarget) : undefined;
    }

    async provideHover(
        document: vscode.TextDocument,
        position: vscode.Position,
        token: vscode.CancellationToken,
    ): Promise<vscode.Hover | undefined> {
        if (!vscode.workspace.isTrusted || document.uri.scheme !== 'file') {
            return undefined;
        }

        const line = position.line + 1;
        const col = position.character + 1;
        const indexResult = await runMoonIndexForPosition(document.uri.fsPath, line, col);
        if (token.isCancellationRequested || !indexResult) {
            return undefined;
        }

        let definition: MoonDefinitionEntry | null = null;
        if (indexResult.symbol_at || indexResult.reference_at?.resolved_symbol) {
            definition = await runMoonDefinitionForPosition(document.uri.fsPath, line, col);
            if (token.isCancellationRequested) {
                return undefined;
            }
        }

        const csharpTarget = getNavigationCSharpTarget(indexResult, definition);
        const csharpInfo = csharpTarget ? await this.csharp.getHoverDetails(csharpTarget) : null;
        if (token.isCancellationRequested) {
            return undefined;
        }

        const hoverText = getNavigationHoverText(indexResult, definition, csharpInfo);
        if (!hoverText) {
            return undefined;
        }

        const markdown = new vscode.MarkdownString(hoverText);
        return new vscode.Hover(markdown, indexResultToRange(indexResult));
    }

    async provideReferences(
        document: vscode.TextDocument,
        position: vscode.Position,
        context: vscode.ReferenceContext,
        token: vscode.CancellationToken,
    ): Promise<vscode.Location[] | undefined> {
        if (!vscode.workspace.isTrusted || document.uri.scheme !== 'file') {
            return undefined;
        }

        const line = position.line + 1;
        const col = position.character + 1;
        const result = await runMoonReferencesForPosition(document.uri.fsPath, line, col);
        if (token.isCancellationRequested || !result?.definition) {
            return undefined;
        }

        const seen = new Set<string>();
        const locations: vscode.Location[] = [];

        if (context.includeDeclaration) {
            const definitionLocation = sourceLocationToDefinition(result.definition);
            locations.push(definitionLocation);
            seen.add(locationKey(result.definition));
        }

        for (const reference of result.references) {
            const key = locationKey(reference);
            if (seen.has(key)) {
                continue;
            }
            seen.add(key);
            locations.push(new vscode.Location(vscode.Uri.file(reference.file), sourceLocationToRange(reference)));
        }

        return locations;
    }

    async prepareRename(
        document: vscode.TextDocument,
        position: vscode.Position,
        token: vscode.CancellationToken,
    ): Promise<vscode.Range | { range: vscode.Range; placeholder: string } | undefined> {
        if (!vscode.workspace.isTrusted || document.uri.scheme !== 'file') {
            return undefined;
        }

        const line = position.line + 1;
        const col = position.character + 1;
        const [referencesResult, indexResult] = await Promise.all([
            runMoonReferencesForPosition(document.uri.fsPath, line, col),
            runMoonIndexForPosition(document.uri.fsPath, line, col),
        ]);
        if (token.isCancellationRequested) {
            return undefined;
        }

        const plan = getMoonRenamePlan(referencesResult);
        if (!plan) {
            throw new Error(getMoonRenameSupportError(referencesResult));
        }

        const target = indexResult?.symbol_at ?? indexResult?.reference_at ?? referencesResult?.definition;
        if (!target) {
            throw new Error(getMoonRenameSupportError(referencesResult));
        }

        return {
            range: sourceLocationToRange(target),
            placeholder: plan.placeholder,
        };
    }

    async provideRenameEdits(
        document: vscode.TextDocument,
        position: vscode.Position,
        newName: string,
        token: vscode.CancellationToken,
    ): Promise<vscode.WorkspaceEdit | undefined> {
        if (!vscode.workspace.isTrusted || document.uri.scheme !== 'file') {
            return undefined;
        }

        const line = position.line + 1;
        const col = position.character + 1;
        const referencesResult = await runMoonReferencesForPosition(document.uri.fsPath, line, col);
        if (token.isCancellationRequested) {
            return undefined;
        }

        const plan = getMoonRenamePlan(referencesResult);
        if (!plan) {
            throw new Error(getMoonRenameSupportError(referencesResult));
        }

        const validationError = validateMoonRenameName(newName);
        if (validationError) {
            throw new Error(validationError);
        }

        const edit = new vscode.WorkspaceEdit();
        for (const location of plan.locations) {
            edit.replace(vscode.Uri.file(location.file), sourceLocationToRange(location), newName);
        }

        return edit;
    }
}

function sourceLocationToDefinition(location: MoonDefinitionEntry | MoonIndexedSymbol): vscode.Location {
    return new vscode.Location(vscode.Uri.file(location.file), sourceLocationToRange(location));
}

function indexResultToRange(result: MoonIndexResult): vscode.Range | undefined {
    if (result.symbol_at) {
        return sourceLocationToRange(result.symbol_at);
    }

    if (result.reference_at) {
        return sourceLocationToRange(result.reference_at);
    }

    return undefined;
}

function sourceLocationToRange(location: MoonSourceLocation | MoonIndexedReference): vscode.Range {
    const startLine = Math.max(0, location.line - 1);
    const startCol = Math.max(0, location.col - 1);
    const fallbackEndCol = startCol + 1;
    let endLine = Math.max(0, (location.end_line ?? location.line) - 1);
    let endCol = Math.max(0, (location.end_col ?? location.col) - 1);

    if (endLine < startLine || (endLine === startLine && endCol <= startCol)) {
        endLine = startLine;
        endCol = fallbackEndCol;
    }

    return new vscode.Range(startLine, startCol, endLine, endCol);
}

function locationKey(location: MoonSourceLocation): string {
    return [location.file, location.line, location.col, location.end_line ?? location.line, location.end_col ?? location.col].join(':');
}