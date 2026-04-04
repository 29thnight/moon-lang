import * as vscode from 'vscode';
import * as fs from 'fs';
import * as path from 'path';
import { normalizeDiagnosticRange } from './diagnostic-range';
import { clearCompilerPathCache, MoonDiagnosticResult, runMoonCheck, runMoonCompile } from './moonc-cli';

interface MoonDiagnosticEntry {
    code: string;
    severity: 'error' | 'warning';
    message: string;
    file: string;
    line: number;
    col: number;
    end_line?: number;
    end_col?: number;
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
    const lineLengths = Array.from({ length: document.lineCount }, (_, index) => document.lineAt(index).text.length);

    for (const entry of result.diagnostics) {
        if (entry.severity === 'warning' && !showWarnings) {
            continue;
        }

        const normalizedRange = normalizeDiagnosticRange(
            {
                line: entry.line,
                col: entry.col,
                endLine: entry.end_line,
                endCol: entry.end_col,
            },
            lineLengths,
        );
        const range = new vscode.Range(
            normalizedRange.startLine,
            normalizedRange.startCol,
            normalizedRange.endLine,
            normalizedRange.endCol,
        );
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
    return runMoonCompile(filePath, 60000);
}

export function compileDirectory(dirPath: string): Promise<{ success: boolean; output: string }> {
    return runMoonCompile(dirPath, 120000);
}
