import * as vscode from 'vscode';
import { exec } from 'child_process';
import * as fs from 'fs';
import * as path from 'path';
import {
    getBundledCompilerCandidates,
    getProjectCompilerPath,
    getWorkspaceDevCompilerCandidates,
    resolveCompilerPathFromContext,
} from './compiler-resolver';

interface MoonDiagnosticResult {
    files: number;
    errors: number;
    warnings: number;
    diagnostics: MoonDiagnosticEntry[];
}

interface MoonDiagnosticEntry {
    code: string;
    severity: 'error' | 'warning';
    message: string;
    file: string;
    line: number;
    col: number;
}

let _cachedCompilerPath: string | null = null;

function getCompilerPath(): string {
    if (_cachedCompilerPath) {
        return _cachedCompilerPath;
    }

    const config = vscode.workspace.getConfiguration('moon');
    const workspaceRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
    const extensionPath = vscode.extensions.getExtension('moon-lang.moon-lang')?.extensionPath;

    _cachedCompilerPath = resolveCompilerPathFromContext({
        userOverride: config.get<string>('compilerPath', ''),
        projectCompilerPath: getProjectCompilerPath(workspaceRoot),
        bundledCandidates: getBundledCompilerCandidates(extensionPath),
        devCandidates: getWorkspaceDevCompilerCandidates(workspaceRoot),
        fallback: 'moonc',
    });

    return _cachedCompilerPath;
}

export function clearCompilerPathCache() {
    _cachedCompilerPath = null;
}

export function runMoonCheck(filePath: string): Promise<MoonDiagnosticResult> {
    return new Promise((resolve) => {
        const compiler = getCompilerPath();
        const command = `"${compiler}" check "${filePath}" --json`;

        exec(command, { timeout: 30000 }, (_error, stdout) => {
            try {
                const output = stdout.trim();
                if (output) {
                    resolve(JSON.parse(output) as MoonDiagnosticResult);
                    return;
                }
            } catch {
                // fall through to empty result
            }

            resolve({ files: 0, errors: 0, warnings: 0, diagnostics: [] });
        });
    });
}

export async function checkDocument(
    document: vscode.TextDocument,
    diagCollection: vscode.DiagnosticCollection,
    statusBar: vscode.StatusBarItem,
): Promise<void> {
    const config = vscode.workspace.getConfiguration('moon');
    if (!config.get<boolean>('checkOnSave', true)) {
        return;
    }

    let checkPath = document.uri.fsPath;
    if (document.isDirty) {
        const os = require('os');
        const tmpDir = path.join(os.tmpdir(), 'moon-check');
        if (!fs.existsSync(tmpDir)) {
            fs.mkdirSync(tmpDir, { recursive: true });
        }
        checkPath = path.join(tmpDir, path.basename(document.uri.fsPath));
        fs.writeFileSync(checkPath, document.getText());
    }

    const result = await runMoonCheck(checkPath);
    const showWarnings = config.get<boolean>('showWarnings', true);
    const diagnostics: vscode.Diagnostic[] = [];

    for (const entry of result.diagnostics) {
        if (entry.severity === 'warning' && !showWarnings) {
            continue;
        }

        const line = Math.max(0, entry.line - 1);
        const col = Math.max(0, entry.col - 1);
        const range = new vscode.Range(line, col, line, col + 10);
        const severity = entry.severity === 'error'
            ? vscode.DiagnosticSeverity.Error
            : vscode.DiagnosticSeverity.Warning;

        const diagnostic = new vscode.Diagnostic(range, entry.message, severity);
        diagnostic.code = entry.code;
        diagnostic.source = 'moonc';
        diagnostics.push(diagnostic);
    }

    diagCollection.set(document.uri, diagnostics);

    if (result.errors > 0) {
        statusBar.text = `$(error) Moon: ${result.errors} error(s)`;
        statusBar.backgroundColor = new vscode.ThemeColor('statusBarItem.errorBackground');
    } else if (result.warnings > 0) {
        statusBar.text = `$(warning) Moon: ${result.warnings} warning(s)`;
        statusBar.backgroundColor = new vscode.ThemeColor('statusBarItem.warningBackground');
    } else {
        statusBar.text = '$(check) Moon';
        statusBar.backgroundColor = undefined;
    }
    statusBar.show();
}

export function compileFile(filePath: string): Promise<{ success: boolean; output: string }> {
    return new Promise((resolve) => {
        const compiler = getCompilerPath();
        const command = `"${compiler}" compile "${filePath}"`;

        exec(command, { timeout: 60000 }, (error, stdout, stderr) => {
            if (error) {
                resolve({ success: false, output: stderr || error.message });
            } else {
                resolve({ success: true, output: stdout });
            }
        });
    });
}

export function compileDirectory(dirPath: string): Promise<{ success: boolean; output: string }> {
    return new Promise((resolve) => {
        const compiler = getCompilerPath();
        const command = `"${compiler}" compile "${dirPath}"`;

        exec(command, { timeout: 120000 }, (error, stdout, stderr) => {
            if (error) {
                resolve({ success: false, output: stderr || error.message });
            } else {
                resolve({ success: true, output: stdout });
            }
        });
    });
}
