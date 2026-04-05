import * as vscode from 'vscode';
import { UnityApiDb, ApiSymbol, resolveDbPath } from './unity-db';
import { TYPE_MAP, NAMESPACE_TYPES, PRSM_KEYWORDS, PRSM_BUILTINS, UnityMember } from './unity-api';
import { CSharpBridge } from './csharp-bridge';
import { filterCompletions, isTypeBlocked, isMemberBlocked } from './api-filter';
import { extractUsings, extractUserSymbols, resolveReceiverTypeFromText } from './completion-helpers';

/**
 * Hybrid completion provider:
 * 1. Unity API: SQLite DB (primary) → hardcoded fallback
 * 2. User scripts: regex scan of workspace .prsm files
 */
export class PrismCompletionProvider implements vscode.CompletionItemProvider {

    private db: UnityApiDb;
    private dbReady = false;
    private csharp: CSharpBridge;

    // Cache of user-defined symbols
    private userSymbols: Map<string, UserSymbol[]> = new Map();
    private symbolsStale = true;

    constructor(extensionPath: string) {
        this.db = new UnityApiDb();
        this.csharp = new CSharpBridge();
        const dbPath = resolveDbPath(extensionPath);
        this.dbReady = this.db.open(dbPath);
        if (this.dbReady) {
            console.log('PrSM: Unity API DB loaded from', dbPath);
        } else {
            console.log('PrSM: Unity API DB not found at', dbPath, '— using hardcoded fallback');
        }

        vscode.workspace.onDidSaveTextDocument(doc => {
            if (doc.languageId === 'prsm') { this.symbolsStale = true; }
        });
        vscode.workspace.onDidCreateFiles(() => { this.symbolsStale = true; });
        vscode.workspace.onDidDeleteFiles(() => { this.symbolsStale = true; });
    }

    dispose() {
        this.db.close();
    }

    async provideCompletionItems(
        document: vscode.TextDocument,
        position: vscode.Position,
    ): Promise<vscode.CompletionItem[]> {

        const lineText = document.lineAt(position).text;
        const textBefore = lineText.substring(0, position.character);
        const items: vscode.CompletionItem[] = [];

        // After "." or "?." → member completion
        const dotMatch = textBefore.match(/(\w+)(?:\?)?\.(\w*)$/);
        if (dotMatch) {
            const receiver = dotMatch[1];
            items.push(...await this.getMemberCompletions(document, receiver));
            return items;
        }

        // After ": " → type completion
        if (textBefore.match(/:\s*\w*$/)) {
            items.push(...this.getTypeCompletions(document));
            return items;
        }

        // After "<" → generic type
        if (textBefore.match(/<\w*$/)) {
            items.push(...this.getTypeCompletions(document));
            return items;
        }

        // General: keywords + builtins + types + user symbols
        items.push(...this.getKeywordCompletions());
        items.push(...this.getBuiltinCompletions());
        items.push(...this.getTypeCompletions(document));
        items.push(...await this.getUserSymbolCompletions());

        return items;
    }

    // ── Member completions (3-tier: DB → hardcoded → C# extension) ───

    private async getMemberCompletions(document: vscode.TextDocument, receiver: string): Promise<vscode.CompletionItem[]> {
        const typeName = this.resolveReceiverType(document, receiver);
        if (!typeName) { return []; }

        // Tier 1: SQLite DB
        if (this.dbReady) {
            const members = this.db.getMembersByTypeName(typeName);
            if (members.length > 0) {
                return members
                    .filter(m => !m.signature?.includes('[Obsolete'))
                    .filter(m => !isMemberBlocked(typeName, m.name))
                    .map(m => this.dbSymbolToCompletion(m));
            }
        }

        // Tier 2: Hardcoded primitives + Unity core
        const typeInfo = TYPE_MAP[typeName];
        if (typeInfo) {
            return typeInfo.members.map(m => this.hardcodedMemberToCompletion(m, typeName));
        }

        // Tier 3: C# extension (OmniSharp) — query generated .cs files
        try {
            const csharpItems = await this.csharp.getCompletionsFromCSharp(typeName, '', document.uri.fsPath);
            if (csharpItems.length > 0) {
                return filterCompletions(csharpItems);
            }

            const unityItems = await this.csharp.getUnityTypeMembers(typeName, document.uri.fsPath);
            if (unityItems.length > 0) {
                return filterCompletions(unityItems);
            }
        } catch { /* C# extension not available */ }

        // Final fallback: MonoBehaviour inherited
        if (TYPE_MAP['MonoBehaviour']) {
            return TYPE_MAP['MonoBehaviour'].members.map(m => this.hardcodedMemberToCompletion(m, 'MonoBehaviour'));
        }

        return [];
    }

