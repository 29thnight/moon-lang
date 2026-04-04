import test from 'node:test';
import assert from 'node:assert/strict';
import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';
import {
    DEFAULT_COMPILER_PATH,
    findMoonProjectRoot,
    parseMoonProject,
    readMoonProject,
    resolveConfiguredPath,
    resolveGeneratedCsPath,
} from '../project-config';

function tempDir(prefix: string): string {
    return fs.mkdtempSync(path.join(os.tmpdir(), prefix));
}

function fixturePath(name: string): string {
    return path.resolve(__dirname, '../../src/test/fixtures/mnproject', name);
}

test('parseMoonProject reads compiler fields from fixture content', () => {
    const config = parseMoonProject(fs.readFileSync(fixturePath('relative-compiler.mnproject'), 'utf8'));

    assert.equal(config.compilerPath, 'tools/moonc.exe');
    assert.equal(config.outputDir, 'Packages/com.moon.generated/Runtime');
});

test('readMoonProject preserves the default compiler sentinel', () => {
    const root = tempDir('moon-default-compiler-');
    fs.copyFileSync(
        fixturePath('default-compiler.mnproject'),
        path.join(root, '.mnproject'),
    );

    const config = readMoonProject(root);

    assert.equal(config?.compilerPath, DEFAULT_COMPILER_PATH);
    assert.equal(config?.outputDir, 'Packages/com.moon.generated/Runtime');

    fs.rmSync(root, { recursive: true, force: true });
});

test('resolveConfiguredPath keeps the moonc sentinel untouched', () => {
    const root = tempDir('moon-configured-path-');

    assert.equal(resolveConfiguredPath(root, DEFAULT_COMPILER_PATH), DEFAULT_COMPILER_PATH);
    assert.equal(resolveConfiguredPath(root, 'tools/moonc.exe'), path.join(root, 'tools', 'moonc.exe'));

    fs.rmSync(root, { recursive: true, force: true });
});

test('findMoonProjectRoot walks upward from source files', () => {
    const root = tempDir('moon-project-root-');
    fs.writeFileSync(path.join(root, '.mnproject'), '[project]\nname = "Test"\n');
    const nestedFile = path.join(root, 'Assets', 'Scripts', 'Demo.mn');
    fs.mkdirSync(path.dirname(nestedFile), { recursive: true });
    fs.writeFileSync(nestedFile, 'component Demo : MonoBehaviour {}');

    assert.equal(findMoonProjectRoot(nestedFile), root);

    fs.rmSync(root, { recursive: true, force: true });
});

test('resolveGeneratedCsPath honors configured output_dir and package fallback', () => {
    const root = tempDir('moon-generated-path-');
    const mnPath = path.join(root, 'Assets', 'Scripts', 'Demo.mn');
    fs.mkdirSync(path.dirname(mnPath), { recursive: true });
    fs.writeFileSync(mnPath, 'component Demo : MonoBehaviour {}');
    fs.writeFileSync(
        path.join(root, '.mnproject'),
        `[project]
name = "Test"

[compiler]
output_dir = "Packages/com.moon.generated/Runtime"
`,
    );

    const configuredOutput = path.join(root, 'Packages', 'com.moon.generated', 'Runtime');
    fs.mkdirSync(configuredOutput, { recursive: true });
    const configuredCs = path.join(configuredOutput, 'Demo.cs');
    fs.writeFileSync(configuredCs, '// generated');

    assert.equal(resolveGeneratedCsPath(mnPath, [root]), configuredCs);

    fs.rmSync(configuredCs, { force: true });
    const fallbackOutput = path.join(root, 'Assets', 'Generated', 'Moon');
    fs.mkdirSync(fallbackOutput, { recursive: true });
    const fallbackCs = path.join(fallbackOutput, 'Demo.cs');
    fs.writeFileSync(fallbackCs, '// fallback');

    assert.equal(resolveGeneratedCsPath(mnPath, [root]), fallbackCs);

    fs.rmSync(root, { recursive: true, force: true });
});
