import { PrismDefinitionEntry, PrismIndexResult, PrismIndexedSymbol } from './prism-cli';

export interface PrismCSharpLookupTarget {
    typeName: string;
    memberName?: string;
}

export interface PrismCSharpHoverInfo extends PrismCSharpLookupTarget {
    generatedFile?: string;
    hoverText?: string;
}

const TOP_LEVEL_TYPE_KINDS = new Set([
    'asset',
    'attribute',
    'class',
    'component',
    'enum',
    'type',
]);

const MEMBER_KINDS = new Set([
    'coroutine',
    'enumentry',
    'field',
    'function',
    'lifecycle',
]);

const LIFECYCLE_MEMBER_NAMES: Record<string, string> = {
    awake: 'Awake',
    start: 'Start',
    update: 'Update',
    fixedupdate: 'FixedUpdate',
    lateupdate: 'LateUpdate',
    onenable: 'OnEnable',
    ondisable: 'OnDisable',
    ondestroy: 'OnDestroy',
    ontriggerenter: 'OnTriggerEnter',
    ontriggerexit: 'OnTriggerExit',
    ontriggerstay: 'OnTriggerStay',
    oncollisionenter: 'OnCollisionEnter',
    oncollisionexit: 'OnCollisionExit',
    oncollisionstay: 'OnCollisionStay',
};

export function getNavigationCSharpTarget(
    result: PrismIndexResult,
    definition?: PrismDefinitionEntry | null,
): PrismCSharpLookupTarget | null {
    const definitionTarget = definition ? getTargetFromDefinition(definition) : null;
    if (definitionTarget) {
        return definitionTarget;
    }

    const resolvedReferenceTarget = result.reference_at?.resolved_symbol
        ? getTargetFromSymbol(result.reference_at.resolved_symbol)
        : null;
    if (resolvedReferenceTarget) {
        return resolvedReferenceTarget;
    }

    const symbolTarget = result.symbol_at ? getTargetFromSymbol(result.symbol_at) : null;
    if (symbolTarget) {
        return symbolTarget;
    }

    if (normalizeKind(result.reference_at?.kind) === 'type' && result.reference_at?.name) {
        return { typeName: result.reference_at.name };
    }

    return null;
}

function getTargetFromDefinition(definition: PrismDefinitionEntry): PrismCSharpLookupTarget | null {
    const kind = normalizeKind(definition.kind);
    if (TOP_LEVEL_TYPE_KINDS.has(kind)) {
        return { typeName: definition.name };
    }

    if (!MEMBER_KINDS.has(kind)) {
        return null;
    }

    const typeName = getContainerTypeName(definition.qualified_name);
    if (!typeName) {
        return null;
    }

    return {
        typeName,
        memberName: getCSharpMemberName(kind, definition.name),
    };
}

function getTargetFromSymbol(symbol: PrismIndexedSymbol): PrismCSharpLookupTarget | null {
    const kind = normalizeKind(symbol.kind);
    if (TOP_LEVEL_TYPE_KINDS.has(kind)) {
        return { typeName: lastQualifiedSegment(symbol.qualified_name) };
    }

    if (!MEMBER_KINDS.has(kind)) {
        return null;
    }

    const typeName = symbol.container_name ? lastQualifiedSegment(symbol.container_name) : getContainerTypeName(symbol.qualified_name);
    if (!typeName) {
        return null;
    }

    return {
        typeName,
        memberName: getCSharpMemberName(kind, symbol.name),
    };
}

function getContainerTypeName(qualifiedName: string): string | null {
    const segments = qualifiedName.split('.').filter(Boolean);
    if (segments.length < 2) {
        return null;
    }

    return segments[segments.length - 2];
}

function getCSharpMemberName(kind: string, memberName: string): string {
    if (kind === 'lifecycle') {
        return LIFECYCLE_MEMBER_NAMES[normalizeKind(memberName)] ?? capitalize(memberName);
    }

    return memberName;
}

function lastQualifiedSegment(name: string): string {
    const segments = name.split('.').filter(Boolean);
    return segments[segments.length - 1] ?? name;
}

function normalizeKind(kind?: string | null): string {
    return (kind ?? '').replace(/[\s_-]/g, '').toLowerCase();
}

function capitalize(value: string): string {
    if (!value) {
        return value;
    }

    return value[0].toUpperCase() + value.slice(1);
}