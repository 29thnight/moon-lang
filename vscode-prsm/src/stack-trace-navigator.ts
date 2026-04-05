/**
 * Stack trace navigator for PrSM source-map workflow.
 *
 * Parses Unity / .NET stack trace lines that reference generated .cs files,
 * looks up the corresponding .prsmmap.json sidecar, and opens the original
 * .prsm source file at the remapped position.
 *
 * Supported stack-trace formats
 * ─────────────────────────────
 *   Unity:  (at Assets/Generated/Foo.cs:42)
 *   .NET:   in C:\path\to\Foo.cs:line 42
 *   plain:  Foo.cs:42
 */

import * as fs from 'fs';
import * as path from 'path';
import * as vscode from 'vscode';
import {
    readGeneratedSourceMap,
    findSourceAnchorForGeneratedPosition,
    resolveSourceMapSourcePath,
} from './generated-source-map';

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

export interface StackFrame {
    /** Absolute or relative path of the .cs file found in the stack trace. */
    csPath: string;
    /** 1-based line number. */
    lineNumber: number;
    /** Raw text of the stack-trace line, for display. */
    rawLine: string;
}

export interface ResolvedFrame {
    frame: StackFrame;
    /** Absolute path of the .prsm source file. */
    prsmPath: string;
    /** 1-based line number inside the .prsm file. */
    prsmLine: number;
    /** 1-based column inside the .prsm file. */
    prsmCol: number;
}

// ─────────────────────────────────────────────────────────────────────────────
// Patterns
// ─────────────────────────────────────────────────────────────────────────────

// (at Assets/Generated/Foo.cs:42)
const UNITY_PATTERN = /\(at\s+(.+\.cs):(\d+)\)/gi;
// in C:\path\Foo.cs:line 42   OR   in /path/Foo.cs:line 42
const DOTNET_PATTERN = /\bin\s+(.+\.cs):line\s+(\d+)/gi;
// bare: something/Foo.cs:42 or Foo.cs:42  (fallback, anchored at word boundary)
const BARE_PATTERN = /\b([A-Za-z0-9_./\\-]+\.cs):(\d+)/gi;

// ─────────────────────────────────────────────────────────────────────────────
// Parsing
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Extract all `.cs` file references from arbitrary multi-line text.
 * Returns de-duplicated frames (first occurrence wins for duplicates).
 */
export function parseStackTraceText(text: string): StackFrame[] {
    const results: StackFrame[] = [];
    const seen = new Set<string>();

    function addMatch(rawLine: string, csPath: string, lineStr: string): void {
        const lineNumber = parseInt(lineStr, 10);
        if (isNaN(lineNumber) || lineNumber < 1) {
            return;
        }
        const key = `${csPath}:${lineNumber}`;
        if (seen.has(key)) {
            return;
        }
        seen.add(key);
        results.push({ csPath, lineNumber, rawLine: rawLine.trim() });
    }

    for (const rawLine of text.split('\n')) {
        let matched = false;

        for (const m of rawLine.matchAll(UNITY_PATTERN)) {
            addMatch(rawLine, m[1], m[2]);
            matched = true;
        }
        for (const m of rawLine.matchAll(DOTNET_PATTERN)) {
            addMatch(rawLine, m[1], m[2]);
            matched = true;
        }
        if (!matched) {
            for (const m of rawLine.matchAll(BARE_PATTERN)) {
                addMatch(rawLine, m[1], m[2]);
            }
        }
    }

    return results;
}

// ─────────────────────────────────────────────────────────────────────────────
// Resolution
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Given an absolute .cs path and a 1-based line number, look up the
 * .prsmmap.json sidecar and return the corresponding .prsm location.
 * Returns null if no source map or no matching anchor is found.
 */
