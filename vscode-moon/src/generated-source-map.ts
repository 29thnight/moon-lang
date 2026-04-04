import * as fs from 'fs';
import * as path from 'path';
import { findMoonProjectRoot, ProjectFsLike, resolveGeneratedCsPath } from './project-config';

export interface MoonGeneratedSourceMapSpan {
    line: number;
    col: number;
    end_line: number;
    end_col: number;
}

export interface MoonGeneratedSourceMapAnchor {
    kind: string;
    name: string;
    qualified_name: string;
    source_span: MoonGeneratedSourceMapSpan;
    generated_span?: MoonGeneratedSourceMapSpan | null;
    generated_name_span?: MoonGeneratedSourceMapSpan | null;
    segments?: MoonGeneratedSourceMapAnchor[];
}

export interface MoonGeneratedSourceMap {
    version: number;
    source_file: string;
    generated_file: string;
    declaration?: MoonGeneratedSourceMapAnchor | null;
    members: MoonGeneratedSourceMapAnchor[];
}

export function sourceMapPathForGeneratedFile(generatedFile: string): string {
    const parsed = path.parse(generatedFile);
    return path.join(parsed.dir, `${parsed.name}.mnmap.json`);
}

export function resolveGeneratedSourceMapPath(
    moonFilePath: string,
    workspaceRoots: string[],
    fsLike: ProjectFsLike = fs,
): string | null {
    const generatedFile = resolveGeneratedCsPath(moonFilePath, workspaceRoots, fsLike);
    if (!generatedFile) {
        return null;
    }

    const sourceMapPath = sourceMapPathForGeneratedFile(generatedFile);
    return fsLike.existsSync(sourceMapPath) ? sourceMapPath : null;
}

export function readSourceMapFile(
    sourceMapPath: string,
    fsLike: ProjectFsLike = fs,
): MoonGeneratedSourceMap | null {
    if (!fsLike.existsSync(sourceMapPath)) {
        return null;
    }

    try {
        const parsed = JSON.parse(fsLike.readFileSync(sourceMapPath, 'utf8')) as Partial<MoonGeneratedSourceMap>;
        if (typeof parsed.version !== 'number' || typeof parsed.source_file !== 'string' || typeof parsed.generated_file !== 'string') {
            return null;
        }

        return {
            version: parsed.version,
            source_file: parsed.source_file,
            generated_file: parsed.generated_file,
            declaration: normalizeAnchor(parsed.declaration),
            members: Array.isArray(parsed.members)
                ? parsed.members.map(anchor => normalizeAnchor(anchor)).filter((anchor): anchor is MoonGeneratedSourceMapAnchor => anchor !== null)
                : [],
        };
    } catch {
        return null;
    }
}

export function readGeneratedSourceMap(
    generatedFile: string,
    fsLike: ProjectFsLike = fs,
): MoonGeneratedSourceMap | null {
    return readSourceMapFile(sourceMapPathForGeneratedFile(generatedFile), fsLike);
}

export function findGeneratedSpanForSourcePosition(
    sourceMap: MoonGeneratedSourceMap,
    line: number,
    col: number,
): MoonGeneratedSourceMapSpan | null {
    const matchingAnchors = getAllAnchors(sourceMap)
        .filter(anchor => containsSpan(anchor.source_span, line, col))
        .sort((left, right) => compareSpanSize(left.source_span, right.source_span));

    for (const anchor of matchingAnchors) {
        const generatedSpan = getPreferredGeneratedSpan(anchor);
        if (generatedSpan) {
            return generatedSpan;
        }
    }

    return null;
}

export function findSourceAnchorForGeneratedPosition(
    sourceMap: MoonGeneratedSourceMap,
    line: number,
    col: number,
): MoonGeneratedSourceMapAnchor | null {
    const anchors = getAllAnchors(sourceMap);
    const generatedNameMatch = findMostSpecificAnchor(anchors, line, col, anchor => anchor.generated_name_span ?? null);
    if (generatedNameMatch) {
        return generatedNameMatch;
    }

    return findMostSpecificAnchor(anchors, line, col, anchor => anchor.generated_span ?? null);
}

