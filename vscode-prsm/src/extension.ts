import * as vscode from 'vscode';
import * as path from 'path';
import { checkDocument, compileFile, compileDirectory } from './diagnostics';
import { PrismCompletionProvider } from './completion';
import { PrismExplorerProvider } from './sidebar';
import { PrismVisualizer } from './visualizer';
import { PrismGraphView } from './graph-view';
import {
    findGeneratedSpanForSourcePosition,
    findSourceAnchorForGeneratedPosition,
    readGeneratedSourceMap,
    resolveSourceMapSourcePath,
    type PrismGeneratedSourceMapSpan,
} from './generated-source-map';
import { insertLifecycleBlock } from './lifecycle-inserter';
import { openFromStackTraceCommand } from './stack-trace-navigator';
import { startPrismLanguageClient, stopPrismLanguageClient } from './lsp-client';
import { PrismNavigationProvider } from './navigation';
import { PrismSymbolProvider } from './symbols';
import { resolveGeneratedCsPath as resolveGeneratedCsPathFromConfig } from './project-config';

let diagCollection: vscode.DiagnosticCollection | undefined;
let statusBar: vscode.StatusBarItem;
let trustedFeaturesActivated = false;
let legacyCompletionDisposable: vscode.Disposable | undefined;

export async function activate(context: vscode.ExtensionContext) {
    const isTrusted = vscode.workspace.isTrusted;

    // Add prism to VSCode terminal PATH
    const prismBinDir = path.join(context.extensionPath, 'bin');
    context.environmentVariableCollection.append('PATH', path.delimiter + prismBinDir);

    // Sidebar: PrSM Explorer
    const prsmExplorer = new PrismExplorerProvider();
    vscode.window.registerTreeDataProvider('prsmExplorer', prsmExplorer);
    context.subscriptions.push(
        vscode.commands.registerCommand('prsm.refreshExplorer', () => prsmExplorer.refresh())
    );

    // Refresh sidebar when editor changes
    vscode.window.onDidChangeActiveTextEditor(e => {
        if (e && e.document.languageId === 'prsm') {
            prsmExplorer.refresh();
        }
    });

    // File watcher: refresh sidebar on .prsm file changes
    const mnWatcher = vscode.workspace.createFileSystemWatcher('**/*.prsm');
    mnWatcher.onDidCreate(() => {
        prsmExplorer.refresh();
    });
    mnWatcher.onDidDelete(() => {
        prsmExplorer.refresh();
    });
    mnWatcher.onDidChange(() => {
        prsmExplorer.refresh();
    });
    context.subscriptions.push(mnWatcher);

    // Visualizer command
    context.subscriptions.push(
        vscode.commands.registerCommand('prsm.visualize', () => {
            const editor = vscode.window.activeTextEditor;
            if (editor && editor.document.languageId === 'prsm') {
                PrismVisualizer.show(editor.document);
            } else {
                vscode.window.showWarningMessage('No .prsm file is open');
            }
        })
    );

    // Lifecycle inserter command
    context.subscriptions.push(
        vscode.commands.registerCommand('prsm.insertLifecycle', insertLifecycleBlock)
    );

    // Graph View command
    context.subscriptions.push(
        vscode.commands.registerCommand('prsm.graphView', () => {
            PrismGraphView.show(context);
        })
    );

    // Status bar
    statusBar = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
    statusBar.command = 'workbench.action.problems.focus';
    statusBar.tooltip = 'PrSM language status — click to open Problems';
    context.subscriptions.push(statusBar);

    // Show status bar when a .prsm file is active
    updateStatusBarVisibility(isTrusted);
    vscode.window.onDidChangeActiveTextEditor(() => updateStatusBarVisibility(isTrusted));

    if (isTrusted) {
        await activateTrustedLanguageFeatures(context, prsmExplorer);
    } else {
        ensureLegacyCompletionProvider(context);
        statusBar.text = '$(shield) PrSM (restricted)';
    }

    context.subscriptions.push(
        vscode.workspace.onDidGrantWorkspaceTrust(async () => {
            if (!trustedFeaturesActivated) {
                await activateTrustedLanguageFeatures(context, prsmExplorer);
            }
        })
    );

    // Command: Compile Current File
    context.subscriptions.push(
        vscode.commands.registerCommand('prsm.compileFile', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor || editor.document.languageId !== 'prsm') {
                vscode.window.showWarningMessage('No .prsm file is open');
                return;
            }

            // Save first
            await editor.document.save();
            const filePath = editor.document.uri.fsPath;

            vscode.window.withProgress({
                location: vscode.ProgressLocation.Notification,
                title: 'PrSM: Compiling...',
                cancellable: false
            }, async () => {
                const result = await compileFile(filePath);
                if (result.success) {
                    vscode.window.showInformationMessage(`PrSM: Compiled successfully`);
                } else {
                    vscode.window.showErrorMessage(`PrSM: Compilation failed\n${result.output}`);
                }
                if (diagCollection) {
                    checkDocument(editor.document, diagCollection, statusBar);
                }
            });
        })
    );

    // Command: Compile Workspace
    context.subscriptions.push(
        vscode.commands.registerCommand('prsm.compileWorkspace', async () => {
            const folders = vscode.workspace.workspaceFolders;
            if (!folders || folders.length === 0) {
                vscode.window.showWarningMessage('No workspace folder open');
                return;
            }

            const workspacePath = folders[0].uri.fsPath;

            vscode.window.withProgress({
                location: vscode.ProgressLocation.Notification,
                title: 'PrSM: Compiling workspace...',
                cancellable: false
            }, async () => {
                const result = await compileDirectory(workspacePath);
                if (result.success) {
                    vscode.window.showInformationMessage(`PrSM: Workspace compiled\n${result.output}`);
                } else {
                    vscode.window.showErrorMessage(`PrSM: Compilation failed\n${result.output}`);
                }
            });
        })
    );

    // Command: Check Current File
    context.subscriptions.push(
        vscode.commands.registerCommand('prsm.checkFile', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor || editor.document.languageId !== 'prsm') {
                vscode.window.showWarningMessage('No .prsm file is open');
                return;
            }

            await editor.document.save();
            if (diagCollection) {
                await checkDocument(editor.document, diagCollection, statusBar);
            }
            vscode.window.showInformationMessage('PrSM: Check complete');
        })
    );

    // Command: Show Generated C# (split right)
    context.subscriptions.push(
        vscode.commands.registerCommand('prsm.showGenerated', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor || editor.document.languageId !== 'prsm') {
                vscode.window.showWarningMessage('No .prsm file is open');
                return;
            }

            const csPath = resolveGeneratedCsPath(editor.document.uri.fsPath);
            if (!csPath) {
                vscode.window.showWarningMessage('Generated .cs file not found. Compile first.');
                return;
            }

            const csUri = vscode.Uri.file(csPath);
            const sourceMap = readGeneratedSourceMap(csPath);
            const generatedSelection = sourceMapSpanToRange(sourceMap
                ? findGeneratedSpanForSourcePosition(
                    sourceMap,
                    editor.selection.active.line + 1,
                    editor.selection.active.character + 1,
                )
                : null);
            const csDoc = await vscode.workspace.openTextDocument(csUri);
            await vscode.window.showTextDocument(csDoc, {
                viewColumn: vscode.ViewColumn.Beside,
                preview: true,
                preserveFocus: true,
                selection: generatedSelection,
            });
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('prsm.showSourceFromGenerated', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor || editor.document.uri.scheme !== 'file' || path.extname(editor.document.uri.fsPath).toLowerCase() !== '.cs') {
                vscode.window.showWarningMessage('No generated .cs file is open');
                return;
            }

            const generatedPath = editor.document.uri.fsPath;
            const sourceMap = readGeneratedSourceMap(generatedPath);
            if (!sourceMap) {
                vscode.window.showWarningMessage('PrSM source map sidecar not found for this generated .cs file. Compile first.');
                return;
            }

            const sourceAnchor = findSourceAnchorForGeneratedPosition(
                sourceMap,
                editor.selection.active.line + 1,
                editor.selection.active.character + 1,
            );
            if (!sourceAnchor) {
                vscode.window.showWarningMessage('No PrSM source anchor was found for the current generated C# position.');
                return;
            }

            const sourcePath = resolveSourceMapSourcePath(generatedPath, sourceMap, getWorkspaceRoots());
            if (!sourcePath) {
                vscode.window.showWarningMessage('PrSM source path could not be resolved from the generated source map.');
                return;
            }

            const sourceUri = vscode.Uri.file(sourcePath);
            const sourceDoc = await vscode.workspace.openTextDocument(sourceUri);
            await vscode.window.showTextDocument(sourceDoc, {
                viewColumn: editor.viewColumn,
                preview: true,
                selection: sourceMapSpanToRange(sourceAnchor.source_span),
            });
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('prsm.openFromStackTrace', async () => {
            await openFromStackTraceCommand(getWorkspaceRoots());
        })
    );

}

