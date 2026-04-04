import { execFile } from 'child_process';
import * as vscode from 'vscode';
import {
    getBundledCompilerCandidates,
    getProjectCompilerPath,
    getWorkspaceDevCompilerCandidates,
    resolveCompilerPathFromContext,
} from './compiler-resolver';
import { findMoonProjectRoot } from './project-config';

export interface MoonSourceLocation {
    file: string;
    line: number;
    col: number;
    end_line?: number;
    end_col?: number;
}

export interface MoonDiagnosticEntry extends MoonSourceLocation {
    code: string;
    severity: 'error' | 'warning';
    message: string;
}

export interface MoonDiagnosticResult {
    files: number;
    errors: number;
    warnings: number;
    diagnostics: MoonDiagnosticEntry[];
}

export interface MoonIndexedSymbol extends MoonSourceLocation {
    name: string;
    qualified_name: string;
    container_name?: string | null;
    kind: string;
    signature: string;
}

export interface MoonIndexedReference extends MoonSourceLocation {
    name: string;
    container_name?: string | null;
    kind: string;
    target_qualified_name?: string | null;
    resolved_symbol?: MoonIndexedSymbol | null;
}

export interface MoonIndexResult {
    symbol_at?: MoonIndexedSymbol | null;
    reference_at?: MoonIndexedReference | null;
}

interface MoonIndexQueryResult extends MoonIndexResult {
    matches: MoonIndexedSymbol[];
}

export interface MoonDefinitionEntry extends MoonSourceLocation {
    id: number;
    name: string;
    qualified_name: string;
    kind: string;
    type: string;
    mutable: boolean;
}

interface MoonDefinitionResult {
    definition?: MoonDefinitionEntry | null;
}

export interface MoonReferenceEntry extends MoonSourceLocation {
    name: string;
    kind: string;
    resolved_definition_id?: number | null;
    candidate_qualified_name?: string | null;
}

export interface MoonReferencesResult {
    definition?: MoonDefinitionEntry | null;
    references: MoonReferenceEntry[];
}

let cachedCompilerPath: string | null = null;

function getCompilerPath(): string {
    if (cachedCompilerPath) {
        return cachedCompilerPath;
    }

    const config = vscode.workspace.getConfiguration('moon');
    const workspaceRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
    const extensionPath = vscode.extensions.getExtension('moon-lang.moon-lang')?.extensionPath;

    cachedCompilerPath = resolveCompilerPathFromContext({
        userOverride: config.get<string>('compilerPath', ''),
        projectCompilerPath: getProjectCompilerPath(workspaceRoot),
        bundledCandidates: getBundledCompilerCandidates(extensionPath),
        devCandidates: getWorkspaceDevCompilerCandidates(workspaceRoot),
        fallback: 'moonc',
    });

    return cachedCompilerPath;
}

export function clearCompilerPathCache() {
    cachedCompilerPath = null;
}

function getQueryRoot(filePath: string): string | null {
    const projectRoot = findMoonProjectRoot(filePath);
    if (projectRoot) {
        return projectRoot;
    }

    const workspaceFolder = vscode.workspace.getWorkspaceFolder(vscode.Uri.file(filePath));
    return workspaceFolder?.uri.fsPath ?? null;
}

function getWorkspaceQueryRoot(resourcePath?: string): string | null {
    if (resourcePath) {
        return getQueryRoot(resourcePath);
    }

    const workspaceRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
    if (!workspaceRoot) {
        return null;
    }

    return findMoonProjectRoot(workspaceRoot) ?? workspaceRoot;
}

function execMoon(args: string[], timeout: number, cwd?: string): Promise<{ stdout: string; stderr: string; success: boolean }> {
    return new Promise((resolve) => {
        execFile(
            getCompilerPath(),
            args,
            { cwd, timeout, windowsHide: true },
            (error, stdout, stderr) => {
                resolve({
                    stdout: stdout.trim(),
                    stderr: stderr.trim(),
                    success: !error,
                });
            },
        );
    });
}

async function execMoonJson<T>(args: string[], timeout: number, cwd?: string): Promise<T | null> {
    const result = await execMoon(args, timeout, cwd);
    if (!result.stdout) {
        return null;
    }

    try {
        return JSON.parse(result.stdout) as T;
    } catch {
        return null;
    }
}

export async function runMoonCheck(filePath: string): Promise<MoonDiagnosticResult> {
    const result = await execMoonJson<MoonDiagnosticResult>(['check', filePath, '--json'], 30000);
    return result ?? { files: 0, errors: 0, warnings: 0, diagnostics: [] };
}

export async function runMoonCompile(targetPath: string, timeout: number): Promise<{ success: boolean; output: string }> {
    const result = await execMoon(['compile', targetPath], timeout);
    return {
        success: result.success,
        output: result.success ? result.stdout : result.stderr || result.stdout,
    };
}

export async function runMoonDefinitionForPosition(
    filePath: string,
    line: number,
    col: number,
): Promise<MoonDefinitionEntry | null> {
    const queryRoot = getQueryRoot(filePath);
    if (!queryRoot) {
        return null;
    }

    const result = await execMoonJson<MoonDefinitionResult>(
        ['definition', queryRoot, '--json', '--file', filePath, '--line', String(line), '--col', String(col)],
        30000,
        queryRoot,
    );

    return result?.definition ?? null;
}

export async function runMoonIndexForPosition(
    filePath: string,
    line: number,
    col: number,
): Promise<MoonIndexResult | null> {
    const queryRoot = getQueryRoot(filePath);
    if (!queryRoot) {
        return null;
    }

    return execMoonJson<MoonIndexResult>(
        ['index', queryRoot, '--json', '--file', filePath, '--line', String(line), '--col', String(col)],
        30000,
        queryRoot,
    );
}

export async function runMoonReferencesForPosition(
    filePath: string,
    line: number,
    col: number,
): Promise<MoonReferencesResult | null> {
    const queryRoot = getQueryRoot(filePath);
    if (!queryRoot) {
        return null;
    }

    return execMoonJson<MoonReferencesResult>(
        ['references', queryRoot, '--json', '--file', filePath, '--line', String(line), '--col', String(col)],
        30000,
        queryRoot,
    );
}

export async function runMoonProjectSymbols(resourcePath?: string): Promise<MoonIndexedSymbol[]> {
    const queryRoot = getWorkspaceQueryRoot(resourcePath);
    if (!queryRoot) {
        return [];
    }

    const result = await execMoonJson<MoonIndexQueryResult>(
        ['index', queryRoot, '--json'],
        30000,
        queryRoot,
    );

    return result?.matches ?? [];
}