    private resolveReceiverType(document: vscode.TextDocument, receiver: string): string | undefined {
        // Built-in receivers
        const builtinMap: Record<string, string> = {
            'gameObject': 'GameObject',
            'transform': 'Transform',
            'input': 'Input',
            'Time': 'Time',
            'Debug': 'Debug',
            'Physics': 'Physics',
            'Mathf': 'Mathf',
            'Application': 'Application',
            'SceneManager': 'SceneManager',
            'Screen': 'Screen',
            'Cursor': 'Cursor',
            'Resources': 'Resources',
            'PlayerPrefs': 'PlayerPrefs',
        };
        if (builtinMap[receiver]) { return builtinMap[receiver]; }

        // Check if receiver is a type name (static access)
        if (this.dbReady && this.db.hasType(receiver)) { return receiver; }
        if (TYPE_MAP[receiver]) { return receiver; }

        return resolveReceiverTypeFromText(document.getText(), receiver);
    }

    // ── Type completions ─────────────────────────────

    private getTypeCompletions(document: vscode.TextDocument): vscode.CompletionItem[] {
        const items: vscode.CompletionItem[] = [];
        const usings = this.getUsings(document);

        // Primitive types
        for (const t of ['Int', 'Float', 'Double', 'Bool', 'String', 'Long', 'Byte']) {
            const item = new vscode.CompletionItem(t, vscode.CompletionItemKind.TypeParameter);
            item.detail = 'Primitive (prsm)';
            items.push(item);
        }

        // DB types — get all known classes, filtered
        if (this.dbReady) {
            const dbTypes = this.db.getAllClassNames();
            for (const t of dbTypes) {
                if (isTypeBlocked(t.name)) continue;
                const item = new vscode.CompletionItem(t.name, vscode.CompletionItemKind.Class);
                item.detail = 'Class';
                if (t.summary) {
                    item.documentation = t.summary.substring(0, 120);
                }
                items.push(item);
            }
        } else {
            // Fallback: hardcoded
            for (const ns of usings) {
                const types = NAMESPACE_TYPES[ns];
                if (types) {
                    for (const tn of types) {
                        const info = TYPE_MAP[tn];
                        const kind = info?.kind === 'enum' ? vscode.CompletionItemKind.Enum
                            : info?.kind === 'struct' ? vscode.CompletionItemKind.Struct
                            : vscode.CompletionItemKind.Class;
                        const item = new vscode.CompletionItem(tn, kind);
                        item.detail = `${ns}.${tn}`;
                        if (info) { item.documentation = info.description; }
                        items.push(item);
                    }
                }
            }
        }

        return items;
    }

    // ── Keyword / builtin completions ────────────────

    private getKeywordCompletions(): vscode.CompletionItem[] {
        return PRSM_KEYWORDS.map(kw => {
            const item = new vscode.CompletionItem(kw, vscode.CompletionItemKind.Keyword);
            item.detail = 'Keyword (prsm)';
            return item;
        });
    }

    private getBuiltinCompletions(): vscode.CompletionItem[] {
        return PRSM_BUILTINS.map(b => {
            const item = new vscode.CompletionItem(b.name, vscode.CompletionItemKind.Function);
            item.detail = b.prsmOnly ? 'Builtin (prsm)' : 'Builtin';
            const md = new vscode.MarkdownString();
            md.appendCodeblock(`${b.name}${b.params}`, 'prsm');
            md.appendText(b.description);
            item.documentation = md;
            item.insertText = new vscode.SnippetString(`${b.name}($1)`);
            return item;
        });
    }