async function activateTrustedLanguageFeatures(
    context: vscode.ExtensionContext,
    prsmExplorer: PrismExplorerProvider,
) {
    trustedFeaturesActivated = true;

    try {
        await startPrismLanguageClient(context, statusBar);
        disposeLegacyCompletionProvider();
        context.subscriptions.push(
            vscode.workspace.onDidChangeTextDocument(event => {
                if (event.document.languageId === 'prsm') {
                    prsmExplorer.refresh();
                }
            }),
            vscode.workspace.onDidSaveTextDocument(document => {
                if (document.languageId === 'prsm') {
                    prsmExplorer.refresh();
                }
            }),
        );
        return;
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        vscode.window.showWarningMessage(`PrSM LSP start failed, falling back to legacy providers. ${message}`);
    }

    registerLegacyLanguageFeatures(context, prsmExplorer);
}

function registerLegacyLanguageFeatures(
    context: vscode.ExtensionContext,
    prsmExplorer: PrismExplorerProvider,
) {
    ensureLegacyCompletionProvider(context);

    const symbolProvider = new PrismSymbolProvider();
    const navigationProvider = new PrismNavigationProvider();

    context.subscriptions.push(
        vscode.languages.registerDefinitionProvider(
            { language: 'prsm', scheme: 'file' },
            navigationProvider,
        ),
        vscode.languages.registerHoverProvider(
            { language: 'prsm', scheme: 'file' },
            navigationProvider,
        ),
        vscode.languages.registerReferenceProvider(
            { language: 'prsm', scheme: 'file' },
            navigationProvider,
        ),
        vscode.languages.registerRenameProvider(
            { language: 'prsm', scheme: 'file' },
            navigationProvider,
        ),
        vscode.languages.registerDocumentSymbolProvider(
            { language: 'prsm', scheme: 'file' },
            symbolProvider,
        ),
        vscode.languages.registerWorkspaceSymbolProvider(symbolProvider),
    );

    diagCollection = vscode.languages.createDiagnosticCollection('prsm');
    context.subscriptions.push(diagCollection);

    statusBar.text = '$(check) PrSM (legacy)';

    let checkTimer: NodeJS.Timeout | undefined;
    context.subscriptions.push(
        vscode.workspace.onDidChangeTextDocument(event => {
            if (event.document.languageId === 'prsm') {
                if (checkTimer) {
                    clearTimeout(checkTimer);
                }
                checkTimer = setTimeout(() => {
                    if (diagCollection) {
                        checkDocument(event.document, diagCollection, statusBar);
                    }
                    prsmExplorer.refresh();
                    symbolProvider.invalidate();
                }, 500);
            }
        }),
        vscode.workspace.onDidSaveTextDocument(document => {
            if (document.languageId === 'prsm') {
                if (checkTimer) {
                    clearTimeout(checkTimer);
                }
                if (diagCollection) {
                    checkDocument(document, diagCollection, statusBar);
                }
                prsmExplorer.refresh();
                symbolProvider.invalidate();
            }
        }),
        vscode.workspace.onDidOpenTextDocument(document => {
            if (document.languageId === 'prsm' && diagCollection) {
                checkDocument(document, diagCollection, statusBar);
                symbolProvider.invalidate();
            }
        }),
        vscode.workspace.onDidCloseTextDocument(document => {
            if (document.languageId === 'prsm') {
                diagCollection?.delete(document.uri);
            }
        }),
    );

    vscode.workspace.textDocuments.forEach(document => {
        if (document.languageId === 'prsm' && diagCollection) {
            checkDocument(document, diagCollection, statusBar);
        }
    });
}

