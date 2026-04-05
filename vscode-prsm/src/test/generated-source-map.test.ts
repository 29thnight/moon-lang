import test from 'node:test';
import assert from 'node:assert/strict';
import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';
import {
    findGeneratedSpanForSourcePosition,
    findGeneratedSpanForTarget,
    findSourceAnchorForGeneratedPosition,
    readGeneratedSourceMap,
    resolveGeneratedSourceMapPath,
    resolveSourceMapSourcePath,
    sourceMapPathForGeneratedFile,
    type PrSMGeneratedSourceMap,
} from '../generated-source-map';

function tempDir(prefix: string): string {
    return fs.mkdtempSync(path.join(os.tmpdir(), prefix));
}

function sampleSourceMap(sourceFile: string, generatedFile: string): PrSMGeneratedSourceMap {
    return {
        version: 1,
        source_file: sourceFile,
        generated_file: generatedFile,
        declaration: {
            kind: 'type',
            name: 'Player',
            qualified_name: 'Player',
            source_span: { line: 1, col: 11, end_line: 1, end_col: 16 },
            generated_span: { line: 7, col: 1, end_line: 23, end_col: 1 },
            generated_name_span: { line: 7, col: 14, end_line: 7, end_col: 19 },
        },
        members: [
            {
                kind: 'field',
                name: 'speed',
                qualified_name: 'Player.speed',
                source_span: { line: 2, col: 15, end_line: 2, end_col: 19 },
                generated_span: { line: 9, col: 1, end_line: 14, end_col: 5 },
                generated_name_span: { line: 10, col: 18, end_line: 10, end_col: 22 },
            },
            {
                kind: 'function',
                name: 'jump',
                qualified_name: 'Player.jump',
                source_span: { line: 8, col: 10, end_line: 8, end_col: 13 },
                generated_span: { line: 18, col: 1, end_line: 22, end_col: 5 },
                generated_name_span: { line: 18, col: 17, end_line: 18, end_col: 20 },
                segments: [
                    {
                        kind: 'statement',
                        name: 'stmt1',
                        qualified_name: 'Player.jump#stmt1',
                        source_span: { line: 9, col: 13, end_line: 9, end_col: 24 },
                        generated_span: { line: 19, col: 1, end_line: 19, end_col: 32 },
                    },
                ],
            },
        ],
    };
}

test('sourceMapPathForGeneratedFile uses the prsmmap sidecar extension', () => {
    assert.equal(
        sourceMapPathForGeneratedFile(path.join('Generated', 'Player.cs')),
        path.join('Generated', 'Player.prsmmap.json'),
    );
});

test('resolveGeneratedSourceMapPath finds a sidecar next to configured generated output', () => {
    const root = tempDir('prsm-source-map-path-');
    const prsmFile = path.join(root, 'Assets', 'Scripts', 'Player.prsm');
    const generatedDir = path.join(root, 'Generated', 'PrSM');
    const generatedFile = path.join(generatedDir, 'Player.cs');
    const sourceMapFile = path.join(generatedDir, 'Player.prsmmap.json');

    fs.mkdirSync(path.dirname(prsmFile), { recursive: true });
    fs.mkdirSync(generatedDir, { recursive: true });
    fs.writeFileSync(path.join(root, '.prsmproject'), `[project]\nname = "Test"\n\n[compiler]\noutput_dir = "Generated/PrSM"\n`);
    fs.writeFileSync(prsmFile, 'component Player : MonoBehaviour {}');
    fs.writeFileSync(generatedFile, '// generated');
    fs.writeFileSync(sourceMapFile, JSON.stringify(sampleSourceMap(prsmFile, generatedFile)));

    assert.equal(resolveGeneratedSourceMapPath(prsmFile, [root]), sourceMapFile);
    assert.ok(readGeneratedSourceMap(generatedFile));

    fs.rmSync(root, { recursive: true, force: true });
});

test('source map helpers map source and generated positions through anchors', () => {
    const sourceMap = sampleSourceMap('/workspace/Assets/Player.prsm', '/workspace/Generated/Player.cs');

    assert.deepEqual(findGeneratedSpanForTarget(sourceMap, 'Player'), sourceMap.declaration?.generated_name_span ?? null);
    assert.deepEqual(findGeneratedSpanForTarget(sourceMap, 'Player', 'jump'), sourceMap.members[1].generated_name_span ?? null);
    assert.deepEqual(findGeneratedSpanForSourcePosition(sourceMap, 2, 16), sourceMap.members[0].generated_name_span ?? null);
    assert.deepEqual(findGeneratedSpanForSourcePosition(sourceMap, 9, 14), sourceMap.members[1].segments?.[0].generated_span ?? null);

    const generatedAnchor = findSourceAnchorForGeneratedPosition(sourceMap, 19, 3);
    assert.equal(generatedAnchor?.qualified_name, 'Player.jump#stmt1');
});

test('resolveSourceMapSourcePath resolves relative source files against the project root', () => {
    const root = tempDir('prsm-source-map-source-path-');
    const sourceFile = path.join(root, 'Assets', 'Player.prsm');
    const generatedFile = path.join(root, 'Generated', 'PrSM', 'Player.cs');

    fs.mkdirSync(path.dirname(sourceFile), { recursive: true });
    fs.mkdirSync(path.dirname(generatedFile), { recursive: true });
    fs.writeFileSync(path.join(root, '.prsmproject'), '[project]\nname = "Test"\n');
    fs.writeFileSync(sourceFile, 'component Player : MonoBehaviour {}');
    fs.writeFileSync(generatedFile, '// generated');

    const resolved = resolveSourceMapSourcePath(generatedFile, sampleSourceMap(path.join('Assets', 'Player.prsm'), generatedFile), [root]);
    assert.equal(resolved, sourceFile);

    fs.rmSync(root, { recursive: true, force: true });
});