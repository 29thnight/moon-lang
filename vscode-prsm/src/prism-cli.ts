import { execFile } from 'child_process';
import * as vscode from 'vscode';
import {
    getBundledCompilerCandidates,
    getProjectCompilerPath,
    getWorkspaceDevCompilerCandidates,
    resolveCompilerPathFromContext,
} from './compiler-resolver';
import { findPrismProjectRoot } from './project-config';

export interface PrismSourceLocation {
    file: string;
    line: number;
    col: number;
    end_line?: number;
    end_col?: number;
}

export interface PrismDiagnosticEntry extends PrismSourceLocation {
    code: string;
    severity: 'error' | 'warning';
    message: string;
}

export interface PrismDiagnosticResult {
    files: number;
    errors: number;
    warnings: number;
    diagnostics: PrismDiagnosticEntry[];
}

export interface PrismIndexedSymbol extends PrismSourceLocation {
    name: string;
    qualified_name: string;
    container_name?: string | null;
    kind: string;
    signature: string;
}

export interface PrismIndexedReference extends PrismSourceLocation {
    name: string;
    container_name?: string | null;
    kind: string;
    target_qualified_name?: string | null;
    resolved_symbol?: PrismIndexedSymbol | null;
}

export interface PrismIndexResult {
    symbol_at?: PrismIndexedSymbol | null;
    reference_at?: PrismIndexedReference | null;
}

interface PrSMIndexQueryResult extends PrismIndexResult {
    matches: PrismIndexedSymbol[];
}

export interface PrismDefinitionEntry extends PrismSourceLocation {
    id: number;
    name: string;
    qualified_name: string;
    kind: string;
    type: string;
    mutable: boolean;
}

interface PrSMDefinitionResult {
    definition?: PrismDefinitionEntry | null;
}

export interface PrSMReferenceEntry extends PrismSourceLocation {
    name: string;
    kind: string;
    resolved_definition_id?: number | null;
    candidate_qualified_name?: string | null;
}

export interface PrismReferencesResult {
    definition?: PrismDefinitionEntry | null;
    references: PrSMReferenceEntry[];
}

let cachedCompilerPath: string | null = null;

function getCompilerPath(): string {
    if (cachedCompilerPath) {
        return cachedCompilerPath;
    }

    const config = vscode.workspace.getConfiguration('prsm');
    const workspaceRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
    const extensionPath = vscode.extensions.getExtension('parkyoungung.prsm-lang')?.extensionPath;

    cachedCompilerPath = resolveCompilerPathFromContext({
        userOverride: config.get<string>('compilerPath', ''),
        projectCompilerPath: getProjectCompilerPath(workspaceRoot),
        bundledCandidates: getBundledCompilerCandidates(extensionPath),
        devCandidates: getWorkspaceDevCompilerCandidates(workspaceRoot),
        fallback: 'prism',
    });

    return cachedCompilerPath;
}

export function clearCompilerPathCache() {
    cachedCompilerPath = null;
}

function getQueryRoot(filePath: string): string | null {
    const projectRoot = findPrismProjectRoot(filePath);
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

    return findPrismProjectRoot(workspaceRoot) ?? workspaceRoot;
}

function execPrSM(args: string[], timeout: number, cwd?: string): Promise<{ stdout: string; stderr: string; success: boolean }> {
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

async function execPrSMJson<T>(args: string[], timeout: number, cwd?: string): Promise<T | null> {
    const result = await execPrSM(args, timeout, cwd);
    if (!result.stdout) {
        return null;
    }

    try {
        return JSON.parse(result.stdout) as T;
    } catch {
        return null;
    }
}

export async function runPrSMCheck(filePath: string): Promise<PrismDiagnosticResult> {
    const result = await execPrSMJson<PrismDiagnosticResult>(['check', filePath, '--json'], 30000);
    return result ?? { files: 0, errors: 0, warnings: 0, diagnostics: [] };
}

export async function runPrSMCompile(targetPath: string, timeout: number): Promise<{ success: boolean; output: string }> {
    const result = await execPrSM(['compile', targetPath], timeout);
    return {
        success: result.success,
        output: result.success ? result.stdout : result.stderr || result.stdout,
    };
}

export async function runPrSMDefinitionForPosition(
    filePath: string,
    line: number,
    col: number,
): Promise<PrismDefinitionEntry | null> {
    const queryRoot = getQueryRoot(filePath);
    if (!queryRoot) {
        return null;
    }

    const result = await execPrSMJson<PrSMDefinitionResult>(
        ['definition', queryRoot, '--json', '--file', filePath, '--line', String(line), '--col', String(col)],
        30000,
        queryRoot,
    );

    return result?.definition ?? null;
}

export async function runPrSMIndexForPosition(
    filePath: string,
    line: number,
    col: number,
): Promise<PrismIndexResult | null> {
    const queryRoot = getQueryRoot(filePath);
    if (!queryRoot) {
        return null;
    }

    return execPrSMJson<PrismIndexResult>(
        ['index', queryRoot, '--json', '--file', filePath, '--line', String(line), '--col', String(col)],
        30000,
        queryRoot,
    );
}

export async function runPrSMReferencesForPosition(
    filePath: string,
    line: number,
    col: number,
): Promise<PrismReferencesResult | null> {
    const queryRoot = getQueryRoot(filePath);
    if (!queryRoot) {
        return null;
    }

    return execPrSMJson<PrismReferencesResult>(
        ['references', queryRoot, '--json', '--file', filePath, '--line', String(line), '--col', String(col)],
        30000,
        queryRoot,
    );
}

export async function runPrSMProjectSymbols(resourcePath?: string): Promise<PrismIndexedSymbol[]> {
    const queryRoot = getWorkspaceQueryRoot(resourcePath);
    if (!queryRoot) {
        return [];
    }

    const result = await execPrSMJson<PrSMIndexQueryResult>(
        ['index', queryRoot, '--json'],
        30000,
        queryRoot,
    );

    return result?.matches ?? [];
}