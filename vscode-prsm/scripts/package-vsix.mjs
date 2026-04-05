import AdmZip from 'adm-zip';
import { execSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const rootDir = resolve(scriptDir, '..');
const manifestPath = resolve(rootDir, 'package.json');
const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
const artifactsDir = resolve(rootDir, 'artifacts');
const vsixPath = resolve(artifactsDir, `${manifest.publisher}.${manifest.name}-${manifest.version}.vsix`);
const vsceExecutable = process.platform === 'win32'
  ? resolve(rootDir, 'node_modules', '.bin', 'vsce.cmd')
  : resolve(rootDir, 'node_modules', '.bin', 'vsce');

if (!existsSync(vsceExecutable)) {
  throw new Error('VS Code packaging tool not found. Run npm install before packaging the extension.');
}

mkdirSync(artifactsDir, { recursive: true });

execSync(`"${vsceExecutable}" package --out "${vsixPath}"`, {
  cwd: rootDir,
  stdio: 'inherit'
});

const zip = new AdmZip(vsixPath);
verifyPackagedFile(zip, resolve(rootDir, 'dist', 'extension.js'), 'extension/dist/extension.js');
verifyPackagedFile(zip, resolve(rootDir, 'bin', 'prism.exe'), 'extension/bin/prism.exe');
verifyPackagedManifest(zip, manifest);

console.log(`Verified VSIX package: ${vsixPath}`);

function verifyPackagedFile(zipFile, localPath, entryName) {
  if (!existsSync(localPath)) {
    throw new Error(`Local package artifact not found: ${localPath}`);
  }

  const entry = zipFile.getEntry(entryName);
  if (!entry) {
    throw new Error(`VSIX entry not found: ${entryName}`);
  }

  const packagedBuffer = zipFile.readFile(entry);
  if (!packagedBuffer) {
    throw new Error(`Failed to read VSIX entry: ${entryName}`);
  }

  const packagedHash = sha256(packagedBuffer);
  const localHash = sha256(readFileSync(localPath));
  if (packagedHash !== localHash) {
    throw new Error(
      `VSIX entry mismatch for ${entryName}. Expected ${localHash}, got ${packagedHash}.`
    );
  }
}

function verifyPackagedManifest(zipFile, localManifest) {
  const entry = zipFile.getEntry('extension/package.json');
  if (!entry) {
    throw new Error('VSIX entry not found: extension/package.json');
  }

  const packagedBuffer = zipFile.readFile(entry);
  if (!packagedBuffer) {
    throw new Error('Failed to read VSIX entry: extension/package.json');
  }

  const packagedManifest = JSON.parse(packagedBuffer.toString('utf8'));
  if (packagedManifest.version !== localManifest.version) {
    throw new Error(
      `VSIX manifest version mismatch. Expected ${localManifest.version}, got ${packagedManifest.version}.`
    );
  }

  if (packagedManifest.main !== localManifest.main) {
    throw new Error(
      `VSIX manifest main entry mismatch. Expected ${localManifest.main}, got ${packagedManifest.main}.`
    );
  }
}

function sha256(buffer) {
  return createHash('sha256').update(buffer).digest('hex');
}