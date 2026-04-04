import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import { getOutputDirCandidates, readMoonProject } from './project-config';

/**
 * Bridge to C# extension (OmniSharp / C# Dev Kit).
 * Queries the generated .cs files for type information,
 * then feeds results back to Moon's CompletionProvider.
 */
export class CSharpBridge {

    private outputDir: string = '';
    private ready = false;

    constructor() {
        this.detectOutputDir();
    }

    private detectOutputDir() {
        const folders = vscode.workspace.workspaceFolders;
        if (!folders) return;

        const root = folders[0].uri.fsPath;
        const config = readMoonProject(root);
        const resolved = getOutputDirCandidates(root, config).find(candidate => fs.existsSync(candidate));
        if (!resolved) return;

        this.outputDir = resolved;
        this.ready = true;
    }

    /**
     * Get completion items from C# extension for a symbol at a position
     * in the generated .cs file.
     */
    async getCompletionsFromCSharp(
        typeName: string,
        memberPrefix: string
    ): Promise<vscode.CompletionItem[]> {
        if (!this.ready) return [];

        // Find the generated .cs file for this type
        const csFile = path.join(this.outputDir, typeName + '.cs');
        if (!fs.existsSync(csFile)) return [];

        try {
            const csUri = vscode.Uri.file(csFile);
            const csDoc = await vscode.workspace.openTextDocument(csUri);

            // Find a position inside the class body where we can query completions
            // Look for "this." or the class name to find a good position
            const text = csDoc.getText();
            const classMatch = text.match(new RegExp(`class\\s+${typeName}`));
            if (!classMatch || classMatch.index === undefined) return [];

            // Find first method body — look for opening { after a method
            const methodStart = text.indexOf('{', text.indexOf('{', classMatch.index) + 1);
            if (methodStart < 0) return [];

            const pos = csDoc.positionAt(methodStart + 2);

            // Query C# extension for completions at this position
            const completions = await vscode.commands.executeCommand<vscode.CompletionList>(
                'vscode.executeCompletionItemProvider',
                csUri,
                pos,
                '.' // trigger character
            );

            if (!completions || !completions.items) return [];

            // Filter and convert
            return completions.items
                .filter(item => {
                    const label = typeof item.label === 'string' ? item.label : item.label.label;
                    return !memberPrefix || label.toLowerCase().startsWith(memberPrefix.toLowerCase());
                })
                .map(item => {
                    const newItem = new vscode.CompletionItem(
                        item.label,
                        item.kind
                    );
                    newItem.detail = (item.detail || '') + ' (via C#)';
                    newItem.documentation = item.documentation;
                    newItem.sortText = 'z' + (typeof item.label === 'string' ? item.label : item.label.label);
                    return newItem;
                });
        } catch (e) {
            // C# extension not ready or file not in project
            return [];
        }
    }

    /**
     * Get hover/signature info for a member from C# extension.
     */
    async getHoverFromCSharp(
        typeName: string,
        memberName: string
    ): Promise<string | undefined> {
        if (!this.ready) return undefined;

        const csFile = path.join(this.outputDir, typeName + '.cs');
        if (!fs.existsSync(csFile)) return undefined;

        try {
            const csUri = vscode.Uri.file(csFile);
            const csDoc = await vscode.workspace.openTextDocument(csUri);
            const text = csDoc.getText();

            // Find the member in the generated C# file
            const memberRegex = new RegExp(`\\b${memberName}\\b`);
            const match = memberRegex.exec(text);
            if (!match || match.index === undefined) return undefined;

            const pos = csDoc.positionAt(match.index);

            const hovers = await vscode.commands.executeCommand<vscode.Hover[]>(
                'vscode.executeHoverProvider',
                csUri,
                pos
            );

            if (!hovers || hovers.length === 0) return undefined;

            // Extract text from hover
            return hovers.map(h =>
                h.contents.map(c => {
                    if (typeof c === 'string') return c;
                    if ('value' in c) return c.value;
                    return '';
                }).join('\n')
            ).join('\n');
        } catch {
            return undefined;
        }
    }

    /**
     * Query all members of a Unity type by using C# extension on any .cs file.
     */
    async getUnityTypeMembers(typeName: string): Promise<vscode.CompletionItem[]> {
        if (!this.ready) return [];

        // Find any .cs file in the output directory
        if (!fs.existsSync(this.outputDir)) return [];

        const csFiles = fs.readdirSync(this.outputDir).filter(f => f.endsWith('.cs'));
        if (csFiles.length === 0) return [];

        const csFile = path.join(this.outputDir, csFiles[0]);

        try {
            const csUri = vscode.Uri.file(csFile);
            const csDoc = await vscode.workspace.openTextDocument(csUri);
            const text = csDoc.getText();

            // Find usage of the type in the file, or any variable declaration
            const typeRegex = new RegExp(`\\b${typeName}\\b`);
            const match = typeRegex.exec(text);
            if (!match || match.index === undefined) return [];

            // Try to get definition info
            const pos = csDoc.positionAt(match.index);

            const completions = await vscode.commands.executeCommand<vscode.CompletionList>(
                'vscode.executeCompletionItemProvider',
                csUri,
                pos,
                '.'
            );

            if (!completions || !completions.items) return [];

            return completions.items.map(item => {
                const newItem = new vscode.CompletionItem(item.label, item.kind);
                newItem.detail = (item.detail || '') + ' (via C#)';
                newItem.documentation = item.documentation;
                return newItem;
            });
        } catch {
            return [];
        }
    }
}
