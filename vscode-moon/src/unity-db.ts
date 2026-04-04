import * as fs from 'fs';
import * as path from 'path';
import * as vscode from 'vscode';

type BetterSqliteStatement = {
    get(...params: unknown[]): unknown;
    all(...params: unknown[]): unknown[];
};

type BetterSqliteDatabase = {
    prepare(source: string): BetterSqliteStatement;
    close(): void;
};

type BetterSqliteConstructor = new (
    filename?: string | Buffer,
    options?: {
        readonly?: boolean;
        fileMustExist?: boolean;
        nativeBinding?: string;
    },
) => BetterSqliteDatabase;

interface BetterSqliteRuntime {
    Database: BetterSqliteConstructor;
    nativeBinding?: string;
}

let cachedBetterSqliteRuntime: BetterSqliteRuntime | null = null;

function loadBetterSqliteRuntime(): BetterSqliteRuntime {
    if (cachedBetterSqliteRuntime) {
        return cachedBetterSqliteRuntime;
    }

    try {
        cachedBetterSqliteRuntime = {
            Database: require('better-sqlite3') as BetterSqliteConstructor,
        };
        return cachedBetterSqliteRuntime;
    } catch {
        const vendorRoot = path.resolve(__dirname, 'vendor', 'better-sqlite3');
        const vendorEntry = path.join(vendorRoot, 'lib', 'index.js');
        const nativeBinding = path.join(vendorRoot, 'build', 'Release', 'better_sqlite3.node');

        if (fs.existsSync(vendorEntry) && fs.existsSync(nativeBinding)) {
            cachedBetterSqliteRuntime = {
                Database: require(vendorEntry) as BetterSqliteConstructor,
                nativeBinding,
            };
            return cachedBetterSqliteRuntime;
        }

        throw new Error('better-sqlite3 runtime not found');
    }
}

export interface ApiSymbol {
    id: number;
    parent_id: number | null;
    name: string;
    full_name: string;
    kind: string;
    summary: string;
    signature: string;
    url: string;
}

/**
 * SQLite-backed Unity API database.
 *
 * DB tree structure:
 *   toc (Namespace)
 *     └─ UnityEngine (Symbol)
 *         └─ UnityEngine.X (Namespace)
 *             └─ Classes (Namespace)
 *                 └─ ClassName (Class)
 *                     ├─ propertyName (Property)
 *                     └─ methodName (Property)
 */
export class UnityApiDb {
    private db: BetterSqliteDatabase | null = null;

    private stmtMembersByParent: BetterSqliteStatement | null = null;
    private stmtClassByName: BetterSqliteStatement | null = null;
    private stmtAllClasses: BetterSqliteStatement | null = null;
    private stmtSearchByName: BetterSqliteStatement | null = null;

    constructor() {}

    open(dbPath: string): boolean {
        try {
            const runtime = loadBetterSqliteRuntime();
            this.db = new runtime.Database(dbPath, {
                readonly: true,
                fileMustExist: true,
                nativeBinding: runtime.nativeBinding,
            });
            this.prepareStatements();
            return true;
        } catch (e) {
            console.warn(`Moon: Unity API DB failed to open: ${dbPath}`, e);
            this.db = null;
            return false;
        }
    }

    isOpen(): boolean { return this.db !== null; }

    close() { this.db?.close(); this.db = null; }

    private prepareStatements() {
        if (!this.db) { return; }

        // Get members (Property kind) of a class by class id
        this.stmtMembersByParent = this.db.prepare(`
            SELECT * FROM symbols WHERE parent_id = ? ORDER BY name
        `);

        // Find class by exact name
        this.stmtClassByName = this.db.prepare(`
            SELECT * FROM symbols WHERE name = ? AND kind = 'Class' LIMIT 1
        `);

        // Get all classes (for type completion)
        this.stmtAllClasses = this.db.prepare(`
            SELECT id, name, summary FROM symbols WHERE kind = 'Class' ORDER BY name
        `);

        // Search by name prefix
        this.stmtSearchByName = this.db.prepare(`
            SELECT * FROM symbols WHERE name LIKE ? || '%' LIMIT 50
        `);
    }

    /**
     * Get all members of a type by type name.
     */
    getMembersByTypeName(typeName: string): ApiSymbol[] {
        if (!this.db || !this.stmtClassByName || !this.stmtMembersByParent) { return []; }
        try {
            const cls = this.stmtClassByName.get(typeName) as ApiSymbol | undefined;
            if (!cls) { return []; }
            return this.stmtMembersByParent.all(cls.id) as ApiSymbol[];
        } catch { return []; }
    }

    /**
     * Get all known class names (for type completion).
     */
    getAllClassNames(): { name: string; summary: string }[] {
        if (!this.db || !this.stmtAllClasses) { return []; }
        try {
            return this.stmtAllClasses.all() as { name: string; summary: string }[];
        } catch { return []; }
    }

    /**
     * Check if a type exists.
     */
    hasType(name: string): boolean {
        if (!this.db || !this.stmtClassByName) { return false; }
        try {
            return !!this.stmtClassByName.get(name);
        } catch { return false; }
    }

    /**
     * Search by prefix.
     */
    search(prefix: string): ApiSymbol[] {
        if (!this.db || !this.stmtSearchByName) { return []; }
        try {
            return this.stmtSearchByName.all(prefix) as ApiSymbol[];
        } catch { return []; }
    }
}

/**
 * Resolve the DB path from configuration.
 */
export function resolveDbPath(extensionPath: string): string {
    const config = vscode.workspace.getConfiguration('moon');
    const configPath = config.get<string>('unityApiDbPath', '');
    if (configPath) { return configPath; }

    // Check workspace root
    const folders = vscode.workspace.workspaceFolders;
    if (folders) {
        const wsDb = path.join(folders[0].uri.fsPath, 'unity_api_nodes.db');
        try {
            require('fs').accessSync(wsDb);
            return wsDb;
        } catch { /* not found */ }
    }

    // Default: extension bundled
    return path.join(extensionPath, 'data', 'unity-api.db');
}