    // ── User symbol completions ──────────────────────

    private async getUserSymbolCompletions(): Promise<vscode.CompletionItem[]> {
        if (this.symbolsStale) {
            await this.scanWorkspaceSymbols();
            this.symbolsStale = false;
        }

        const items: vscode.CompletionItem[] = [];
        for (const [, symbols] of this.userSymbols) {
            for (const sym of symbols) {
                const kind = sym.kind === 'component' ? vscode.CompletionItemKind.Class
                    : sym.kind === 'asset' ? vscode.CompletionItemKind.Class
                    : sym.kind === 'func' ? vscode.CompletionItemKind.Method
                    : sym.kind === 'field' ? vscode.CompletionItemKind.Field
                    : sym.kind === 'coroutine' ? vscode.CompletionItemKind.Method
                    : sym.kind === 'enum' ? vscode.CompletionItemKind.Enum
                    : sym.kind === 'enumEntry' ? vscode.CompletionItemKind.EnumMember
                    : vscode.CompletionItemKind.Variable;

                const kindLabels: Record<string, string> = {
                    component: 'Component (prsm)', asset: 'Asset (prsm)', func: 'Method (prsm)',
                    field: 'Field (prsm)', coroutine: 'Coroutine (prsm)', enum: 'Enum (prsm)', enumEntry: 'EnumMember (prsm)'
                };
                const item = new vscode.CompletionItem(sym.name, kind);
                item.detail = kindLabels[sym.kind] || sym.kind;
                item.documentation = new vscode.MarkdownString(sym.detail);
                if (sym.params) {
                    item.insertText = new vscode.SnippetString(`${sym.name}($1)`);
                }
                items.push(item);
            }
        }
        return items;
    }

    private async scanWorkspaceSymbols() {
        this.userSymbols.clear();
        const files = await vscode.workspace.findFiles('**/*.prsm', '**/node_modules/**');
        for (const file of files) {
            try {
                const doc = await vscode.workspace.openTextDocument(file);
                const symbols = extractUserSymbols(doc.getText(), file.fsPath);
                this.userSymbols.set(file.fsPath, symbols);
            } catch { /* skip */ }
        }
    }

    private extractSymbols(text: string, filePath: string): UserSymbol[] {
        const symbols: UserSymbol[] = [];
        const fileName = filePath.split(/[/\\]/).pop() || '';

        const declMatch = text.match(/\b(component|asset)\s+(\w+)/);
        if (declMatch) {
            symbols.push({ name: declMatch[2], kind: declMatch[1] as any, detail: `${declMatch[1]} — ${fileName}`, type: declMatch[2] });
        }

        const enumMatch = text.match(/\benum\s+(\w+)\s*\{([^}]*)\}/);
        if (enumMatch) {
            symbols.push({ name: enumMatch[1], kind: 'enum', detail: `enum — ${fileName}` });
            for (const entry of enumMatch[2].split(',').map(e => e.trim()).filter(e => e)) {
                const eName = entry.split('(')[0].trim();
                if (eName) { symbols.push({ name: `${enumMatch[1]}.${eName}`, kind: 'enumEntry', detail: enumMatch[1] }); }
            }
        }