export function findGeneratedSpanForTarget(
    sourceMap: MoonGeneratedSourceMap,
    typeName: string,
    memberName?: string,
): MoonGeneratedSourceMapSpan | null {
    if (memberName) {
        const member = sourceMap.members.find(anchor => anchor.name === memberName);
        return member ? getPreferredGeneratedSpan(member) : null;
    }

    const declaration = sourceMap.declaration;
    if (!declaration || declaration.name !== typeName) {
        return null;
    }

    return getPreferredGeneratedSpan(declaration);
}

export function resolveSourceMapSourcePath(
    generatedFile: string,
    sourceMap: MoonGeneratedSourceMap,
    workspaceRoots: string[],
    fsLike: ProjectFsLike = fs,
): string | null {
    if (!sourceMap.source_file) {
        return null;
    }

    if (path.isAbsolute(sourceMap.source_file)) {
        return sourceMap.source_file;
    }

    const projectRoot = findMoonProjectRoot(generatedFile, fsLike);
    const candidates = [
        ...(projectRoot ? [path.join(projectRoot, sourceMap.source_file)] : []),
        ...workspaceRoots.map(root => path.join(root, sourceMap.source_file)),
        path.resolve(path.dirname(generatedFile), sourceMap.source_file),
    ];

    for (const candidate of candidates) {
        if (fsLike.existsSync(candidate)) {
            return candidate;
        }
    }

    return candidates[0] ?? null;
}

function findMostSpecificAnchor(
    anchors: MoonGeneratedSourceMapAnchor[],
    line: number,
    col: number,
    getSpan: (anchor: MoonGeneratedSourceMapAnchor) => MoonGeneratedSourceMapSpan | null,
): MoonGeneratedSourceMapAnchor | null {
    const matches = anchors
        .map(anchor => ({ anchor, span: getSpan(anchor) }))
        .filter((entry): entry is { anchor: MoonGeneratedSourceMapAnchor; span: MoonGeneratedSourceMapSpan } => {
            return entry.span !== null && containsSpan(entry.span, line, col);
        })
        .sort((left, right) => compareSpanSize(left.span, right.span));

    return matches[0]?.anchor ?? null;
}

function getAllAnchors(sourceMap: MoonGeneratedSourceMap): MoonGeneratedSourceMapAnchor[] {
    return [
        ...(sourceMap.declaration ? flattenAnchor(sourceMap.declaration) : []),
        ...sourceMap.members.flatMap(member => flattenAnchor(member)),
    ];
}

function getPreferredGeneratedSpan(anchor: MoonGeneratedSourceMapAnchor): MoonGeneratedSourceMapSpan | null {
    return anchor.generated_name_span ?? anchor.generated_span ?? null;
}

function containsSpan(span: MoonGeneratedSourceMapSpan, line: number, col: number): boolean {
    if (line < span.line || line > span.end_line) {
        return false;
    }

    if (line === span.line && col < span.col) {
        return false;
    }

    if (line === span.end_line && col > span.end_col) {
        return false;
    }

    return true;
}

function compareSpanSize(left: MoonGeneratedSourceMapSpan, right: MoonGeneratedSourceMapSpan): number {
    return spanSize(left) - spanSize(right)
        || left.line - right.line
        || left.col - right.col;
}

function spanSize(span: MoonGeneratedSourceMapSpan): number {
    const lineDelta = Math.max(0, span.end_line - span.line);
    const colDelta = Math.max(0, span.end_col - span.col);
    return lineDelta * 10000 + colDelta;
}

function normalizeAnchor(anchor: Partial<MoonGeneratedSourceMapAnchor> | null | undefined): MoonGeneratedSourceMapAnchor | null {
    if (!anchor || typeof anchor.kind !== 'string' || typeof anchor.name !== 'string' || typeof anchor.qualified_name !== 'string' || !anchor.source_span) {
        return null;
    }

    return {
        kind: anchor.kind,
        name: anchor.name,
        qualified_name: anchor.qualified_name,
        source_span: anchor.source_span,
        generated_span: anchor.generated_span ?? null,
        generated_name_span: anchor.generated_name_span ?? null,
        segments: Array.isArray(anchor.segments)
            ? anchor.segments.map(segment => normalizeAnchor(segment)).filter((segment): segment is MoonGeneratedSourceMapAnchor => segment !== null)
            : [],
    };
}

function flattenAnchor(anchor: MoonGeneratedSourceMapAnchor): MoonGeneratedSourceMapAnchor[] {
    return [
        anchor,
        ...((anchor.segments ?? []).flatMap(segment => flattenAnchor(segment))),
    ];
}