import test from 'node:test';
import assert from 'node:assert/strict';
import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';
import {
    getProjectCompilerPath,
    resolveCompilerPathFromContext,
} from '../compiler-resolver';

function tempDir(prefix: string): string {
    return fs.mkdtempSync(path.join(os.tmpdir(), prefix));
}

function fixturePath(name: string): string {
    return path.resolve(__dirname, '../../src/test/fixtures/mnproject', name);
}

test('resolveCompilerPathFromContext follows override precedence', () => {
    const root = tempDir('moon-compiler-resolution-');
    const userOverride = path.join(root, 'override', 'moonc.exe');
    const projectCompiler = path.join(root, 'project', 'moonc.exe');
    const bundledCompiler = path.join(root, 'bundled', 'moonc.exe');
    const devCompiler = path.join(root, 'target', 'debug', 'moonc.exe');

    for (const compilerPath of [userOverride, projectCompiler, bundledCompiler, devCompiler]) {
        fs.mkdirSync(path.dirname(compilerPath), { recursive: true });
        fs.writeFileSync(compilerPath, '');
    }

    assert.equal(
        resolveCompilerPathFromContext({
            userOverride,
            projectCompilerPath: projectCompiler,
            bundledCandidates: [bundledCompiler],
            devCandidates: [devCompiler],
            fallback: 'moonc',
        }),
        userOverride,
    );

    fs.rmSync(userOverride, { force: true });
    assert.equal(
        resolveCompilerPathFromContext({
            userOverride,
            projectCompilerPath: projectCompiler,
            bundledCandidates: [bundledCompiler],
            devCandidates: [devCompiler],
            fallback: 'moonc',
        }),
        projectCompiler,
    );

    fs.rmSync(projectCompiler, { force: true });
    assert.equal(
        resolveCompilerPathFromContext({
            projectCompilerPath: projectCompiler,
            bundledCandidates: [bundledCompiler],
            devCandidates: [devCompiler],
            fallback: 'moonc',
        }),
        bundledCompiler,
    );

    fs.rmSync(bundledCompiler, { force: true });
    assert.equal(
        resolveCompilerPathFromContext({
            bundledCandidates: [bundledCompiler],
            devCandidates: [devCompiler],
            fallback: 'moonc',
        }),
        devCompiler,
    );

    fs.rmSync(root, { recursive: true, force: true });
});

test('getProjectCompilerPath resolves relative paths from .mnproject', () => {
    const root = tempDir('moon-project-compiler-');
    fs.mkdirSync(path.join(root, 'tools'), { recursive: true });
    fs.copyFileSync(
        fixturePath('relative-compiler.mnproject'),
        path.join(root, '.mnproject'),
    );

    assert.equal(
        getProjectCompilerPath(root),
        path.join(root, 'tools', 'moonc.exe'),
    );

    fs.rmSync(root, { recursive: true, force: true });
});

test('getProjectCompilerPath ignores the default moonc sentinel', () => {
    const root = tempDir('moon-project-default-compiler-');
    fs.copyFileSync(
        fixturePath('default-compiler.mnproject'),
        path.join(root, '.mnproject'),
    );

    assert.equal(getProjectCompilerPath(root), undefined);

    fs.rmSync(root, { recursive: true, force: true });
});
