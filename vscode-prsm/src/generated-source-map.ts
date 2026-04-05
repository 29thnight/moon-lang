import * as fs from 'fs';
import * as path from 'path';
import { findPrismProjectRoot, ProjectFsLike, resolveGeneratedCsPath } from './project-config';

export interface PrismGeneratedSourceMapSpan {
    line: number;
    col: number;
    end_line: number;
    end_col: number;
}

export interface PrSMGeneratedSourceMapAnchor {
    kind: string;
    name: string;
    qualified_name: string;
    source_span: PrismGeneratedSourceMapSpan;
    generated_span?: PrismGeneratedSourceMapSpan | null;
    generated_name_span?: PrismGeneratedSourceMapSpan | null;
    segments?: PrSMGeneratedSourceMapAnchor[];
}

export interface PrSMGeneratedSourceMap {
    version: number;
    source_file: string;
    generated_file: string;
    declaration?: PrSMGeneratedSourceMapAnchor | null;
    members: PrSMGeneratedSourceMapAnchor[];
}

export function sourceMapPathForGeneratedFile(generatedFile: string): string {
    const parsed = path.parse(generatedFile);
    return path.join(parsed.dir, `${parsed.name}.prsmmap.json`);
}

export function resolveGeneratedSourceMapPath(
    prsmFilePath: string,
    workspaceRoots: string[],
    fsLike: ProjectFsLike = fs,
): string | null {
    const generatedFile = resolveGeneratedCsPath(prsmFilePath, workspaceRoots, fsLike);
    if (!generatedFile) {
        return null;
    }

    const sourceMapPath = sourceMapPathForGeneratedFile(generatedFile);
    return fsLike.existsSync(sourceMapPath) ? sourceMapPath : null;
}

export function readSourceMapFile(
    sourceMapPath: string,
    fsLike: ProjectFsLike = fs,
): PrSMGeneratedSourceMap | null {
    if (!fsLike.existsSync(sourceMapPath)) {
        return null;
    }

    try {
        const parsed = JSON.parse(fsLike.readFileSync(sourceMapPath, 'utf8')) as Partial<PrSMGeneratedSourceMap>;
        if (typeof parsed.version !== 'number' || typeof parsed.source_file !== 'string' || typeof parsed.generated_file !== 'string') {
            return null;
        }

        return {
            version: parsed.version,
            source_file: parsed.source_file,
            generated_file: parsed.generated_file,
            declaration: normalizeAnchor(parsed.declaration),
            members: Array.isArray(parsed.members)
                ? parsed.members.map(anchor => normalizeAnchor(anchor)).filter((anchor): anchor is PrSMGeneratedSourceMapAnchor => anchor !== null)
                : [],
        };
    } catch {
        return null;
    }
}

export function readGeneratedSourceMap(
    generatedFile: string,
    fsLike: ProjectFsLike = fs,
): PrSMGeneratedSourceMap | null {
    return readSourceMapFile(sourceMapPathForGeneratedFile(generatedFile), fsLike);
}

export function findGeneratedSpanForSourcePosition(
    sourceMap: PrSMGeneratedSourceMap,
    line: number,
    col: number,
): PrismGeneratedSourceMapSpan | null {
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
    sourceMap: PrSMGeneratedSourceMap,
    line: number,
    col: number,
): PrSMGeneratedSourceMapAnchor | null {
    const anchors = getAllAnchors(sourceMap);
    const generatedNameMatch = findMostSpecificAnchor(anchors, line, col, anchor => anchor.generated_name_span ?? null);
    if (generatedNameMatch) {
        return generatedNameMatch;
    }

    return findMostSpecificAnchor(anchors, line, col, anchor => anchor.generated_span ?? null);
}

export function findGeneratedSpanForTarget(
    sourceMap: PrSMGeneratedSourceMap,
    typeName: string,
    memberName?: string,
): PrismGeneratedSourceMapSpan | null {
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
    sourceMap: PrSMGeneratedSourceMap,
    workspaceRoots: string[],
    fsLike: ProjectFsLike = fs,
): string | null {
    if (!sourceMap.source_file) {
        return null;
    }

    if (path.isAbsolute(sourceMap.source_file)) {
        return sourceMap.source_file;
    }

    const projectRoot = findPrismProjectRoot(generatedFile, fsLike);
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
    anchors: PrSMGeneratedSourceMapAnchor[],
    line: number,
    col: number,
    getSpan: (anchor: PrSMGeneratedSourceMapAnchor) => PrismGeneratedSourceMapSpan | null,
): PrSMGeneratedSourceMapAnchor | null {
    const matches = anchors
        .map(anchor => ({ anchor, span: getSpan(anchor) }))
        .filter((entry): entry is { anchor: PrSMGeneratedSourceMapAnchor; span: PrismGeneratedSourceMapSpan } => {
            return entry.span !== null && containsSpan(entry.span, line, col);
        })
        .sort((left, right) => compareSpanSize(left.span, right.span));

    return matches[0]?.anchor ?? null;
}

function getAllAnchors(sourceMap: PrSMGeneratedSourceMap): PrSMGeneratedSourceMapAnchor[] {
    return [
        ...(sourceMap.declaration ? flattenAnchor(sourceMap.declaration) : []),
        ...sourceMap.members.flatMap(member => flattenAnchor(member)),
    ];
}

function getPreferredGeneratedSpan(anchor: PrSMGeneratedSourceMapAnchor): PrismGeneratedSourceMapSpan | null {
    return anchor.generated_name_span ?? anchor.generated_span ?? null;
}

function containsSpan(span: PrismGeneratedSourceMapSpan, line: number, col: number): boolean {
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

function compareSpanSize(left: PrismGeneratedSourceMapSpan, right: PrismGeneratedSourceMapSpan): number {
    return spanSize(left) - spanSize(right)
        || left.line - right.line
        || left.col - right.col;
}

function spanSize(span: PrismGeneratedSourceMapSpan): number {
    const lineDelta = Math.max(0, span.end_line - span.line);
    const colDelta = Math.max(0, span.end_col - span.col);
    return lineDelta * 10000 + colDelta;
}

function normalizeAnchor(anchor: Partial<PrSMGeneratedSourceMapAnchor> | null | undefined): PrSMGeneratedSourceMapAnchor | null {
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
            ? anchor.segments.map(segment => normalizeAnchor(segment)).filter((segment): segment is PrSMGeneratedSourceMapAnchor => segment !== null)
            : [],
    };
}

function flattenAnchor(anchor: PrSMGeneratedSourceMapAnchor): PrSMGeneratedSourceMapAnchor[] {
    return [
        anchor,
        ...((anchor.segments ?? []).flatMap(segment => flattenAnchor(segment))),
    ];
}