export function resolveFrameToPrsm(
    csAbsPath: string,
    lineNumber: number,
    workspaceRoots: string[],
): ResolvedFrame | null {
    const sourceMap = readGeneratedSourceMap(csAbsPath);
    if (!sourceMap) {
        return null;
    }

    const anchor = findSourceAnchorForGeneratedPosition(sourceMap, lineNumber, 1);
    if (!anchor || !anchor.source_span) {
        return null;
    }

    const prsmPath = resolveSourceMapSourcePath(csAbsPath, sourceMap, workspaceRoots);
    if (!prsmPath) {
        return null;
    }

    return {
        frame: { csPath: csAbsPath, lineNumber, rawLine: '' },
        prsmPath,
        prsmLine: anchor.source_span.line,
        prsmCol: anchor.source_span.col,
    };
}

/**
 * Given a StackFrame whose `csPath` may be relative, try to resolve it to an
 * absolute path by searching workspace roots and nearby generated-output directories.
 */
export function resolveFrameCsPath(
    frame: StackFrame,
    workspaceRoots: string[],
): string | null {
    const { csPath } = frame;

    if (path.isAbsolute(csPath) && fs.existsSync(csPath)) {
        return csPath;
    }

    const candidates: string[] = [];
    for (const root of workspaceRoots) {
        candidates.push(path.join(root, csPath));
        // Common Unity generated-output patterns
        candidates.push(path.join(root, 'Assets', 'Generated', path.basename(csPath)));
        candidates.push(path.join(root, 'Assets', path.basename(csPath)));
        candidates.push(path.join(root, 'Generated', path.basename(csPath)));
    }

    for (const candidate of candidates) {
        if (fs.existsSync(candidate)) {
            return candidate;
        }
    }

    return null;
}

// ─────────────────────────────────────────────────────────────────────────────
// VS Code command handler
// ─────────────────────────────────────────────────────────────────────────────

/**
 * `prsm.openFromStackTrace` command implementation.
 *
 * Uses the active editor selection when text is selected; otherwise prompts
 * the user to paste a stack-trace snippet. Finds all .cs references, resolves
 * them through source maps, and opens the first successful .prsm location.
 * If multiple frames resolve successfully a QuickPick is shown.
 */
export async function openFromStackTraceCommand(workspaceRoots: string[]): Promise<void> {
    let input: string | undefined;

    const editor = vscode.window.activeTextEditor;
    if (editor && !editor.selection.isEmpty) {
        input = editor.document.getText(editor.selection);
    }

    if (!input || input.trim() === '') {
        input = await vscode.window.showInputBox({
            prompt: 'Paste a Unity / .NET stack trace (one or more lines)',
            placeHolder: '(at Assets/Generated/PlayerController.cs:42)',
            ignoreFocusOut: true,
        });
    }

    if (!input) {
        return;
    }

    const frames = parseStackTraceText(input);
    if (frames.length === 0) {
        vscode.window.showWarningMessage('No .cs file references found in the pasted text.');
        return;
    }

    const resolved: ResolvedFrame[] = [];
    for (const frame of frames) {
        const absCs = resolveFrameCsPath(frame, workspaceRoots);
        if (!absCs) {
            continue;
        }
        const result = resolveFrameToPrsm(absCs, frame.lineNumber, workspaceRoots);
        if (result) {
            result.frame.rawLine = frame.rawLine;
            resolved.push(result);
        }
    }

    if (resolved.length === 0) {
        vscode.window.showWarningMessage(
            'No PrSM source map found for the referenced .cs file(s). Compile the workspace first.',
        );
        return;
    }

    let target: ResolvedFrame;

    if (resolved.length === 1) {
        target = resolved[0];
    } else {
        const items = resolved.map(r => ({
            label: `$(go-to-file) ${path.basename(r.prsmPath)}:${r.prsmLine}`,
            description: r.frame.rawLine,
            detail: r.prsmPath,
            resolved: r,
        }));

        const picked = await vscode.window.showQuickPick(items, {
            placeHolder: 'Multiple PrSM locations found — select one to open',
        });

        if (!picked) {
            return;
        }
        target = picked.resolved;
    }

    const uri = vscode.Uri.file(target.prsmPath);
    const doc = await vscode.workspace.openTextDocument(uri);
    const line = Math.max(0, target.prsmLine - 1);
    const col = Math.max(0, target.prsmCol - 1);
    const range = new vscode.Range(line, col, line, col);

    await vscode.window.showTextDocument(doc, {
        preview: false,
        selection: range,
    });
}
