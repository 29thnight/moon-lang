import { PrismReferencesResult, PrismSourceLocation } from './prism-cli';
import { PRSM_KEYWORDS } from './unity-api';

const IDENTIFIER_RE = /^[A-Za-z_][A-Za-z0-9_]*$/;
const UNSUPPORTED_RENAME_KINDS = new Set(['lifecycle']);
const KEYWORDS = new Set(PRSM_KEYWORDS.map(keyword => keyword.toLowerCase()));

export interface PrismRenamePlan {
    placeholder: string;
    locations: PrismSourceLocation[];
}

export function getPrSMRenamePlan(result: PrismReferencesResult | null): PrismRenamePlan | null {
    const definition = result?.definition;
    if (!definition || !supportsRename(definition.kind)) {
        return null;
    }

    const seen = new Set<string>();
    const locations = [definition, ...result.references].filter(location => {
        const key = getLocationKey(location);
        if (seen.has(key)) {
            return false;
        }
        seen.add(key);
        return true;
    });

    return {
        placeholder: definition.name,
        locations,
    };
}

export function getPrSMRenameSupportError(result: PrismReferencesResult | null): string {
    const definition = result?.definition;
    if (!definition) {
        return 'Only PrSM symbols defined in the current project can be renamed.';
    }

    if (normalizeKind(definition.kind) === 'lifecycle') {
        return 'Lifecycle blocks map to fixed Unity callback names and cannot be renamed.';
    }

    return `Rename is not supported for ${definition.kind} symbols.`;
}

export function validatePrSMRenameName(newName: string): string | undefined {
    const trimmed = newName.trim();
    if (!trimmed) {
        return 'Rename target cannot be empty.';
    }

    if (trimmed !== newName) {
        return 'Rename target cannot start or end with whitespace.';
    }

    if (!IDENTIFIER_RE.test(trimmed)) {
        return 'Rename target must be a valid PrSM identifier.';
    }

    if (KEYWORDS.has(trimmed.toLowerCase())) {
        return `'${trimmed}' is a reserved PrSM keyword.`;
    }

    return undefined;
}

function supportsRename(kind: string): boolean {
    return !UNSUPPORTED_RENAME_KINDS.has(normalizeKind(kind));
}

function getLocationKey(location: PrismSourceLocation): string {
    return [
        location.file,
        location.line,
        location.col,
        location.end_line ?? location.line,
        location.end_col ?? location.col,
    ].join(':');
}

function normalizeKind(kind?: string | null): string {
    return (kind ?? '').replace(/[\s_-]/g, '').toLowerCase();
}