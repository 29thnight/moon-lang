import test from 'node:test';
import assert from 'node:assert/strict';
import { normalizeDiagnosticRange } from '../diagnostic-range';

test('normalizeDiagnosticRange uses explicit end coordinates', () => {
    const range = normalizeDiagnosticRange(
        { line: 2, col: 3, endLine: 2, endCol: 8 },
        [5, 12, 4],
    );

    assert.deepEqual(range, {
        startLine: 1,
        startCol: 2,
        endLine: 1,
        endCol: 7,
    });
});

test('normalizeDiagnosticRange widens zero-width spans to one character when possible', () => {
    const range = normalizeDiagnosticRange(
        { line: 1, col: 2, endLine: 1, endCol: 2 },
        [6],
    );

    assert.deepEqual(range, {
        startLine: 0,
        startCol: 1,
        endLine: 0,
        endCol: 2,
    });
});

test('normalizeDiagnosticRange clamps coordinates to document bounds', () => {
    const range = normalizeDiagnosticRange(
        { line: 10, col: 20, endLine: 11, endCol: 30 },
        [3, 4],
    );

    assert.deepEqual(range, {
        startLine: 1,
        startCol: 4,
        endLine: 1,
        endCol: 4,
    });
});