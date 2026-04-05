import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';

/**
 * PrSM Script Visualizer — shows script structure as an interactive diagram
 * in a right-side split panel (WebView).
 */
export class PrismVisualizer {

    private static panel: vscode.WebviewPanel | undefined;

    static show(document: vscode.TextDocument) {
        const text = document.getText();
        const fileName = path.basename(document.uri.fsPath, '.prsm');
        const structure = parseForVisualization(text, fileName);

        if (PrismVisualizer.panel) {
            PrismVisualizer.panel.webview.html = generateHtml(structure, fileName);
            PrismVisualizer.panel.reveal(vscode.ViewColumn.Beside, true);
        } else {
            PrismVisualizer.panel = vscode.window.createWebviewPanel(
                'prsmVisualizer',
                `PrSM: ${fileName}`,
                { viewColumn: vscode.ViewColumn.Beside, preserveFocus: true },
                { enableScripts: true }
            );
            PrismVisualizer.panel.webview.html = generateHtml(structure, fileName);
            PrismVisualizer.panel.onDidDispose(() => { PrismVisualizer.panel = undefined; });
        }
    }

    static dispose() {
        PrismVisualizer.panel?.dispose();
    }
}

interface VisNode {
    name: string;
    type: 'component' | 'asset' | 'class' | 'enum' | 'field' | 'method' | 'coroutine' | 'lifecycle' | 'require' | 'optional' | 'const';
    detail: string;
    children?: VisNode[];
}

function parseForVisualization(text: string, fileName: string): VisNode {
    const root: VisNode = { name: fileName, type: 'component', detail: '', children: [] };

    // Declaration
    const declMatch = text.match(/\b(component|asset|class|enum)\s+(\w+)(?:\s*:\s*(\w+))?/);
    if (declMatch) {
        root.name = declMatch[2];
        root.type = declMatch[1] as any;
        root.detail = declMatch[3] ? `: ${declMatch[3]}` : '';
    }

    const fields: VisNode[] = [];
    const methods: VisNode[] = [];
    const lifecycle: VisNode[] = [];

    // Fields
    const fieldRegex = /\b(serialize|require|optional|child|parent|val|var|const|fixed)\s+(?:var\s+|val\s+)?(\w+)\s*(?::\s*(\w+[^=\n]*))?/g;
    let fm;
    while ((fm = fieldRegex.exec(text)) !== null) {
        const type = fm[1] === 'require' ? 'require' :
            fm[1] === 'optional' ? 'optional' :
            fm[1] === 'const' ? 'const' : 'field';
        fields.push({
            name: fm[2],
            type,
            detail: `${fm[1]}${fm[3] ? ': ' + fm[3].trim() : ''}`
        });
    }

    // Functions
    const funcRegex = /\bfunc\s+(\w+)\s*\(([^)]*)\)(?:\s*:\s*(\w+))?/g;
    let funcM;
    while ((funcM = funcRegex.exec(text)) !== null) {
        methods.push({
            name: funcM[1],
            type: 'method',
            detail: `(${funcM[2]})${funcM[3] ? ': ' + funcM[3] : ''}`
        });
    }

    // Coroutines
    const corRegex = /\bcoroutine\s+(\w+)\s*\(([^)]*)\)/g;
    let corM;
    while ((corM = corRegex.exec(text)) !== null) {
        methods.push({
            name: corM[1],
            type: 'coroutine',
            detail: `(${corM[2]})`
        });
    }

    // Lifecycle
    const lcRegex = /\b(awake|start|update|fixedUpdate|lateUpdate|onEnable|onDisable|onDestroy|onTriggerEnter|onTriggerExit|onCollisionEnter|onCollisionExit)\s*(?:\([^)]*\))?\s*\{/g;
    let lcM;
    while ((lcM = lcRegex.exec(text)) !== null) {
        lifecycle.push({ name: lcM[1], type: 'lifecycle', detail: '' });
    }

    root.children = [...fields, ...lifecycle, ...methods];
    return root;
}

