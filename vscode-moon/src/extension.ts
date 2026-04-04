import * as vscode from 'vscode';
import * as path from 'path';
import { checkDocument, compileFile, compileDirectory } from './diagnostics';
import { MoonCompletionProvider } from './completion';
import { MoonExplorerProvider } from './sidebar';
import { MoonVisualizer } from './visualizer';
import { MoonGraphView } from './graph-view';
import {
    findGeneratedSpanForSourcePosition,
    findSourceAnchorForGeneratedPosition,
    readGeneratedSourceMap,
    resolveSourceMapSourcePath,
    type MoonGeneratedSourceMapSpan,
} from './generated-source-map';
import { insertLifecycleBlock } from './lifecycle-inserter';
import { MoonNavigationProvider } from './navigation';
import { MoonSymbolProvider } from './symbols';
import { resolveGeneratedCsPath as resolveGeneratedCsPathFromConfig } from './project-config';

let diagCollection: vscode.DiagnosticCollection;
let statusBar: vscode.StatusBarItem;

export function activate(context: vscode.ExtensionContext) {
    const isTrusted = vscode.workspace.isTrusted;

    // Add moonc to VSCode terminal PATH
    const mooncBinDir = path.join(context.extensionPath, 'bin');
    context.environmentVariableCollection.append('PATH', path.delimiter + mooncBinDir);

    // Sidebar: Moon Explorer
    const moonExplorer = new MoonExplorerProvider();
    vscode.window.registerTreeDataProvider('moonExplorer', moonExplorer);
    context.subscriptions.push(
        vscode.commands.registerCommand('moon.refreshExplorer', () => moonExplorer.refresh())
    );

    // Refresh sidebar when editor changes
    vscode.window.onDidChangeActiveTextEditor(e => {
        if (e && e.document.languageId === 'moon') {
            moonExplorer.refresh();
        }
    });

    const symbolProvider = new MoonSymbolProvider();

    // File watcher: refresh sidebar on .mn file changes
    const mnWatcher = vscode.workspace.createFileSystemWatcher('**/*.mn');
    mnWatcher.onDidCreate(() => {
        moonExplorer.refresh();
        symbolProvider.invalidate();
    });
    mnWatcher.onDidDelete(() => {
        moonExplorer.refresh();
        symbolProvider.invalidate();
    });
    mnWatcher.onDidChange(() => {
        moonExplorer.refresh();
        symbolProvider.invalidate();
    });
    context.subscriptions.push(mnWatcher);

    // Visualizer command
    context.subscriptions.push(
        vscode.commands.registerCommand('moon.visualize', () => {
            const editor = vscode.window.activeTextEditor;
            if (editor && editor.document.languageId === 'moon') {
                MoonVisualizer.show(editor.document);
            } else {
                vscode.window.showWarningMessage('No .mn file is open');
            }
        })
    );

    // Lifecycle inserter command
    context.subscriptions.push(
        vscode.commands.registerCommand('moon.insertLifecycle', insertLifecycleBlock)
    );

    // Graph View command
    context.subscriptions.push(
        vscode.commands.registerCommand('moon.graphView', () => {
            MoonGraphView.show(context);
        })
    );

    // Autocomplete — works in both trusted and untrusted
    const completionProvider = new MoonCompletionProvider(context.extensionPath);
    context.subscriptions.push(
        vscode.languages.registerCompletionItemProvider(
            { language: 'moon', scheme: 'file' },
            completionProvider,
            '.', '?', ':', '<' // trigger characters
        )
    );

    const navigationProvider = new MoonNavigationProvider();
    context.subscriptions.push(
        vscode.languages.registerDefinitionProvider(
            { language: 'moon', scheme: 'file' },
            navigationProvider,
        ),
        vscode.languages.registerHoverProvider(
            { language: 'moon', scheme: 'file' },
            navigationProvider,
        ),
        vscode.languages.registerReferenceProvider(
            { language: 'moon', scheme: 'file' },
            navigationProvider,
        ),
        vscode.languages.registerRenameProvider(
            { language: 'moon', scheme: 'file' },
            navigationProvider,
        ),
        vscode.languages.registerDocumentSymbolProvider(
            { language: 'moon', scheme: 'file' },
            symbolProvider,
        ),
        vscode.languages.registerWorkspaceSymbolProvider(symbolProvider),
    );

    // Diagnostic collection (always created, but only used in trusted mode)
    diagCollection = vscode.languages.createDiagnosticCollection('moon');
    context.subscriptions.push(diagCollection);

    // Status bar
    statusBar = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
    statusBar.command = 'workbench.action.problems.focus';
    statusBar.tooltip = 'Moon language status — click to open Problems';
    context.subscriptions.push(statusBar);

    // Show status bar when a .mn file is active
    updateStatusBarVisibility(isTrusted);
    vscode.window.onDidChangeActiveTextEditor(() => updateStatusBarVisibility(isTrusted));

    // Diagnostics + compile — only in trusted workspaces
    if (isTrusted) {
        statusBar.text = '$(check) Moon';

        // Real-time check on typing (debounced 500ms) + sidebar refresh
        let checkTimer: NodeJS.Timeout | undefined;
        context.subscriptions.push(
            vscode.workspace.onDidChangeTextDocument(e => {
                if (e.document.languageId === 'moon') {
                    if (checkTimer) { clearTimeout(checkTimer); }
                    checkTimer = setTimeout(() => {
                        checkDocument(e.document, diagCollection, statusBar);
                        moonExplorer.refresh();
                        symbolProvider.invalidate();
                    }, 500);
                }
            })
        );

        // Also check on save (immediate)
        context.subscriptions.push(
            vscode.workspace.onDidSaveTextDocument(doc => {
                if (doc.languageId === 'moon') {
                    if (checkTimer) { clearTimeout(checkTimer); }
                    checkDocument(doc, diagCollection, statusBar);
                    moonExplorer.refresh();
                    symbolProvider.invalidate();
                }
            })
        );

        // Check on open
        context.subscriptions.push(
            vscode.workspace.onDidOpenTextDocument(doc => {
                if (doc.languageId === 'moon') {
                    checkDocument(doc, diagCollection, statusBar);
                    symbolProvider.invalidate();
                }
            })
        );

        // Clear diagnostics when file is closed
        context.subscriptions.push(
            vscode.workspace.onDidCloseTextDocument(doc => {
                if (doc.languageId === 'moon') {
                    diagCollection.delete(doc.uri);
                }
            })
        );
    } else {
        statusBar.text = '$(shield) Moon (restricted)';
    }

    // Re-activate full features when workspace becomes trusted
    context.subscriptions.push(
        vscode.workspace.onDidGrantWorkspaceTrust(() => {
            statusBar.text = '$(check) Moon';
            // Check all open .mn files
            vscode.workspace.textDocuments.forEach(doc => {
                if (doc.languageId === 'moon') {
                    checkDocument(doc, diagCollection, statusBar);
                }
            });
        })
    );

    // Command: Compile Current File
    context.subscriptions.push(
        vscode.commands.registerCommand('moon.compileFile', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor || editor.document.languageId !== 'moon') {
                vscode.window.showWarningMessage('No .mn file is open');
                return;
            }

            // Save first
            await editor.document.save();
            const filePath = editor.document.uri.fsPath;

            vscode.window.withProgress({
                location: vscode.ProgressLocation.Notification,
                title: 'Moon: Compiling...',
                cancellable: false
            }, async () => {
                const result = await compileFile(filePath);
                if (result.success) {
                    vscode.window.showInformationMessage(`Moon: Compiled successfully`);
                } else {
                    vscode.window.showErrorMessage(`Moon: Compilation failed\n${result.output}`);
                }
                // Refresh diagnostics
                checkDocument(editor.document, diagCollection, statusBar);
            });
        })
    );

    // Command: Compile Workspace
    context.subscriptions.push(
        vscode.commands.registerCommand('moon.compileWorkspace', async () => {
            const folders = vscode.workspace.workspaceFolders;
            if (!folders || folders.length === 0) {
                vscode.window.showWarningMessage('No workspace folder open');
                return;
            }

            const workspacePath = folders[0].uri.fsPath;

            vscode.window.withProgress({
                location: vscode.ProgressLocation.Notification,
                title: 'Moon: Compiling workspace...',
                cancellable: false
            }, async () => {
                const result = await compileDirectory(workspacePath);
                if (result.success) {
                    vscode.window.showInformationMessage(`Moon: Workspace compiled\n${result.output}`);
                } else {
                    vscode.window.showErrorMessage(`Moon: Compilation failed\n${result.output}`);
                }
            });
        })
    );

    // Command: Check Current File
    context.subscriptions.push(
        vscode.commands.registerCommand('moon.checkFile', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor || editor.document.languageId !== 'moon') {
                vscode.window.showWarningMessage('No .mn file is open');
                return;
            }

            await editor.document.save();
            await checkDocument(editor.document, diagCollection, statusBar);
            vscode.window.showInformationMessage('Moon: Check complete');
        })
    );

    // Command: Show Generated C# (split right)
    context.subscriptions.push(
        vscode.commands.registerCommand('moon.showGenerated', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor || editor.document.languageId !== 'moon') {
                vscode.window.showWarningMessage('No .mn file is open');
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
        vscode.commands.registerCommand('moon.showSourceFromGenerated', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor || editor.document.uri.scheme !== 'file' || path.extname(editor.document.uri.fsPath).toLowerCase() !== '.cs') {
                vscode.window.showWarningMessage('No generated .cs file is open');
                return;
            }

            const generatedPath = editor.document.uri.fsPath;
            const sourceMap = readGeneratedSourceMap(generatedPath);
            if (!sourceMap) {
                vscode.window.showWarningMessage('Moon source map sidecar not found for this generated .cs file. Compile first.');
                return;
            }

            const sourceAnchor = findSourceAnchorForGeneratedPosition(
                sourceMap,
                editor.selection.active.line + 1,
                editor.selection.active.character + 1,
            );
            if (!sourceAnchor) {
                vscode.window.showWarningMessage('No Moon source anchor was found for the current generated C# position.');
                return;
            }

            const sourcePath = resolveSourceMapSourcePath(generatedPath, sourceMap, getWorkspaceRoots());
            if (!sourcePath) {
                vscode.window.showWarningMessage('Moon source path could not be resolved from the generated source map.');
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

    // Check all open .mn files on activation
    vscode.workspace.textDocuments.forEach(doc => {
        if (doc.languageId === 'moon') {
            checkDocument(doc, diagCollection, statusBar);
        }
    });
}

/**
 * Resolve the generated .cs path for a .mn file.
 * Reads output_dir from .mnproject and falls back to common generated-code locations.
 */
function resolveGeneratedCsPath(mnPath: string): string | null {
    return resolveGeneratedCsPathFromConfig(mnPath, getWorkspaceRoots());
}

function getWorkspaceRoots(): string[] {
    return (vscode.workspace.workspaceFolders || []).map(folder => folder.uri.fsPath);
}

function sourceMapSpanToRange(span: MoonGeneratedSourceMapSpan | null | undefined): vscode.Range | undefined {
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
    if (editor && editor.document.languageId === 'moon') {
        statusBar.show();
    } else {
        statusBar.hide();
    }
}

export function deactivate() {
    if (diagCollection) {
        diagCollection.dispose();
    }
    if (statusBar) {
        statusBar.dispose();
    }
}
