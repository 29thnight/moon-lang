import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';

/**
 * Moon Scripts sidebar TreeView.
 * Shows all .mn files in the workspace grouped by folder.
 */
export class MoonExplorerProvider implements vscode.TreeDataProvider<MoonTreeItem> {

    private _onDidChangeTreeData = new vscode.EventEmitter<MoonTreeItem | undefined>();
    readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

    refresh(): void {
        this._onDidChangeTreeData.fire(undefined);
    }

    getTreeItem(element: MoonTreeItem): vscode.TreeItem {
        return element;
    }

    async getChildren(element?: MoonTreeItem): Promise<MoonTreeItem[]> {
        if (!element) {
            return this.getRootItems();
        }

        if (element.contextValue === 'folder') {
            return element.children || [];
        }

        if (element.contextValue === 'mnFile') {
            return this.getFileStructure(element.resourceUri!.fsPath);
        }

        return [];
    }

    private async getRootItems(): Promise<MoonTreeItem[]> {
        let files: vscode.Uri[] = [];

        // 1. Try workspace
        if (vscode.workspace.workspaceFolders) {
            files = await vscode.workspace.findFiles('**/*.mn', '**/node_modules/**');
        }

        // 2. If no workspace or no files, search from active editor's directory
        if (files.length === 0) {
            const editor = vscode.window.activeTextEditor;
            if (editor) {
                const dir = path.dirname(editor.document.uri.fsPath);
                files = await this.scanDirectory(dir);
            }
        }

        if (files.length === 0) return [];

        // Group by directory
        const groups = new Map<string, vscode.Uri[]>();
        for (const file of files) {
            const dir = path.dirname(file.fsPath);
            const rel = vscode.workspace.asRelativePath(dir);
            if (!groups.has(rel)) groups.set(rel, []);
            groups.get(rel)!.push(file);
        }

        const items: MoonTreeItem[] = [];

        if (groups.size === 1) {
            // Single folder: show files directly
            const [, uris] = [...groups.entries()][0];
            for (const uri of uris.sort((a, b) => a.fsPath.localeCompare(b.fsPath))) {
                items.push(this.createFileItem(uri));
            }
        } else {
            // Multiple folders: group
            for (const [folder, uris] of [...groups.entries()].sort()) {
                const children = uris
                    .sort((a, b) => a.fsPath.localeCompare(b.fsPath))
                    .map(uri => this.createFileItem(uri));

                const folderItem = new MoonTreeItem(
                    folder,
                    vscode.TreeItemCollapsibleState.Expanded
                );
                folderItem.contextValue = 'folder';
                folderItem.iconPath = new vscode.ThemeIcon('folder');
                folderItem.children = children;
                items.push(folderItem);
            }
        }

        return items;
    }

    private createFileItem(uri: vscode.Uri): MoonTreeItem {
        const name = path.basename(uri.fsPath, '.mn');
        const item = new MoonTreeItem(name, vscode.TreeItemCollapsibleState.Collapsed);
        item.contextValue = 'mnFile';
        item.resourceUri = uri;
        item.iconPath = new vscode.ThemeIcon('file-code');
        item.tooltip = vscode.workspace.asRelativePath(uri);
        item.command = {
            command: 'vscode.open',
            title: 'Open',
            arguments: [uri]
        };
        return item;
    }

    private getFileStructure(filePath: string): MoonTreeItem[] {
        try {
            // Prefer open document content (unsaved changes)
            const uri = vscode.Uri.file(filePath);
            const openDoc = vscode.workspace.textDocuments.find(d => d.uri.fsPath === uri.fsPath);
            const text = openDoc ? openDoc.getText() : fs.readFileSync(filePath, 'utf8');
            return this.parseStructure(text);
        } catch {
            return [];
        }
    }

    private async scanDirectory(dir: string): Promise<vscode.Uri[]> {
        const results: vscode.Uri[] = [];
        // Walk up to find project root (.mnproject)
        let root = dir;
        for (let i = 0; i < 10; i++) {
            if (fs.existsSync(path.join(root, '.mnproject'))) break;
            const parent = path.dirname(root);
            if (parent === root) break;
            root = parent;
        }
        // Recursively find .mn files
        this.findMnFiles(root, results);
        return results;
    }