        const funcRegex = /\bfunc\s+(\w+)\s*\(([^)]*)\)(?:\s*:\s*(\w+))?/g;
        let fm;
        while ((fm = funcRegex.exec(text)) !== null) {
            symbols.push({ name: fm[1], kind: 'func', detail: `func(${fm[2]}): ${fm[3] || 'Unit'} — ${fileName}`, params: fm[2] });
        }

        const corRegex = /\bcoroutine\s+(\w+)\s*\(([^)]*)\)/g;
        let cm;
        while ((cm = corRegex.exec(text)) !== null) {
            symbols.push({ name: cm[1], kind: 'coroutine', detail: `coroutine(${cm[2]}) — ${fileName}`, params: cm[2] });
        }

        const fieldRegex = /\b(serialize|require|optional|child|parent)\s+(\w+)\s*:\s*(\w+)/g;
        let sm;
        while ((sm = fieldRegex.exec(text)) !== null) {
            symbols.push({ name: sm[2], kind: 'field', detail: `${sm[1]} ${sm[3]} — ${fileName}`, type: sm[3] });
        }

        const varRegex = /\b(var|val)\s+(\w+)\s*:\s*(\w+)/g;
        let vm;
        while ((vm = varRegex.exec(text)) !== null) {
            symbols.push({ name: vm[2], kind: 'field', detail: `${vm[1]} ${vm[3]} — ${fileName}`, type: vm[3] });
        }

        return symbols;
    }

    // ── DB symbol → CompletionItem ───────────────────

    private dbSymbolToCompletion(sym: ApiSymbol): vscode.CompletionItem {
        const kind = sym.kind === 'Class' ? vscode.CompletionItemKind.Class
            : sym.kind === 'Struct' ? vscode.CompletionItemKind.Struct
            : sym.kind === 'Enum' ? vscode.CompletionItemKind.Enum
            : sym.kind === 'EnumValue' ? vscode.CompletionItemKind.EnumMember
            : sym.kind === 'Method' ? vscode.CompletionItemKind.Method
            : sym.kind === 'Constructor' ? vscode.CompletionItemKind.Constructor
            : sym.kind === 'Property' ? vscode.CompletionItemKind.Property
            : sym.kind === 'Field' ? vscode.CompletionItemKind.Field
            : sym.kind === 'Event' ? vscode.CompletionItemKind.Event
            : sym.kind === 'Message' ? vscode.CompletionItemKind.Event
            : sym.kind === 'Operator' ? vscode.CompletionItemKind.Operator
            : vscode.CompletionItemKind.Variable;

        const item = new vscode.CompletionItem(sym.name, kind);
        item.detail = sym.kind;

        if (sym.summary) {
            const md = new vscode.MarkdownString();
            if (sym.signature) {
                md.appendCodeblock(sym.signature, 'csharp');
            }
            md.appendText(sym.summary);
            if (sym.url) {
                md.appendMarkdown(`\n\n[Unity Docs](https://docs.unity3d.com/6000.3/Documentation/ScriptReference/${sym.url})`);
            }
            item.documentation = md;
        }

        if (sym.kind === 'Method' || sym.kind === 'Constructor') {
            item.insertText = new vscode.SnippetString(`${sym.name}($1)`);
        }

        return item;
    }

    private hardcodedMemberToCompletion(m: UnityMember, typeName: string): vscode.CompletionItem {
        const kind = m.kind === 'method' ? vscode.CompletionItemKind.Method
            : m.kind === 'property' ? vscode.CompletionItemKind.Property
            : m.kind === 'field' ? vscode.CompletionItemKind.Field
            : vscode.CompletionItemKind.Event;

        const kindLabel = m.kind.charAt(0).toUpperCase() + m.kind.slice(1);
        const item = new vscode.CompletionItem(m.name, kind);
        item.detail = kindLabel;

        const md = new vscode.MarkdownString();
        if (m.kind === 'method') {
            md.appendCodeblock(`${m.type} ${m.name}${m.params || '()'}`, 'csharp');
        } else {
            md.appendCodeblock(`${m.type} ${m.name}`, 'csharp');
        }
        md.appendText(`${m.description}\n\n_${typeName}_`);
        item.documentation = md;

        if (m.kind === 'method') {
            item.insertText = new vscode.SnippetString(`${m.name}($1)`);
        }

        return item;
    }

    // ── Utility ──────────────────────────────────────

    private getUsings(document: vscode.TextDocument): string[] {
        return extractUsings(document.getText(), 20);
    }
}

interface UserSymbol {
    name: string;
    kind: 'component' | 'asset' | 'func' | 'field' | 'coroutine' | 'enum' | 'enumEntry';
    detail: string;
    type?: string;
    params?: string;
}
