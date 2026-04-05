/**
 * verify-package.mjs
 *
 * Validates that the bundled prism.exe inside the VS Code extension is up-to-date
 * and that its core commands work correctly.
 *
 * Checks:
 *  1. bin/prism.exe exists and matches the latest cargo build artifact (SHA-256)
 *  2. bin/prism.exe version outputs the expected version string
 *  3. prism check --json reports an error on an invalid .prsm file
 *  4. prism compile --json succeeds on a minimal valid .prsm file
 *  5. If artifacts/*.vsix exists, the binary inside the archive matches bin/prism.exe
 *
 * Exit code: 0 = all checks passed, non-zero = at least one check failed.
 */

import AdmZip from 'adm-zip';
import { execSync, spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const rootDir = resolve(scriptDir, '..');
const repoRootDir = resolve(rootDir, '..');
const manifestPath = resolve(rootDir, 'package.json');
const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));

const bundledBin = resolve(rootDir, 'bin', 'prism.exe');
const artifactsDir = resolve(rootDir, 'artifacts');
const vsixPath = resolve(
  artifactsDir,
  `${manifest.publisher}.${manifest.name}-${manifest.version}.vsix`
);

let passed = 0;
let failed = 0;

function pass(label) {
  console.log(`  ✓ ${label}`);
  passed++;
}

function fail(label, detail) {
  console.error(`  ✗ ${label}`);
  if (detail) console.error(`    ${detail}`);
  failed++;
}

function sha256(buffer) {
  return createHash('sha256').update(buffer).digest('hex');
}

// ── Check 1: bundled binary exists ───────────────────────────────────────────

console.log('\n[1] Bundled binary presence');
if (!existsSync(bundledBin)) {
  fail('bin/prism.exe exists', `Not found at ${bundledBin}. Run: npm run bundle`);
} else {
  pass('bin/prism.exe exists');

  // ── Check 2: matches latest cargo build artifact ──────────────────────────
  console.log('\n[2] Binary freshness (SHA-256 vs cargo artifact)');
  const cargoCandidates = [
    resolve(repoRootDir, 'target', 'release', 'prism.exe'),
    resolve(repoRootDir, 'target', 'debug', 'prism.exe'),
  ];
  const cargoPath = cargoCandidates.find(p => existsSync(p));

  if (!cargoPath) {
    fail('cargo artifact found', `Neither release nor debug binary found under target/. Run: cargo build -p refraction`);
  } else {
    const bundledHash = sha256(readFileSync(bundledBin));
    const cargoHash = sha256(readFileSync(cargoPath));
    if (bundledHash === cargoHash) {
      pass(`bin/prism.exe matches ${cargoPath.includes('release') ? 'release' : 'debug'} artifact`);
    } else {
      fail(
        'bin/prism.exe is up-to-date',
        `SHA-256 mismatch.\n    bundled: ${bundledHash}\n    cargo  : ${cargoHash}\n    Re-run: npm run bundle`
      );
    }
  }
}

// ── Check 3: version command ──────────────────────────────────────────────────
console.log('\n[3] version command');
if (existsSync(bundledBin)) {
  const result = spawnSync(bundledBin, ['version'], { encoding: 'utf8' });
  if (result.status !== 0) {
    fail('prism version exits 0', `exit code ${result.status}: ${result.stderr}`);
  } else if (!result.stdout.includes('prism')) {
    fail('prism version output contains "prism"', `stdout: ${result.stdout.trim()}`);
  } else {
    pass(`prism version → ${result.stdout.split('\n')[0].trim()}`);
  }
} else {
  fail('prism version (skipped — binary missing)', '');
}

// ── Check 4: check --json on an invalid file ──────────────────────────────────
console.log('\n[4] prism check --json (invalid file → E050)');
if (existsSync(bundledBin)) {
  const tmpDir = resolve(tmpdir(), `prsm-verify-${Date.now()}`);
  mkdirSync(tmpDir, { recursive: true });
  const invalidFile = resolve(tmpDir, 'Empty.prsm');
  writeFileSync(invalidFile, 'enum Empty {}\n', 'utf8');

  const result = spawnSync(bundledBin, ['check', invalidFile, '--json'], { encoding: 'utf8' });
  try {
    const json = JSON.parse(result.stdout);
    if (json.errors > 0 && json.diagnostics?.[0]?.code === 'E050') {
      pass('check --json reports E050 for empty enum');
    } else {
      fail('check --json reports E050', `Got: ${JSON.stringify(json)}`);
    }
  } catch (e) {
    fail('check --json produces valid JSON', `${e.message}\nstdout: ${result.stdout}`);
  } finally {
    rmSync(tmpDir, { recursive: true, force: true });
  }
} else {
  fail('check --json (skipped — binary missing)', '');
}

// ── Check 5: compile --json on a valid file ───────────────────────────────────
console.log('\n[5] prism compile --json (valid file → success)');
if (existsSync(bundledBin)) {
  const tmpDir = resolve(tmpdir(), `prsm-verify-${Date.now()}`);
  mkdirSync(tmpDir, { recursive: true });
  const validFile = resolve(tmpDir, 'Hello.prsm');
  writeFileSync(validFile, 'component Hello : MonoBehaviour {}\n', 'utf8');

  const result = spawnSync(
    bundledBin,
    ['compile', validFile, '--output', tmpDir, '--json'],
    { encoding: 'utf8' }
  );
  try {
    const json = JSON.parse(result.stdout);
    if (result.status === 0 && json.errors === 0 && json.compiled === 1) {
      pass('compile --json compiles Hello.prsm with 0 errors');
    } else {
      fail('compile --json succeeds', `exit=${result.status} json=${JSON.stringify(json)}`);
    }
  } catch (e) {
    fail('compile --json produces valid JSON', `${e.message}\nstdout: ${result.stdout}`);
  } finally {
    rmSync(tmpDir, { recursive: true, force: true });
  }
} else {
  fail('compile --json (skipped — binary missing)', '');
}

// ── Check 6: VSIX binary matches bundled binary ───────────────────────────────
console.log('\n[6] VSIX binary consistency (skipped if no VSIX)');
if (!existsSync(vsixPath)) {
  console.log(`  ⊘  ${vsixPath} not found — run: npm run package`);
} else if (!existsSync(bundledBin)) {
  fail('VSIX check (binary missing)', '');
} else {
  const zip = new AdmZip(vsixPath);
  const entry = zip.getEntry('extension/bin/prism.exe');
  if (!entry) {
    fail('extension/bin/prism.exe present in VSIX', `Entries: ${zip.getEntries().map(e => e.entryName).join(', ')}`);
  } else {
    const vsixBuf = zip.readFile(entry);
    const vsixHash = sha256(vsixBuf);
    const bundledHash = sha256(readFileSync(bundledBin));
    if (vsixHash === bundledHash) {
      pass('VSIX binary matches bin/prism.exe');
    } else {
      fail(
        'VSIX binary is up-to-date',
        `SHA-256 mismatch.\n    VSIX   : ${vsixHash}\n    bundled: ${bundledHash}\n    Re-run: npm run package`
      );
    }
  }
}

// ── Summary ───────────────────────────────────────────────────────────────────
console.log(`\n${'─'.repeat(50)}`);
console.log(`Results: ${passed} passed, ${failed} failed`);
if (failed > 0) {
  process.exit(1);
}
