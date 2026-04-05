import * as fs from 'fs';
import * as path from 'path';

export interface PrismProjectConfig {
    compilerPath?: string;
    outputDir?: string;
}

export interface ProjectFsLike {
    existsSync(targetPath: string): boolean;
    readFileSync(targetPath: string, encoding: BufferEncoding): string;
}

export const DEFAULT_COMPILER_PATH = 'prism';
const LEGACY_COMPILER_PATH = 'moonc';

const DEFAULT_OUTPUT_DIRS = [
    path.join('Packages', 'com.prsm.generated', 'Runtime'),
    path.join('Assets', 'Generated', 'PrSM'),
    path.join('Packages', 'com.moon.generated', 'Runtime'),
];

function getProjectFilePath(projectRoot: string, fsLike: ProjectFsLike = fs): string | null {
    const currentProjectFile = path.join(projectRoot, '.prsmproject');
    if (fsLike.existsSync(currentProjectFile)) {
        return currentProjectFile;
    }

    const legacyProjectFile = path.join(projectRoot, '.mnproject');
    if (fsLike.existsSync(legacyProjectFile)) {
        return legacyProjectFile;
    }

    return null;
}

function normalizeCompilerPath(configuredPath: string): string {
    if (!configuredPath || configuredPath === LEGACY_COMPILER_PATH) {
        return DEFAULT_COMPILER_PATH;
    }

    return configuredPath;
}

function normalizeOutputDir(outputDir: string): string {
    const normalizedOutputDir = outputDir.replace(/\\/g, '/');
    if (!normalizedOutputDir || normalizedOutputDir === 'Packages/com.moon.generated/Runtime') {
        return 'Packages/com.prsm.generated/Runtime';
    }

    return normalizedOutputDir;
}

function getSourceStem(sourcePath: string): string {
    if (sourcePath.endsWith('.prsm')) {
        return path.basename(sourcePath, '.prsm');
    }

    if (sourcePath.endsWith('.mn')) {
        return path.basename(sourcePath, '.mn');
    }

    return path.parse(sourcePath).name;
}

export function parsePrismProject(content: string): PrismProjectConfig {
    let currentSection = '';
    const config: PrismProjectConfig = {};

    for (const rawLine of content.split(/\r?\n/)) {
        const line = rawLine.trim();
        if (!line || line.startsWith('#')) {
            continue;
        }

        if (line.startsWith('[') && line.endsWith(']')) {
            currentSection = line.slice(1, -1).trim();
            continue;
        }

        const separator = line.indexOf('=');
        if (separator < 0) {
            continue;
        }

        const key = line.slice(0, separator).trim();
        let value = line.slice(separator + 1).trim();
        if (value.startsWith('"') && value.endsWith('"')) {
            value = value.slice(1, -1);
        }

        if (currentSection === 'compiler') {
            if (key === 'prism_path' || key === 'moonc_path') {
                config.compilerPath = normalizeCompilerPath(value);
            } else if (key === 'output_dir') {
                config.outputDir = normalizeOutputDir(value);
            }
        }
    }

    return config;
}

export function readPrismProject(projectRoot: string, fsLike: ProjectFsLike = fs): PrismProjectConfig | null {
    const projectFile = getProjectFilePath(projectRoot, fsLike);
    if (!projectFile) {
        return null;
    }

    try {
        return parsePrismProject(fsLike.readFileSync(projectFile, 'utf8'));
    } catch {
        return null;
    }
}

export function resolveConfiguredPath(projectRoot: string, configuredPath: string): string {
    const normalizedPath = configuredPath.trim();
    if (!normalizedPath || normalizedPath === DEFAULT_COMPILER_PATH) {
        return normalizedPath;
    }

    return path.isAbsolute(normalizedPath)
        ? normalizedPath
        : path.join(projectRoot, normalizedPath);
}

export function findPrismProjectRoot(startPath: string, fsLike: ProjectFsLike = fs): string | null {
    let current = path.extname(startPath) ? path.dirname(startPath) : startPath;

    while (true) {
        if (getProjectFilePath(current, fsLike)) {
            return current;
        }

        const parent = path.dirname(current);
        if (parent === current) {
            return null;
        }
        current = parent;
    }
}

export function getOutputDirCandidates(projectRoot: string, config?: PrismProjectConfig | null): string[] {
    const candidates: string[] = [];
    const seen = new Set<string>();

    const pushCandidate = (candidate: string) => {
        if (!seen.has(candidate)) {
            seen.add(candidate);
            candidates.push(candidate);
        }
    };

    if (config?.outputDir) {
        pushCandidate(resolveConfiguredPath(projectRoot, config.outputDir));
    }

    for (const defaultDir of DEFAULT_OUTPUT_DIRS) {
        pushCandidate(path.join(projectRoot, defaultDir));
    }

    return candidates;
}

export function resolveGeneratedCsPath(prsmPath: string,
    workspaceRoots: string[],
    fsLike: ProjectFsLike = fs,
): string | null {
    const fileName = `${getSourceStem(prsmPath)}.cs`;
    const projectRoot = findPrismProjectRoot(prsmPath, fsLike);

    if (projectRoot) {
        const config = readPrismProject(projectRoot, fsLike);
        for (const outputDir of getOutputDirCandidates(projectRoot, config)) {
            const candidate = path.join(outputDir, fileName);
            if (fsLike.existsSync(candidate)) {
                return candidate;
            }
        }
    }

    for (const workspaceRoot of workspaceRoots) {
        const config = readPrismProject(workspaceRoot, fsLike);
        for (const outputDir of getOutputDirCandidates(workspaceRoot, config)) {
            const candidate = path.join(outputDir, fileName);
            if (fsLike.existsSync(candidate)) {
                return candidate;
            }
        }
    }

    const sameDirPath = prsmPath.replace(/\.(prsm|mn)$/, '.cs');
    return fsLike.existsSync(sameDirPath) ? sameDirPath : null;
}
