import * as vscode from 'vscode';
import { LanguageClient, LanguageClientOptions, ServerOptions, State, TransportKind } from 'vscode-languageclient/node';
import { CSharpBridge } from './csharp-bridge';
import { getNavigationCSharpTarget } from './csharp-navigation';
import {
    getBundledCompilerCandidates,
    getProjectCompilerPath,
    getWorkspaceDevCompilerCandidates,
    resolveCompilerPathFromContext,
} from './compiler-resolver';
import { getNavigationCSharpHoverSection } from './navigation-helpers';
import { runPrSMDefinitionForPosition, runPrSMIndexForPosition } from './prism-cli';

let client: LanguageClient | undefined;
const csharpBridge = new CSharpBridge();

export async function startPrismLanguageClient(
    context: vscode.ExtensionContext,
    statusBar: vscode.StatusBarItem,
): Promise<void> {
    if (client) {
        return;
    }

    const workspaceRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
    const compilerPath = resolveCompilerPathFromContext({
        userOverride: vscode.workspace.getConfiguration('prsm').get<string>('compilerPath', ''),
        projectCompilerPath: getProjectCompilerPath(workspaceRoot),
        bundledCandidates: getBundledCompilerCandidates(context.extensionPath),
        devCandidates: getWorkspaceDevCompilerCandidates(workspaceRoot),
        fallback: 'prism',
    });

    const serverOptions: ServerOptions = {
        run: {
            command: compilerPath,
            args: ['lsp'],
            transport: TransportKind.stdio,
            options: {
                cwd: workspaceRoot ?? context.extensionPath,
            },
        },
        debug: {
            command: compilerPath,
            args: ['lsp'],
            transport: TransportKind.stdio,
            options: {
                cwd: workspaceRoot ?? context.extensionPath,
            },
        },
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ language: 'prsm', scheme: 'file' }],
        outputChannelName: 'PrSM Language Server',
        middleware: {
            provideHover: (document, position, token, next) =>
                provideAugmentedHover(document, position, token, next),
        },
    };

    client = new LanguageClient(
        'prsmLanguageServer',
        'PrSM Language Server',
        serverOptions,
        clientOptions,
    );

    client.onDidChangeState(event => {
        switch (event.newState) {
            case State.Starting:
                statusBar.text = '$(sync~spin) PrSM (LSP)';
                break;
            case State.Running:
                statusBar.text = '$(check) PrSM (LSP)';
                break;
            case State.Stopped:
                statusBar.text = '$(warning) PrSM (LSP stopped)';
                break;
        }
    });

    statusBar.text = '$(sync~spin) PrSM (LSP)';
    await client.start();
}

async function provideAugmentedHover(
    document: vscode.TextDocument,
    position: vscode.Position,
    token: vscode.CancellationToken,
    next: (
        document: vscode.TextDocument,
        position: vscode.Position,
        token: vscode.CancellationToken,
    ) => vscode.ProviderResult<vscode.Hover>,
): Promise<vscode.Hover | undefined> {
    const hover = await Promise.resolve(next(document, position, token));

    if (!vscode.workspace.isTrusted || document.languageId !== 'prsm' || document.uri.scheme !== 'file') {
        return hover ?? undefined;
    }

    if (token.isCancellationRequested) {
        return hover ?? undefined;
    }

    const existingMarkdown = hoverContentsToMarkdown(hover ?? undefined);
    if (existingMarkdown.includes('[Generated C#]')) {
        return hover ?? undefined;
    }

    const line = position.line + 1;
    const col = position.character + 1;
    const indexResult = await runPrSMIndexForPosition(document.uri.fsPath, line, col);
    if (token.isCancellationRequested || !indexResult) {
        return hover ?? undefined;
    }

    let definition = null;
    if (indexResult.symbol_at || indexResult.reference_at?.resolved_symbol) {
        definition = await runPrSMDefinitionForPosition(document.uri.fsPath, line, col);
        if (token.isCancellationRequested) {
            return hover ?? undefined;
        }
    }

    const csharpTarget = getNavigationCSharpTarget(indexResult, definition);
    if (!csharpTarget) {
        return hover ?? undefined;
    }

    const csharpInfo = await csharpBridge.getHoverDetails(csharpTarget, document.uri.fsPath);
    if (token.isCancellationRequested || !csharpInfo) {
        return hover ?? undefined;
    }

    if (!csharpInfo.generatedFile && existingMarkdown.includes('[Unity API]')) {
        return hover ?? undefined;
    }

    const supplement = getNavigationCSharpHoverSection(csharpInfo);
    if (!supplement) {
        return hover ?? undefined;
    }

    const mergedMarkdown = mergeHoverMarkdown(
        existingMarkdown,
        supplement,
        csharpInfo.generatedFile ? '**[Unity API]**' : undefined,
    );
    return new vscode.Hover(new vscode.MarkdownString(mergedMarkdown), hover?.range);
}

function mergeHoverMarkdown(existingMarkdown: string, supplement: string, insertBeforeMarker?: string): string {
    if (!existingMarkdown.trim()) {
        return supplement;
    }

    if (insertBeforeMarker && existingMarkdown.includes(insertBeforeMarker)) {
        return existingMarkdown.replace(insertBeforeMarker, `${supplement}\n\n${insertBeforeMarker}`);
    }

    return `${existingMarkdown}\n\n${supplement}`;
}

function hoverContentsToMarkdown(hover: vscode.Hover | undefined): string {
    if (!hover) {
        return '';
    }

    const contents = Array.isArray(hover.contents) ? hover.contents : [hover.contents];
    return contents
        .map(content => {
            if (typeof content === 'string') {
                return content;
            }

            if (content instanceof vscode.MarkdownString) {
                return content.value;
            }

            if ('value' in content) {
                return content.value;
            }

            return '';
        })
        .filter(Boolean)
        .join('\n\n');
}

export async function stopPrismLanguageClient(): Promise<void> {
    if (!client) {
        return;
    }

    const runningClient = client;
    client = undefined;
    await runningClient.stop();
}