function ensureLegacyCompletionProvider(context: vscode.ExtensionContext) {
    if (legacyCompletionDisposable) {
        return;
    }

    const completionProvider = new PrismCompletionProvider(context.extensionPath);
    legacyCompletionDisposable = vscode.Disposable.from(
        vscode.languages.registerCompletionItemProvider(
            { language: 'prsm', scheme: 'file' },
            completionProvider,
            '.', '?', ':', '<'
        ),
        completionProvider,
    );
    context.subscriptions.push(legacyCompletionDisposable);
}

function disposeLegacyCompletionProvider() {
    legacyCompletionDisposable?.dispose();
    legacyCompletionDisposable = undefined;
}

/**
 * Resolve the generated .cs path for a .prsm file.
 * Reads output_dir from .prsmproject and falls back to common generated-code locations.
 */
function resolveGeneratedCsPath(prsmPath: string): string | null {
    return resolveGeneratedCsPathFromConfig(prsmPath, getWorkspaceRoots());
}

function getWorkspaceRoots(): string[] {
    return (vscode.workspace.workspaceFolders || []).map(folder => folder.uri.fsPath);
}

function sourceMapSpanToRange(span: PrismGeneratedSourceMapSpan | null | undefined): vscode.Range | undefined {
    if (!span) {
        return undefined;
    }

    const startLine = Math.max(0, span.line - 1);
    const startCol = Math.max(0, span.col - 1);
    let endLine = Math.max(startLine, span.end_line - 1);
    let endCol = Math.max(0, span.end_col - 1);

    if (endLine < startLine || (endLine === startLine && endCol <= startCol)) {
        endLine = startLine;
        endCol = startCol + 1;
    }

    return new vscode.Range(startLine, startCol, endLine, endCol);
}

function updateStatusBarVisibility(_trusted?: boolean) {
    const editor = vscode.window.activeTextEditor;
    if (editor && editor.document.languageId === 'prsm') {
        statusBar.show();
    } else {
        statusBar.hide();
    }
}

export function deactivate() {
    disposeLegacyCompletionProvider();
    void stopPrismLanguageClient();
    diagCollection?.dispose();
    if (statusBar) {
        statusBar.dispose();
    }
}
