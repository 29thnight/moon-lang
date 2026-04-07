import test from 'node:test';
import assert from 'node:assert/strict';
import * as fs from 'fs';
import * as path from 'path';

// Pure consistency check: every keyword introduced in PrSM Language 4
// and Language 5 must be reachable from `prsm.tmLanguage.json` so that
// VS Code applies the configured token color. The grammar is loaded as
// raw text and we look for word-boundary `\bKW\b` substrings, which is
// the same shape used by the rules themselves — keeping the test cheap
// and free of a TextMate tokenizer dependency.

const grammarPath = path.resolve(__dirname, '..', '..', 'syntaxes', 'prsm.tmLanguage.json');
const themePath = path.resolve(__dirname, '..', '..', 'themes', 'islands-dark-prism.json');
const packagePath = path.resolve(__dirname, '..', '..', 'package.json');

function loadFile(p: string): string {
    return fs.readFileSync(p, 'utf8');
}

const grammarText = loadFile(grammarPath);
const themeText = loadFile(themePath);
const packageText = loadFile(packagePath);

// Helper: assert a `\bKW\b` token appears at least once anywhere in
// the grammar JSON. The grammar uses double-escaped backslashes
// (`\\b...\\b`) so we test against that literal form.
function grammarMentions(keyword: string): boolean {
    return grammarText.includes(`\\b${keyword}\\b`)
        || grammarText.includes(`\\b${keyword}|`)
        || grammarText.includes(`|${keyword}\\b`)
        || grammarText.includes(`|${keyword}|`)
        || grammarText.includes(`(${keyword}|`)
        || grammarText.includes(`|${keyword})`);
}

// === v4 keywords =============================================================
const v4Keywords: ReadonlyArray<string> = [
    // Type-system & inheritance
    'abstract',
    'sealed',
    'open',
    'where',
    'operator',
    'struct',
    'typealias',
    'interface',
    'attribute',
    'extend',
    // Async / coroutines
    'async',
    'await',
    // State machine / command / bind / event / use
    'state',
    'machine',
    'command',
    'bind',
    'to',
    'event',
    'use',
    // Pool / singleton (already in v3 spec but exercised by v4)
    'pool',
    'singleton',
    // Storage modifiers
    'static',
    'const',
    'fixed',
];

// === v5 keywords =============================================================
const v5Keywords: ReadonlyArray<string> = [
    // Sprint 1
    'yield',
    // Sprint 2
    'nameof',
    'ref',
    'out',
    'vararg',
    // Sprint 4: combinator pattern keywords
    'not',
    'and',
    'unmanaged',
    'notnull',
    // Sprint 5
    'partial',
    // Sprint 6
    'with',
    'stackalloc',
    'throw',
];

test('grammar covers every v4 keyword', () => {
    for (const kw of v4Keywords) {
        assert.ok(
            grammarMentions(kw),
            `v4 keyword '${kw}' is missing from prsm.tmLanguage.json — \\b${kw}\\b not found`,
        );
    }
});

test('grammar covers every v5 keyword', () => {
    for (const kw of v5Keywords) {
        assert.ok(
            grammarMentions(kw),
            `v5 keyword '${kw}' is missing from prsm.tmLanguage.json — \\b${kw}\\b not found`,
        );
    }
});

// === Preprocessor directives (v5 Sprint 1) ==================================

test('grammar declares the v5 preprocessor directive scope', () => {
    assert.ok(
        grammarText.includes('keyword.control.directive.prsm'),
        '#if/#elif/#else/#endif must be tagged with keyword.control.directive.prsm',
    );
    assert.ok(
        grammarText.includes('constant.other.preprocessor.prsm'),
        'preprocessor symbols must be tagged with constant.other.preprocessor.prsm',
    );
});

// === Theme & default token color customizations =============================

test('theme defines a color for the preprocessor directive scope', () => {
    assert.ok(
        themeText.includes('keyword.control.directive'),
        'theme is missing a tokenColors entry for keyword.control.directive',
    );
    assert.ok(
        themeText.includes('constant.other.preprocessor'),
        'theme is missing a tokenColors entry for constant.other.preprocessor',
    );
});

test('package.json default token color customizations cover the preprocessor scopes', () => {
    assert.ok(
        packageText.includes('keyword.control.directive.prsm'),
        'package.json configurationDefaults must color keyword.control.directive.prsm',
    );
    assert.ok(
        packageText.includes('constant.other.preprocessor.prsm'),
        'package.json configurationDefaults must color constant.other.preprocessor.prsm',
    );
});

// === Annotation targets (v5 Sprint 1) =======================================

test('grammar annotation pattern handles v5 attribute targets', () => {
    // The annotations rule matches `@id`, so @field/@property/@return/@type/@param
    // and @burst all flow through the same scope. We just assert the pattern
    // is present and tagged with the decorator scope.
    assert.match(
        grammarText,
        /@\)\(\[a-zA-Z_\]\[a-zA-Z0-9_\]\*\)/,
        'annotation pattern is missing from prsm.tmLanguage.json',
    );
    assert.ok(
        grammarText.includes('entity.name.function.decorator.prsm'),
        'annotation captures must be tagged with entity.name.function.decorator.prsm',
    );
});

// === Composite forms (declaration headers) ==================================

test('grammar recognizes v4/v5 composite declaration headers', () => {
    // `partial component`, `partial class`, `ref struct`, `state machine`,
    // `data class` and `abstract|sealed|open|partial class` should all be
    // captured as multi-token rules so the editor can colorize the
    // modifier and the type name independently.
    const cases = [
        'partial)\\\\s+(component',
        'abstract|sealed|open|partial)\\\\s+(class',
        'ref)?\\\\s*(struct',
        'state)\\\\s+(machine',
        'data)\\\\s+(class',
    ];
    for (const expected of cases) {
        assert.ok(
            grammarText.includes(expected),
            `composite header pattern '${expected}' missing from grammar`,
        );
    }
});

// === static + storage class composite forms (v4 §3) =========================

test('grammar recognizes static + func / static + val|var|const|fixed forms', () => {
    // `static func name`, `static val name`, `static const name`, etc.
    // should all colorize every keyword in the prefix, not just the
    // first one. The composite rules live in function-declaration
    // (for func) and field-declaration (for storage classes).
    assert.ok(
        grammarText.includes('static)\\\\s+(func'),
        'static + func composite pattern missing — `static func name` would only colorize one token',
    );
    assert.ok(
        grammarText.includes('static)\\\\s+(val|var|const|fixed'),
        'static + storage class composite pattern missing — `static const NAME` would only colorize one token',
    );
});

test('grammar declares static as a top-level keyword', () => {
    // The composite rules above handle `static <something>` forms,
    // but `static` on its own (e.g. when typing midway) should still
    // be tagged as a keyword by the all-keywords fallback.
    assert.ok(
        grammarText.includes('public|private|protected|static'),
        'static must appear in the all-keywords visibility/storage modifier rule',
    );
});