function generateHtml(root: VisNode, fileName: string): string {
    const typeColors: Record<string, string> = {
        component: '#16baac',
        asset: '#56a8f5',
        class: '#bcbec4',
        enum: '#b3ae60',
        field: '#c77dbb',
        method: '#57aaf7',
        coroutine: '#6aab73',
        lifecycle: '#cf8e6d',
        require: '#f5d76e',
        optional: '#8a8a8a',
        const: '#2aacb8',
    };

    const typeIcons: Record<string, string> = {
        component: '&#9670;',  // ◆
        asset: '&#9671;',      // ◇
        class: '&#9632;',      // ■
        enum: '&#9650;',       // ▲
        field: '&#9679;',      // ●
        method: '&#9654;',     // ▶
        coroutine: '&#8635;',  // ↻
        lifecycle: '&#9889;',  // ⚡
        require: '&#9733;',    // ★
        optional: '&#9734;',   // ☆
        const: '&#9830;',      // ◆
    };

    const childrenHtml = (root.children || []).map(child => {
        const color = typeColors[child.type] || '#bcbec4';
        const icon = typeIcons[child.type] || '●';
        return `
            <div class="node" style="border-left: 3px solid ${color};">
                <span class="icon" style="color: ${color};">${icon}</span>
                <span class="name">${child.name}</span>
                <span class="detail">${child.detail}</span>
                <span class="badge" style="background: ${color}20; color: ${color};">${child.type}</span>
            </div>`;
    }).join('\n');

    const rootColor = typeColors[root.type] || '#16baac';

    return `<!DOCTYPE html>
<html>
<head>
<meta charset="UTF-8">
<style>
    body {
        background: #1e1e1e;
        color: #bcbec4;
        font-family: 'Segoe UI', sans-serif;
        padding: 16px;
        margin: 0;
    }
    .header {
        display: flex;
        align-items: center;
        gap: 12px;
        margin-bottom: 20px;
        padding-bottom: 12px;
        border-bottom: 1px solid #333;
    }
    .header .type-badge {
        background: ${rootColor}20;
        color: ${rootColor};
        padding: 4px 10px;
        border-radius: 4px;
        font-size: 12px;
        font-weight: 600;
        text-transform: uppercase;
    }
    .header .title {
        font-size: 20px;
        font-weight: 700;
        color: ${rootColor};
    }
    .header .extends {
        color: #6e6a8a;
        font-size: 14px;
    }
    .section {
        margin-bottom: 16px;
    }
    .section-title {
        font-size: 11px;
        text-transform: uppercase;
        letter-spacing: 1.2px;
        color: #6e6a8a;
        margin-bottom: 8px;
        padding-left: 4px;
    }
    .node {
        display: flex;
        align-items: center;
        gap: 8px;
        padding: 6px 12px;
        margin: 2px 0;
        border-radius: 4px;
        background: #252526;
        cursor: default;
        transition: background 0.15s;
    }
    .node:hover {
        background: #2a2d2e;
    }
    .icon {
        font-size: 14px;
        width: 18px;
        text-align: center;
    }
    .name {
        font-weight: 600;
        font-size: 13px;
    }
    .detail {
        color: #6e6a8a;
        font-size: 12px;
        flex: 1;
    }
    .badge {
        font-size: 10px;
        padding: 2px 6px;
        border-radius: 3px;
        font-weight: 500;
    }
    .connector {
        width: 20px;
        border-left: 2px solid #333;
        margin-left: 8px;
    }
    .group {
        margin-left: 4px;
    }
</style>
</head>
<body>
    <div class="header">
        <span class="type-badge">${root.type}</span>
        <span class="title">${root.name}</span>
        <span class="extends">${root.detail}</span>
    </div>

    ${categorize(root.children || [], typeColors, typeIcons)}
</body>
</html>`;
}

function categorize(nodes: VisNode[], colors: Record<string, string>, icons: Record<string, string>): string {
    const groups: Record<string, VisNode[]> = {
        'Dependencies': [],
        'Fields': [],
        'Lifecycle': [],
        'Methods': [],
    };

    for (const n of nodes) {
        if (n.type === 'require' || n.type === 'optional') groups['Dependencies'].push(n);
        else if (n.type === 'field' || n.type === 'const') groups['Fields'].push(n);
        else if (n.type === 'lifecycle') groups['Lifecycle'].push(n);
        else if (n.type === 'method' || n.type === 'coroutine') groups['Methods'].push(n);
        else groups['Fields'].push(n);
    }

    let html = '';
    for (const [title, items] of Object.entries(groups)) {
        if (items.length === 0) continue;
        html += `<div class="section">`;
        html += `<div class="section-title">${title} (${items.length})</div>`;
        html += `<div class="group">`;
        for (const item of items) {
            const color = colors[item.type] || '#bcbec4';
            const icon = icons[item.type] || '●';
            html += `
                <div class="node" style="border-left: 3px solid ${color};">
                    <span class="icon" style="color: ${color};">${icon}</span>
                    <span class="name">${item.name}</span>
                    <span class="detail">${item.detail}</span>
                    <span class="badge" style="background: ${color}20; color: ${color};">${item.type}</span>
                </div>`;
        }
        html += `</div></div>`;
    }
    return html;
}
