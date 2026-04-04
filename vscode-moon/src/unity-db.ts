import * as path from 'path';
import * as vscode from 'vscode';
import Database from 'better-sqlite3';

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
    private db: Database.Database | null = null;

    private stmtMembersByParent: Database.Statement | null = null;
    private stmtClassByName: Database.Statement | null = null;
    private stmtAllClasses: Database.Statement | null = null;
    private stmtSearchByName: Database.Statement | null = null;

    constructor() {}

    open(dbPath: string): boolean {
        try {
            this.db = new Database(dbPath, { readonly: true, fileMustExist: true });
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
