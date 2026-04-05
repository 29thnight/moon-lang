import * as vscode from 'vscode';
import { LanguageClient, LanguageClientOptions, ServerOptions, State, TransportKind } from 'vscode-languageclient/node';
import {
    getBundledCompilerCandidates,
    getProjectCompilerPath,
    getWorkspaceDevCompilerCandidates,
    resolveCompilerPathFromContext,
} from './compiler-resolver';

let client: LanguageClient | undefined;

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

export async function stopPrismLanguageClient(): Promise<void> {
    if (!client) {
        return;
    }

    const runningClient = client;
    client = undefined;
    await runningClient.stop();
}