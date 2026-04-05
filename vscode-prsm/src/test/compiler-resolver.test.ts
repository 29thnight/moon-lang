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
    return path.resolve(__dirname, '../../src/test/fixtures/prsmproject', name);
}

test('resolveCompilerPathFromContext follows override precedence', () => {
    const root = tempDir('prsm-compiler-resolution-');
    const userOverride = path.join(root, 'override', 'prism.exe');
    const projectCompiler = path.join(root, 'project', 'prism.exe');
    const bundledCompiler = path.join(root, 'bundled', 'prism.exe');
    const devCompiler = path.join(root, 'target', 'debug', 'prism.exe');

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
            fallback: 'prism',
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
            fallback: 'prism',
        }),
        projectCompiler,
    );

    fs.rmSync(projectCompiler, { force: true });
    assert.equal(
        resolveCompilerPathFromContext({
            projectCompilerPath: projectCompiler,
            bundledCandidates: [bundledCompiler],
            devCandidates: [devCompiler],
            fallback: 'prism',
        }),
        devCompiler,
    );

    fs.rmSync(devCompiler, { force: true });
    assert.equal(
        resolveCompilerPathFromContext({
            bundledCandidates: [bundledCompiler],
            devCandidates: [devCompiler],
            fallback: 'prism',
        }),
        bundledCompiler,
    );

    fs.rmSync(root, { recursive: true, force: true });
});

test('getProjectCompilerPath resolves relative paths from .prsmproject', () => {
    const root = tempDir('prsm-project-compiler-');
    fs.mkdirSync(path.join(root, 'tools'), { recursive: true });
    fs.copyFileSync(
        fixturePath('relative-compiler.prsmproject'),
        path.join(root, '.prsmproject'),
    );

    assert.equal(
        getProjectCompilerPath(root),
        path.join(root, 'tools', 'prism.exe'),
    );

    fs.rmSync(root, { recursive: true, force: true });
});

test('getProjectCompilerPath ignores the default prism sentinel', () => {
    const root = tempDir('prsm-project-default-compiler-');
    fs.copyFileSync(
        fixturePath('default-compiler.prsmproject'),
        path.join(root, '.prsmproject'),
    );

    assert.equal(getProjectCompilerPath(root), undefined);

    fs.rmSync(root, { recursive: true, force: true });
});
