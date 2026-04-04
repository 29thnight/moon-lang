import * as fs from 'fs';
import * as path from 'path';
import { DEFAULT_COMPILER_PATH, readMoonProject, resolveConfiguredPath } from './project-config';

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
        ...(context.bundledCandidates || []),
        ...(context.devCandidates || []),
    ].filter((candidate): candidate is string => Boolean(candidate));

    for (const candidate of candidates) {
        if (existsSync(candidate)) {
            return candidate;
        }
    }

    return context.fallback || 'moonc';
}

export function getProjectCompilerPath(projectRoot?: string): string | undefined {
    if (!projectRoot) {
        return undefined;
    }

    const config = readMoonProject(projectRoot);
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
        path.join(extensionPath, 'bin', 'moonc.exe'),
        path.join(extensionPath, 'bin', 'moonc'),
    ];
}

export function getWorkspaceDevCompilerCandidates(projectRoot?: string): string[] {
    if (!projectRoot) {
        return [];
    }

    return [
        path.join(projectRoot, 'target', 'release', 'moonc.exe'),
        path.join(projectRoot, 'target', 'release', 'moonc'),
        path.join(projectRoot, 'target', 'debug', 'moonc.exe'),
        path.join(projectRoot, 'target', 'debug', 'moonc'),
    ];
}
