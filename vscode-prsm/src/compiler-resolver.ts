import * as fs from 'fs';
import * as path from 'path';
import { DEFAULT_COMPILER_PATH, readPrismProject, resolveConfiguredPath } from './project-config';

export interface CompilerResolverContext {
    userOverride?: string;
    projectCompilerPath?: string;
    bundledCandidates?: string[];
    devCandidates?: string[];
    fallback?: string;
}

export function resolveCompilerPathFromContext(
    context: CompilerResolverContext,
    existsSync: (targetPath: string) => boolean = fs.existsSync,
): string {
    const candidates = [
        context.userOverride,
        context.projectCompilerPath,
        ...(context.devCandidates || []),
        ...(context.bundledCandidates || []),
    ].filter((candidate): candidate is string => Boolean(candidate));

    for (const candidate of candidates) {
        if (existsSync(candidate)) {
            return candidate;
        }
    }

    return context.fallback || 'prism';
}

export function getProjectCompilerPath(projectRoot?: string): string | undefined {
    if (!projectRoot) {
        return undefined;
    }

    const config = readPrismProject(projectRoot);
    if (!config?.compilerPath) {
        return undefined;
    }

    const configuredPath = resolveConfiguredPath(projectRoot, config.compilerPath);
    if (!configuredPath || configuredPath === DEFAULT_COMPILER_PATH) {
        return undefined;
    }

    return configuredPath;
}

export function getBundledCompilerCandidates(extensionPath?: string): string[] {
    if (!extensionPath) {
        return [];
    }

    return [
        path.join(extensionPath, 'bin', 'prism.exe'),
        path.join(extensionPath, 'bin', 'prism'),
    ];
}

export function getWorkspaceDevCompilerCandidates(projectRoot?: string): string[] {
    if (!projectRoot) {
        return [];
    }

    return [
        path.join(projectRoot, 'target', 'release', 'prism.exe'),
        path.join(projectRoot, 'target', 'release', 'prism'),
        path.join(projectRoot, 'target', 'debug', 'prism.exe'),
        path.join(projectRoot, 'target', 'debug', 'prism'),
    ];
}
