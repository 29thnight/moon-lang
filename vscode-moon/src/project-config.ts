import * as fs from 'fs';
import * as path from 'path';

export interface MoonProjectConfig {
    compilerPath?: string;
    outputDir?: string;
}

export interface ProjectFsLike {
    existsSync(targetPath: string): boolean;
    readFileSync(targetPath: string, encoding: BufferEncoding): string;
}

export const DEFAULT_COMPILER_PATH = 'moonc';

const DEFAULT_OUTPUT_DIRS = [
    path.join('Packages', 'com.moon.generated', 'Runtime'),
    path.join('Assets', 'Generated', 'Moon'),
];

export function parseMoonProject(content: string): MoonProjectConfig {
    let currentSection = '';
    const config: MoonProjectConfig = {};

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
            if (key === 'moonc_path') {
                config.compilerPath = value;
            } else if (key === 'output_dir') {
                config.outputDir = value;
            }
        }
    }

    return config;
}

export function readMoonProject(projectRoot: string, fsLike: ProjectFsLike = fs): MoonProjectConfig | null {
    const projectFile = path.join(projectRoot, '.mnproject');
    if (!fsLike.existsSync(projectFile)) {
        return null;
    }

    try {
        return parseMoonProject(fsLike.readFileSync(projectFile, 'utf8'));
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

export function findMoonProjectRoot(startPath: string, fsLike: ProjectFsLike = fs): string | null {
    let current = path.extname(startPath) ? path.dirname(startPath) : startPath;

    while (true) {
        const projectFile = path.join(current, '.mnproject');
        if (fsLike.existsSync(projectFile)) {
            return current;
        }

        const parent = path.dirname(current);
        if (parent === current) {
            return null;
        }
        current = parent;
    }
}

export function getOutputDirCandidates(projectRoot: string, config?: MoonProjectConfig | null): string[] {
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

export function resolveGeneratedCsPath(
    mnPath: string,
    workspaceRoots: string[],
    fsLike: ProjectFsLike = fs,
): string | null {
    const fileName = `${path.basename(mnPath, '.mn')}.cs`;
    const projectRoot = findMoonProjectRoot(mnPath, fsLike);

    if (projectRoot) {
        const config = readMoonProject(projectRoot, fsLike);
        for (const outputDir of getOutputDirCandidates(projectRoot, config)) {
            const candidate = path.join(outputDir, fileName);
            if (fsLike.existsSync(candidate)) {
                return candidate;
            }
        }
    }

    for (const workspaceRoot of workspaceRoots) {
        const config = readMoonProject(workspaceRoot, fsLike);
        for (const outputDir of getOutputDirCandidates(workspaceRoot, config)) {
            const candidate = path.join(outputDir, fileName);
            if (fsLike.existsSync(candidate)) {
                return candidate;
            }
        }
    }

    const sameDirPath = mnPath.replace(/\.mn$/, '.cs');
    return fsLike.existsSync(sameDirPath) ? sameDirPath : null;
}
