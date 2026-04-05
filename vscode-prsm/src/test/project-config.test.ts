import test from 'node:test';
import assert from 'node:assert/strict';
import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';
import {
    DEFAULT_COMPILER_PATH,
    findPrismProjectRoot,
    parsePrismProject,
    readPrismProject,
    resolveConfiguredPath,
    resolveGeneratedCsPath,
} from '../project-config';

function tempDir(prefix: string): string {
    return fs.mkdtempSync(path.join(os.tmpdir(), prefix));
}

function fixturePath(name: string): string {
    return path.resolve(__dirname, '../../src/test/fixtures/prsmproject', name);
}

test('parsePrismProject reads compiler fields from fixture content', () => {
    const config = parsePrismProject(fs.readFileSync(fixturePath('relative-compiler.prsmproject'), 'utf8'));

    assert.equal(config.compilerPath, 'tools/prism.exe');
    assert.equal(config.outputDir, 'Packages/com.prsm.generated/Runtime');
});

test('readPrismProject preserves the default compiler sentinel', () => {
    const root = tempDir('prsm-default-compiler-');
    fs.copyFileSync(
        fixturePath('default-compiler.prsmproject'),
        path.join(root, '.prsmproject'),
    );

    const config = readPrismProject(root);

    assert.equal(config?.compilerPath, DEFAULT_COMPILER_PATH);
    assert.equal(config?.outputDir, 'Packages/com.prsm.generated/Runtime');

    fs.rmSync(root, { recursive: true, force: true });
});

test('resolveConfiguredPath keeps the prism sentinel untouched', () => {
    const root = tempDir('prsm-configured-path-');

    assert.equal(resolveConfiguredPath(root, DEFAULT_COMPILER_PATH), DEFAULT_COMPILER_PATH);
    assert.equal(resolveConfiguredPath(root, 'tools/prism.exe'), path.join(root, 'tools', 'prism.exe'));

    fs.rmSync(root, { recursive: true, force: true });
});

test('findPrismProjectRoot walks upward from source files', () => {
    const root = tempDir('prsm-project-root-');
    fs.writeFileSync(path.join(root, '.prsmproject'), '[project]\nname = "Test"\n');
    const nestedFile = path.join(root, 'Assets', 'Scripts', 'Demo.prsm');
    fs.mkdirSync(path.dirname(nestedFile), { recursive: true });
    fs.writeFileSync(nestedFile, 'component Demo : MonoBehaviour {}');

    assert.equal(findPrismProjectRoot(nestedFile), root);

    fs.rmSync(root, { recursive: true, force: true });
});

test('readPrismProject falls back to legacy .mnproject values', () => {
    const root = tempDir('prsm-legacy-project-');
    fs.writeFileSync(
        path.join(root, '.mnproject'),
        `[project]
name = "Legacy"

[compiler]
moonc_path = "moonc"
output_dir = "Packages/com.moon.generated/Runtime"
`,
    );

    const config = readPrismProject(root);

    assert.equal(config?.compilerPath, DEFAULT_COMPILER_PATH);
    assert.equal(config?.outputDir, 'Packages/com.prsm.generated/Runtime');

    fs.rmSync(root, { recursive: true, force: true });
});

test('resolveGeneratedCsPath handles legacy .mn source files', () => {
    const root = tempDir('prsm-legacy-generated-path-');
    const sourcePath = path.join(root, 'Assets', 'Scripts', 'Legacy.mn');
    fs.mkdirSync(path.dirname(sourcePath), { recursive: true });
    fs.writeFileSync(sourcePath, 'component Legacy : MonoBehaviour {}');
    fs.writeFileSync(path.join(root, '.mnproject'), '[project]\nname = "Legacy"\n');

    const outputDir = path.join(root, 'Packages', 'com.moon.generated', 'Runtime');
    fs.mkdirSync(outputDir, { recursive: true });
    const generatedPath = path.join(outputDir, 'Legacy.cs');
    fs.writeFileSync(generatedPath, '// generated');

    assert.equal(resolveGeneratedCsPath(sourcePath, [root]), generatedPath);

    fs.rmSync(root, { recursive: true, force: true });
});

test('resolveGeneratedCsPath honors configured output_dir and package fallback', () => {
    const root = tempDir('prsm-generated-path-');
    const prsmPath = path.join(root, 'Assets', 'Scripts', 'Demo.prsm');
    fs.mkdirSync(path.dirname(prsmPath), { recursive: true });
    fs.writeFileSync(prsmPath, 'component Demo : MonoBehaviour {}');
    fs.writeFileSync(
        path.join(root, '.prsmproject'),
        `[project]
name = "Test"

[compiler]
output_dir = "Packages/com.prsm.generated/Runtime"
`,
    );

    const configuredOutput = path.join(root, 'Packages', 'com.prsm.generated', 'Runtime');
    fs.mkdirSync(configuredOutput, { recursive: true });
    const configuredCs = path.join(configuredOutput, 'Demo.cs');
    fs.writeFileSync(configuredCs, '// generated');

    assert.equal(resolveGeneratedCsPath(prsmPath, [root]), configuredCs);

    fs.rmSync(configuredCs, { force: true });
    const fallbackOutput = path.join(root, 'Assets', 'Generated', 'PrSM');
    fs.mkdirSync(fallbackOutput, { recursive: true });
    const fallbackCs = path.join(fallbackOutput, 'Demo.cs');
    fs.writeFileSync(fallbackCs, '// fallback');

    assert.equal(resolveGeneratedCsPath(prsmPath, [root]), fallbackCs);

    fs.rmSync(root, { recursive: true, force: true });
});
