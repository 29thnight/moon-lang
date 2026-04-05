export interface PrismDiagnosticRangeInput {
    line: number;
    col: number;
    endLine?: number;
    endCol?: number;
}

export interface NormalizedRange {
    startLine: number;
    startCol: number;
    endLine: number;
    endCol: number;
}

export function normalizeDiagnosticRange(
    entry: PrismDiagnosticRangeInput,
    lineLengths: number[],
): NormalizedRange {
    const safeLengths = lineLengths.length > 0 ? lineLengths : [0];
    const maxLine = safeLengths.length - 1;

    const startLine = clamp(entry.line - 1, 0, maxLine);
    const startCol = clamp(entry.col - 1, 0, safeLengths[startLine]);

    let endLine = clamp((entry.endLine ?? entry.line) - 1, 0, maxLine);
    let endCol = clamp((entry.endCol ?? entry.col) - 1, 0, safeLengths[endLine]);

    if (endLine < startLine || (endLine === startLine && endCol <= startCol)) {
        endLine = startLine;
        endCol = Math.min(safeLengths[startLine], startCol + 1);
        if (endCol <= startCol && safeLengths[startLine] === 0) {
            endCol = startCol;
        }
    }

    return { startLine, startCol, endLine, endCol };
}

function clamp(value: number, min: number, max: number): number {
    return Math.min(Math.max(value, min), max);
}