    private findMnFiles(dir: string, results: vscode.Uri[]) {
        try {
            const entries = fs.readdirSync(dir, { withFileTypes: true });
            for (const entry of entries) {
                if (entry.name.startsWith('.') || entry.name === 'node_modules' ||
                    entry.name === 'Library' || entry.name === 'Temp') continue;
                const full = path.join(dir, entry.name);
                if (entry.isDirectory()) {
                    this.findMnFiles(full, results);
                } else if (entry.name.endsWith('.mn')) {
                    results.push(vscode.Uri.file(full));
                }
            }
        } catch {}
    }

    private parseStructure(text: string): MoonTreeItem[] {
        const items: MoonTreeItem[] = [];

        // Component/Asset/Class declaration
        const declMatch = text.match(/\b(component|asset|class|enum)\s+(\w+)/);
        if (declMatch) {
            const declItem = new MoonTreeItem(
                `${declMatch[2]}`,
                vscode.TreeItemCollapsibleState.None
            );
            declItem.iconPath = new vscode.ThemeIcon(
                declMatch[1] === 'enum' ? 'symbol-enum' : 'symbol-class'
            );
            declItem.description = declMatch[1];
            items.push(declItem);
        }

        // Fields: serialize, require, optional, val, var, const, fixed
        const fieldRegex = /\b(serialize|require|optional|child|parent|val|var|const|fixed)\s+(?:var\s+|val\s+)?(\w+)\s*(?::\s*(\w+))?/g;
        let fm;
        while ((fm = fieldRegex.exec(text)) !== null) {
            const fieldItem = new MoonTreeItem(fm[2], vscode.TreeItemCollapsibleState.None);
            fieldItem.iconPath = new vscode.ThemeIcon(
                fm[1] === 'require' || fm[1] === 'optional' ? 'plug' :
                fm[1] === 'const' ? 'symbol-constant' :
                'symbol-field'
            );
            fieldItem.description = `${fm[1]}${fm[3] ? ': ' + fm[3] : ''}`;
            items.push(fieldItem);
        }

        // Functions
        const funcRegex = /\bfunc\s+(\w+)\s*\(([^)]*)\)(?:\s*:\s*(\w+))?/g;
        let funcM;
        while ((funcM = funcRegex.exec(text)) !== null) {
            const funcItem = new MoonTreeItem(funcM[1], vscode.TreeItemCollapsibleState.None);
            funcItem.iconPath = new vscode.ThemeIcon('symbol-method');
            funcItem.description = `(${funcM[2]})${funcM[3] ? ': ' + funcM[3] : ''}`;
            items.push(funcItem);
        }

        // Coroutines
        const corRegex = /\bcoroutine\s+(\w+)\s*\(([^)]*)\)/g;
        let corM;
        while ((corM = corRegex.exec(text)) !== null) {
            const corItem = new MoonTreeItem(corM[1], vscode.TreeItemCollapsibleState.None);
            corItem.iconPath = new vscode.ThemeIcon('sync');
            corItem.description = `coroutine(${corM[2]})`;
            items.push(corItem);
        }

        // Lifecycle blocks
        const lifecycleRegex = /\b(awake|start|update|fixedUpdate|lateUpdate|onEnable|onDisable|onDestroy|onTriggerEnter|onTriggerExit|onCollisionEnter|onCollisionExit)\s*(?:\([^)]*\))?\s*\{/g;
        let lcM;
        while ((lcM = lifecycleRegex.exec(text)) !== null) {
            const lcItem = new MoonTreeItem(lcM[1], vscode.TreeItemCollapsibleState.None);
            lcItem.iconPath = new vscode.ThemeIcon('play');
            lcItem.description = 'lifecycle';
            items.push(lcItem);
        }

        return items;
    }
}

export class MoonTreeItem extends vscode.TreeItem {
    children?: MoonTreeItem[];
}
