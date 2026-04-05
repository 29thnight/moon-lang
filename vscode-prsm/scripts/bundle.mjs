import { build, context } from 'esbuild';
import { cpSync, existsSync, mkdirSync, rmSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const rootDir = resolve(scriptDir, '..');
const distDir = resolve(rootDir, 'dist');
const binDir = resolve(rootDir, 'bin');
const repoRootDir = resolve(rootDir, '..');
const watchMode = process.argv.includes('--watch');
const betterSqliteDir = resolve(rootDir, 'node_modules', 'better-sqlite3');

const buildOptions = {
  entryPoints: [resolve(rootDir, 'src', 'extension.ts')],
  outfile: resolve(distDir, 'extension.js'),
  bundle: true,
  platform: 'node',
  format: 'cjs',
  target: 'node20',
  sourcemap: true,
  sourcesContent: false,
  logLevel: 'info',
  legalComments: 'none',
  external: ['vscode', 'better-sqlite3']
};

function stageBetterSqliteRuntime() {
  if (!existsSync(betterSqliteDir)) {
    throw new Error('better-sqlite3 is not installed. Run npm install before packaging the extension.');
  }

  const vendorRoot = resolve(distDir, 'vendor', 'better-sqlite3');
  const runtimeFiles = [
    ['LICENSE', 'LICENSE'],
    ['lib', 'lib'],
    ['build/Release/better_sqlite3.node', 'build/Release/better_sqlite3.node']
  ];

  for (const [sourceRelativePath, destinationRelativePath] of runtimeFiles) {
    const sourcePath = resolve(betterSqliteDir, sourceRelativePath);
    const destinationPath = resolve(vendorRoot, destinationRelativePath);

    if (!existsSync(sourcePath)) {
      throw new Error(`Missing better-sqlite3 runtime file: ${sourceRelativePath}`);
    }

    mkdirSync(dirname(destinationPath), { recursive: true });
    cpSync(sourcePath, destinationPath, { recursive: true });
  }
}

function stageBundledCompiler() {
  const compilerCandidates = [
    resolve(repoRootDir, 'target', 'release', 'prism.exe'),
    resolve(repoRootDir, 'target', 'debug', 'prism.exe')
  ];
  const compilerPath = compilerCandidates.find(candidate => existsSync(candidate));

  if (!compilerPath) {
    throw new Error(
      'PrSM compiler binary not found. Build the compiler first with `cargo build -p refraction` before packaging the VS Code extension.'
    );
  }

  mkdirSync(binDir, { recursive: true });
  cpSync(compilerPath, resolve(binDir, 'prism.exe'));
}

function cleanDist() {
  rmSync(distDir, { recursive: true, force: true });
  mkdirSync(distDir, { recursive: true });
}

if (watchMode) {
  cleanDist();
  stageBundledCompiler();
  stageBetterSqliteRuntime();
  const buildContext = await context(buildOptions);
  await buildContext.watch();
  console.log('Watching PrSM extension bundle...');
} else {
  cleanDist();
  await build(buildOptions);
  stageBundledCompiler();
  stageBetterSqliteRuntime();
}
