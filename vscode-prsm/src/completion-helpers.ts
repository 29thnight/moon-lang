export interface UserSymbol {
    name: string;
    kind: 'component' | 'asset' | 'func' | 'field' | 'coroutine' | 'enum' | 'enumEntry';
    detail: string;
    type?: string;
    params?: string;
}

export function extractUserSymbols(text: string, filePath: string): UserSymbol[] {
    const symbols: UserSymbol[] = [];
    const fileName = filePath.split(/[/\\]/).pop() || '';

    const declMatch = text.match(/\b(component|asset)\s+(\w+)/);
    if (declMatch) {
        symbols.push({
            name: declMatch[2],
            kind: declMatch[1] as 'component' | 'asset',
            detail: `${declMatch[1]} - ${fileName}`,
            type: declMatch[2],
        });
    }

    const enumMatch = text.match(/\benum\s+(\w+)\s*\{([^}]*)\}/);
    if (enumMatch) {
        symbols.push({ name: enumMatch[1], kind: 'enum', detail: `enum - ${fileName}` });
        for (const entry of enumMatch[2].split(',').map(item => item.trim()).filter(Boolean)) {
            const entryName = entry.split('(')[0].trim();
            if (entryName) {
                symbols.push({ name: `${enumMatch[1]}.${entryName}`, kind: 'enumEntry', detail: enumMatch[1] });
            }
        }
    }

    const funcRegex = /\bfunc\s+(\w+)\s*\(([^)]*)\)(?:\s*:\s*(\w+))?/g;
    let funcMatch: RegExpExecArray | null;
    while ((funcMatch = funcRegex.exec(text)) !== null) {
        symbols.push({
            name: funcMatch[1],
            kind: 'func',
            detail: `func(${funcMatch[2]}): ${funcMatch[3] || 'Unit'} - ${fileName}`,
            params: funcMatch[2],
        });
    }

    const coroutineRegex = /\bcoroutine\s+(\w+)\s*\(([^)]*)\)/g;
    let coroutineMatch: RegExpExecArray | null;
    while ((coroutineMatch = coroutineRegex.exec(text)) !== null) {
        symbols.push({
            name: coroutineMatch[1],
            kind: 'coroutine',
            detail: `coroutine(${coroutineMatch[2]}) - ${fileName}`,
            params: coroutineMatch[2],
        });
    }

    const fieldRegex = /\b(serialize|require|optional|child|parent)\s+(\w+)\s*:\s*(\w+)/g;
    let fieldMatch: RegExpExecArray | null;
    while ((fieldMatch = fieldRegex.exec(text)) !== null) {
        symbols.push({
            name: fieldMatch[2],
            kind: 'field',
            detail: `${fieldMatch[1]} ${fieldMatch[3]} - ${fileName}`,
            type: fieldMatch[3],
        });
    }

    const variableRegex = /\b(var|val)\s+(\w+)\s*:\s*(\w+)/g;
    let variableMatch: RegExpExecArray | null;
    while ((variableMatch = variableRegex.exec(text)) !== null) {
        symbols.push({
            name: variableMatch[2],
            kind: 'field',
            detail: `${variableMatch[1]} ${variableMatch[3]} - ${fileName}`,
            type: variableMatch[3],
        });
    }

    return symbols;
}

export function resolveReceiverTypeFromText(text: string, receiver: string): string | undefined {
    const patterns = [
        new RegExp(`\\brequire\\s+${receiver}\\s*:\\s*(\\w+)`),
        new RegExp(`\\boptional\\s+${receiver}\\s*:\\s*(\\w+)`),
        new RegExp(`\\bserialize\\s+${receiver}\\s*:\\s*(\\w+)`),
        new RegExp(`\\b(?:var|val|private|public|protected)\\s+${receiver}\\s*:\\s*(\\w+)`),
        new RegExp(`\\b(?:child|parent)\\s+${receiver}\\s*:\\s*(\\w+)`),
    ];

    for (const pattern of patterns) {
        const match = text.match(pattern);
        if (match) {
            return match[1];
        }
    }

    return undefined;
}

export function extractUsings(text: string, maxLines = 20): string[] {
    const usings: string[] = [];
    for (const line of text.split(/\r?\n/).slice(0, maxLines)) {
        const match = line.match(/^\s*using\s+([\w.]+)/);
        if (match) {
            usings.push(match[1]);
        }
    }
    return usings;
}
