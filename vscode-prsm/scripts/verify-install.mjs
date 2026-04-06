import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import {
  existsSync,
  mkdtempSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const rootDir = resolve(scriptDir, '..');
const manifestPath = resolve(rootDir, 'package.json');
const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
const extensionId = `${manifest.publisher}.${manifest.name}`;
const vsixPath = resolve(rootDir, 'artifacts', `${extensionId}-${manifest.version}.vsix`);
const localMainPath = resolve(rootDir, manifest.main);
const localBinaryPath = resolve(rootDir, 'bin', 'prism.exe');
const DEFAULT_COMMAND_TIMEOUT_MS = 15000;
const INSTALL_COMMAND_TIMEOUT_MS = 45000;

let passed = 0;
let failed = 0;

function pass(label) {
  console.log(`  ✓ ${label}`);
  passed++;
}

function fail(label, detail) {
  console.error(`  ✗ ${label}`);
  if (detail) {
    console.error(`    ${detail}`);
  }
  failed++;
}

function sha256(buffer) {
  return createHash('sha256').update(buffer).digest('hex');
}

function quoteForCmd(argument) {
  const text = String(argument);
  if (text.length === 0 || /[\s"]/u.test(text)) {
    return `"${text.replace(/"/g, '""')}"`;
  }

  return text;
}

function resolveCommandInvocation(command, args) {
  if (process.platform === 'win32' && /\.cmd$/i.test(command)) {
    return {
      filePath: process.env.ComSpec ?? 'cmd.exe',
      args: ['/d', '/c', ['call', quoteForCmd(command), ...args.map(quoteForCmd)].join(' ')],
    };
  }

  return {
    filePath: command,
    args,
  };
}

function runCommand(command, args, timeout = DEFAULT_COMMAND_TIMEOUT_MS) {
  const invocation = resolveCommandInvocation(command, args);

  return spawnSync(invocation.filePath, invocation.args, {
    encoding: 'utf8',
    windowsHide: true,
    timeout,
  });
}

function formatFailure(result) {
  if (result.error) {
    return result.error.message;
  }

  const stdout = result.stdout?.trim();
  const stderr = result.stderr?.trim();
  const details = [`exit=${result.status ?? 'null'}`];

  if (result.signal) {
    details.push(`signal=${result.signal}`);
  }

  if (stdout) {
    details.push(`stdout=${stdout}`);
  }

  if (stderr) {
    details.push(`stderr=${stderr}`);
  }

  return details.join('\n    ');
}

function findCodeCli() {
  const candidates = [];
  const explicitCli = process.env.PRSM_VSCODE_CLI?.trim();
  if (explicitCli) {
    candidates.push(explicitCli);
  }

  if (process.platform === 'win32') {
    candidates.push('code.cmd');

    const localAppData = process.env.LOCALAPPDATA;
    if (localAppData) {
      candidates.push(resolve(localAppData, 'Programs', 'Microsoft VS Code', 'Code.exe'));
    }

    candidates.push('code');
  } else {
    candidates.push('code', 'code-insiders');
  }

  const seen = new Set();
  for (const candidate of candidates) {
    if (!candidate) {
      continue;
    }

    const normalized = process.platform === 'win32' ? candidate.toLowerCase() : candidate;
    if (seen.has(normalized)) {
      continue;
    }
    seen.add(normalized);

    const result = runCommand(candidate, ['--version']);
    if (!result.error && result.status === 0) {
      return candidate;
    }
  }

  return null;
}

function findInstalledExtensionDir(extensionsDir) {
  const directories = readdirSync(extensionsDir, { withFileTypes: true })
    .filter(entry => entry.isDirectory() && entry.name.toLowerCase().startsWith(`${extensionId.toLowerCase()}-`))
    .map(entry => resolve(extensionsDir, entry.name));

  for (const directory of directories) {
    const installedManifestPath = resolve(directory, 'package.json');
    if (!existsSync(installedManifestPath)) {
      continue;
    }

    const installedManifest = JSON.parse(readFileSync(installedManifestPath, 'utf8'));
    if (installedManifest.version === manifest.version) {
      return directory;
    }
  }

  return directories[0] ?? null;
}

function verifyInstalledFile(installedPath, localPath, label) {
  if (!existsSync(installedPath)) {
    fail(`${label} exists in installed extension`, `Missing: ${installedPath}`);
    return;
  }

  if (!existsSync(localPath)) {
    fail(`${label} exists in workspace build`, `Missing: ${localPath}`);
    return;
  }

  const installedHash = sha256(readFileSync(installedPath));
  const localHash = sha256(readFileSync(localPath));
  if (installedHash === localHash) {
    pass(`${label} matches workspace build`);
  } else {
    fail(
      `${label} matches workspace build`,
      `installed=${installedHash}\n    local=${localHash}`
    );
  }
}

console.log('\n[1] VSIX presence');
if (!existsSync(vsixPath)) {
  fail('VSIX artifact exists', `Not found at ${vsixPath}. Run: npm run package`);
}

const codeCli = findCodeCli();
console.log('\n[2] VS Code CLI availability');
if (!codeCli) {
  fail('VS Code CLI available', 'Set PRSM_VSCODE_CLI or ensure `code` is available on PATH.');
} else {
  pass(`VS Code CLI available: ${codeCli}`);
}

if (existsSync(vsixPath) && codeCli) {
  pass(`VSIX artifact present: ${vsixPath}`);

  const sandboxRoot = mkdtempSync(resolve(tmpdir(), 'prsm-vsix-install-'));
  const extensionsDir = resolve(sandboxRoot, 'extensions');
  const userDataDir = resolve(sandboxRoot, 'user-data');
  mkdirSync(extensionsDir, { recursive: true });
  mkdirSync(userDataDir, { recursive: true });

  try {
    console.log('\n[3] VSIX install into isolated profile');
    const installResult = runCommand(codeCli, [
      '--user-data-dir',
      userDataDir,
      '--extensions-dir',
      extensionsDir,
      '--install-extension',
      vsixPath,
      '--force',
    ], INSTALL_COMMAND_TIMEOUT_MS);
    if (installResult.error || installResult.status !== 0) {
      fail('VSIX install exits 0', formatFailure(installResult));
    } else {
      pass('VSIX install exits 0');
    }

    console.log('\n[4] Installed extension registration');
    const listResult = runCommand(codeCli, [
      '--user-data-dir',
      userDataDir,
      '--extensions-dir',
      extensionsDir,
      '--list-extensions',
      '--show-versions',
    ]);
    if (listResult.error || listResult.status !== 0) {
      fail('list-extensions exits 0', formatFailure(listResult));
    } else {
      const expectedEntry = `${extensionId}@${manifest.version}`.toLowerCase();
      const installedEntries = listResult.stdout
        .split(/\r?\n/)
        .map(line => line.trim().toLowerCase())
        .filter(Boolean);

      if (installedEntries.includes(expectedEntry)) {
        pass(`Installed extension listed as ${extensionId}@${manifest.version}`);
      } else {
        fail(
          'Installed extension listed with expected version',
          `Expected ${expectedEntry}.\n    Got: ${installedEntries.join(', ') || '(empty)'}`
        );
      }
    }

    const installedDir = findInstalledExtensionDir(extensionsDir);
    console.log('\n[5] Installed extension payload');
    if (!installedDir) {
      fail('Installed extension directory exists', `No installed directory found under ${extensionsDir}`);
    } else {
      pass(`Installed extension directory: ${installedDir}`);

      const installedManifestPath = resolve(installedDir, 'package.json');
      if (!existsSync(installedManifestPath)) {
        fail('Installed package.json exists', `Missing: ${installedManifestPath}`);
      } else {
        const installedManifest = JSON.parse(readFileSync(installedManifestPath, 'utf8'));

        if (installedManifest.version === manifest.version) {
          pass(`Installed manifest version matches ${manifest.version}`);
        } else {
          fail(
            'Installed manifest version matches workspace manifest',
            `installed=${installedManifest.version} local=${manifest.version}`
          );
        }

        if (installedManifest.main === manifest.main) {
          pass(`Installed manifest main matches ${manifest.main}`);
        } else {
          fail(
            'Installed manifest main matches workspace manifest',
            `installed=${installedManifest.main} local=${manifest.main}`
          );
        }
      }

      verifyInstalledFile(resolve(installedDir, manifest.main), localMainPath, manifest.main);
      verifyInstalledFile(resolve(installedDir, 'bin', 'prism.exe'), localBinaryPath, 'bin/prism.exe');

      console.log('\n[6] Installed prism smoke');
      const installedPrism = resolve(installedDir, 'bin', 'prism.exe');
      const versionResult = runCommand(installedPrism, ['version']);
      if (versionResult.error || versionResult.status !== 0) {
        fail('Installed prism version exits 0', formatFailure(versionResult));
      } else if (!versionResult.stdout.includes('prism')) {
        fail('Installed prism version output contains "prism"', `stdout=${versionResult.stdout.trim()}`);
      } else {
        pass(`Installed prism version → ${versionResult.stdout.split('\n')[0].trim()}`);
      }
    }
  } finally {
    rmSync(sandboxRoot, { recursive: true, force: true });
  }
}

console.log(`\n${'─'.repeat(50)}`);
console.log(`Results: ${passed} passed, ${failed} failed`);
if (failed > 0) {
  process.exit(1);
}