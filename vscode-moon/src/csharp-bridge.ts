import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import type { MoonCSharpHoverInfo, MoonCSharpLookupTarget } from './csharp-navigation';
import { findGeneratedSpanForTarget, readGeneratedSourceMap } from './generated-source-map';
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

    async getHoverDetails(target: MoonCSharpLookupTarget): Promise<MoonCSharpHoverInfo | null> {
        if (!this.ready) {
            return null;
        }

        const generatedFile = this.getGeneratedFilePath(target.typeName);
        const hoverText = target.memberName
            ? await this.getMemberHoverFromCSharp(target.typeName, target.memberName, generatedFile)
            : await this.getTypeHoverFromCSharp(target.typeName, generatedFile);

        if (!generatedFile && !hoverText) {
            return null;
        }

        return {
            ...target,
            generatedFile,
            hoverText,
        };
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
        const csFile = this.getGeneratedFilePath(typeName);
        if (!csFile || !fs.existsSync(csFile)) return [];

        try {
            const csUri = vscode.Uri.file(csFile);
            const csDoc = await vscode.workspace.openTextDocument(csUri);

            // Find a position inside the class body where we can query completions
            // Look for "this." or the class name to find a good position
            const text = csDoc.getText();
            const classMatch = text.match(new RegExp(`class\\s+${escapeRegExp(typeName)}`));
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
        return this.getMemberHoverFromCSharp(typeName, memberName, this.getGeneratedFilePath(typeName));
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

    private getGeneratedFilePath(typeName: string): string | undefined {
        if (!this.ready) {
            return undefined;
        }

        const candidate = path.join(this.outputDir, `${typeName}.cs`);
        return fs.existsSync(candidate) ? candidate : undefined;
    }

    private async getTypeHoverFromCSharp(typeName: string, generatedFile?: string): Promise<string | undefined> {
        if (generatedFile) {
            const hoverFromSourceMap = await this.getHoverFromSourceMap(generatedFile, typeName);
            if (hoverFromSourceMap) {
                return hoverFromSourceMap;
            }
        }

        for (const csFile of this.getHoverSearchFiles(generatedFile)) {
            try {
                const csUri = vscode.Uri.file(csFile);
                const csDoc = await vscode.workspace.openTextDocument(csUri);
                const index = this.findTypeIndex(csDoc.getText(), typeName);
                if (index === undefined) {
                    continue;
                }

                return this.getHoverAt(csUri, csDoc.positionAt(index));
            } catch {
                continue;
            }
        }

        return undefined;
    }

    private async getMemberHoverFromCSharp(
        typeName: string,
        memberName: string,
        generatedFile?: string,
    ): Promise<string | undefined> {
        const csFile = generatedFile ?? this.getGeneratedFilePath(typeName);
        if (!csFile) {
            return undefined;
        }

        try {
            const csUri = vscode.Uri.file(csFile);
            const csDoc = await vscode.workspace.openTextDocument(csUri);
            const sourceMapPosition = this.getSourceMapPosition(csFile, typeName, memberName);
            if (sourceMapPosition) {
                return this.getHoverAt(csUri, sourceMapPosition);
            }

            const index = this.findMemberIndex(csDoc.getText(), memberName);
            if (index === undefined) {
                return undefined;
            }

            return this.getHoverAt(csUri, csDoc.positionAt(index));
        } catch {
            return undefined;
        }
    }

    private async getHoverFromSourceMap(
        generatedFile: string,
        typeName: string,
        memberName?: string,
    ): Promise<string | undefined> {
        const sourceMapPosition = this.getSourceMapPosition(generatedFile, typeName, memberName);
        if (!sourceMapPosition) {
            return undefined;
        }

        try {
            const csUri = vscode.Uri.file(generatedFile);
            return this.getHoverAt(csUri, sourceMapPosition);
        } catch {
            return undefined;
        }
    }

    private getSourceMapPosition(
        generatedFile: string,
        typeName: string,
        memberName?: string,
    ): vscode.Position | undefined {
        const sourceMap = readGeneratedSourceMap(generatedFile);
        const span = sourceMap ? findGeneratedSpanForTarget(sourceMap, typeName, memberName) : null;
        if (!span) {
            return undefined;
        }

        return new vscode.Position(
            Math.max(0, span.line - 1),
            Math.max(0, span.col - 1),
        );
    }

    private getHoverSearchFiles(preferredFile?: string): string[] {
        if (!this.ready || !fs.existsSync(this.outputDir)) {
            return [];
        }

        const files = fs.readdirSync(this.outputDir)
            .filter(name => name.endsWith('.cs'))
            .map(name => path.join(this.outputDir, name));

        if (!preferredFile) {
            return files;
        }

        return [preferredFile, ...files.filter(file => file !== preferredFile)];
    }

    private async getHoverAt(csUri: vscode.Uri, position: vscode.Position): Promise<string | undefined> {
        const hovers = await vscode.commands.executeCommand<vscode.Hover[]>(
            'vscode.executeHoverProvider',
            csUri,
            position,
        );

        if (!hovers || hovers.length === 0) {
            return undefined;
        }

        return hovers
            .map(hover => hover.contents
                .map(content => {
                    if (typeof content === 'string') {
                        return content;
                    }

                    if ('value' in content) {
                        return content.value;
                    }

                    return '';
                })
                .filter(Boolean)
                .join('\n'))
            .filter(Boolean)
            .join('\n');
    }

    private findTypeIndex(text: string, typeName: string): number | undefined {
        const escaped = escapeRegExp(typeName);
        const patterns = [
            new RegExp(`\\b(?:class|enum|struct|interface)\\s+(${escaped})\\b`),
            new RegExp(`\\b(${escaped})\\b`),
        ];

        return findCaptureIndex(text, patterns);
    }

    private findMemberIndex(text: string, memberName: string): number | undefined {
        const escaped = escapeRegExp(memberName);
        const patterns = [
            new RegExp(`\\b(${escaped})\\b\\s*\\(`),
            new RegExp(`\\b(${escaped})\\b\\s*(?:[;=,])`),
            new RegExp(`\\b(${escaped})\\b`),
        ];

        return findCaptureIndex(text, patterns);
    }
}

function findCaptureIndex(text: string, patterns: RegExp[]): number | undefined {
    for (const pattern of patterns) {
        const match = pattern.exec(text);
        const captured = match?.[1];
        if (!match || match.index === undefined || !captured) {
            continue;
        }

        const captureOffset = match[0].indexOf(captured);
        if (captureOffset >= 0) {
            return match.index + captureOffset;
        }
    }

    return undefined;
}

function escapeRegExp(value: string): string {
    return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}
