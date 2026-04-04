import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';

let unityTypes: Set<string> | null = null;
let _extensionPath: string = '';

// Core Unity types that always appear in game scripts
const BUILTIN_UNITY_TYPES = new Set([
    // MonoBehaviour / Component hierarchy
    'MonoBehaviour', 'ScriptableObject', 'Component', 'Behaviour', 'Object',
    // Core types
    'GameObject', 'Transform', 'RectTransform',
    // Physics
    'Rigidbody', 'Rigidbody2D', 'Collider', 'Collider2D',
    'BoxCollider', 'SphereCollider', 'CapsuleCollider', 'MeshCollider',
    'BoxCollider2D', 'CircleCollider2D', 'PolygonCollider2D',
    'Collision', 'Collision2D', 'ContactPoint', 'ContactPoint2D',
    'CharacterController', 'Joint', 'HingeJoint',
    // Rendering
    'Renderer', 'MeshRenderer', 'SkinnedMeshRenderer', 'SpriteRenderer',
    'Camera', 'Light', 'Material', 'Shader', 'Texture', 'Texture2D',
    'Sprite', 'Mesh', 'RenderTexture',
    // UI
    'Canvas', 'CanvasGroup', 'Image', 'Text', 'Button', 'Slider',
    'Toggle', 'InputField', 'Dropdown', 'ScrollRect', 'RawImage',
    'TextMeshPro', 'TextMeshProUGUI', 'TMP_Text', 'TMP_InputField',
    // Audio
    'AudioSource', 'AudioClip', 'AudioListener', 'AudioMixer',
    // Animation
    'Animator', 'Animation', 'AnimationClip', 'RuntimeAnimatorController',
    // Navigation
    'NavMeshAgent', 'NavMeshObstacle',
    // Math / Structs
    'Vector2', 'Vector3', 'Vector4', 'Quaternion', 'Matrix4x4',
    'Color', 'Color32', 'Rect', 'Bounds', 'Ray', 'RaycastHit', 'RaycastHit2D',
    // Input
    'Input', 'KeyCode', 'Touch',
    // Scene / App
    'SceneManager', 'Scene', 'Application',
    // Events
    'UnityEvent', 'UnityAction',
    // Particle
    'ParticleSystem',
    // Misc
    'Coroutine', 'WaitForSeconds', 'WaitForFixedUpdate', 'WaitUntil', 'WaitWhile',
    'Debug', 'Mathf', 'Time', 'Random', 'PlayerPrefs', 'Resources',
    'LayerMask', 'Physics', 'Physics2D', 'Gizmos',
]);

function getUnityTypes(): Set<string> {
    if (unityTypes !== null) return unityTypes;
    unityTypes = new Set(BUILTIN_UNITY_TYPES);
    // Load full type list from bundled JSON (no native module needed)
    try {
        const candidates: string[] = [];
        if (_extensionPath) candidates.push(path.join(_extensionPath, 'data', 'unity-types.json'));
        const ext = vscode.extensions.getExtension('moon-lang.moon-lang');
        if (ext) candidates.push(path.join(ext.extensionPath, 'data', 'unity-types.json'));

        for (const jsonPath of candidates) {
            if (fs.existsSync(jsonPath)) {
                const names: string[] = JSON.parse(fs.readFileSync(jsonPath, 'utf8'));
                names.forEach(n => unityTypes!.add(n));
                break;
            }
        }
    } catch {}
    return unityTypes;
}

export class MoonGraphView {
    private static panel: vscode.WebviewPanel | undefined;
    static async show(context: vscode.ExtensionContext) {
        _extensionPath = context.extensionPath;
        const graph = await buildGraph();
        if (MoonGraphView.panel) {
            MoonGraphView.panel.webview.html = generateGraphHtml(graph);
            MoonGraphView.panel.reveal(vscode.ViewColumn.Beside, true);
        } else {
            MoonGraphView.panel = vscode.window.createWebviewPanel(
                'moonGraphView', 'Moon: Graph View',
                { viewColumn: vscode.ViewColumn.Beside, preserveFocus: true },
                { enableScripts: true }
            );
            MoonGraphView.panel.webview.html = generateGraphHtml(graph);
            MoonGraphView.panel.onDidDispose(() => { MoonGraphView.panel = undefined; });
            MoonGraphView.panel.webview.onDidReceiveMessage(async msg => {
                await MoonGraphView.handleMessage(msg);
            });
        }
    }

    static async handleMessage(msg: any) {
        if ((msg.type === 'openFile' || msg.type === 'visualize') && msg.file) {
            const uri = await MoonGraphView.resolveFile(msg.file);
            if (!uri) {
                vscode.window.showWarningMessage(`File not found: ${msg.file}`);
                return;
            }
            const doc = await vscode.workspace.openTextDocument(uri);
            if (msg.type === 'openFile') {
                await vscode.window.showTextDocument(doc, vscode.ViewColumn.One);
            } else {
                const { MoonVisualizer } = require('./visualizer');
                MoonVisualizer.show(doc);
            }
        }
    }

    /** Resolve a file path (absolute or relative) to a URI */
    private static async resolveFile(filePath: string): Promise<vscode.Uri | null> {
        // 1. Already absolute and exists
        if (path.isAbsolute(filePath) && fs.existsSync(filePath)) {
            return vscode.Uri.file(filePath);
        }
        // 2. Workspace search by basename
        const basename = path.basename(filePath);
        if (vscode.workspace.workspaceFolders) {
            const files = await vscode.workspace.findFiles('**/' + basename);
            if (files.length > 0) return files[0];
        }
        // 3. Walk up from active editor to find project root, then resolve
        const editor = vscode.window.activeTextEditor;
        if (editor) {
            let dir = path.dirname(editor.document.uri.fsPath);
            for (let i = 0; i < 10; i++) {
                const candidate = path.join(dir, filePath);
                if (fs.existsSync(candidate)) return vscode.Uri.file(candidate);
                const byName = path.join(dir, basename);
                if (fs.existsSync(byName)) return vscode.Uri.file(byName);
                if (fs.existsSync(path.join(dir, '.mnproject'))) break;
                const parent = path.dirname(dir);
                if (parent === dir) break;
                dir = parent;
            }
        }
        // 4. Scan for .mn files
        const editorDir = editor ? path.dirname(editor.document.uri.fsPath) : '.';
        const mnFiles = findMnSync(editorDir);
        const match = mnFiles.find(f => path.basename(f) === basename);
        if (match) return vscode.Uri.file(match);
        return null;
    }
}

interface GNode {
    id: string; name: string; file: string;
    type: 'component' | 'asset' | 'class' | 'enum' | 'unity' | 'external';
    usings: string[]; // namespaces this file uses
    x?: number; y?: number;
}
interface GEdge {
    from: string; to: string; label: string;
    type: 'uses' | 'used_by' | 'inherits' | 'require';
    style: 'solid' | 'dash';
}
interface Graph { nodes: GNode[]; edges: GEdge[]; }

/** Parse a single .mn file into a node + edges */
function parseFile(file: vscode.Uri, files?: vscode.Uri[]): { node: GNode; edges: GEdge[]; text: string } | null {
    const openDoc = vscode.workspace.textDocuments.find(d => d.uri.fsPath === file.fsPath);
    const text = openDoc ? openDoc.getText() : fs.readFileSync(file.fsPath, 'utf8');
    const relPath = vscode.workspace.asRelativePath(file) || path.basename(file.fsPath);

    const declMatch = text.match(/\b(component|asset|class|enum)\s+(\w+)(?:\s*:\s*(\w+))?/);
    if (!declMatch) return null;

    const nodeName = declMatch[2];
    const nodeType = declMatch[1] as any;
    const baseClass = declMatch[3];

    const usings: string[] = [];
    const usingRegex = /\busing\s+([\w.]+)/g;
    let um;
    while ((um = usingRegex.exec(text)) !== null) usings.push(um[1]);

    const node: GNode = { id: nodeName, name: nodeName, file: relPath, type: nodeType, usings };
    const edges: GEdge[] = [];

    if (baseClass) {
        edges.push({ from: nodeName, to: baseClass, label: 'inherits', type: 'inherits', style: 'solid' });
    }

    const depRegex = /\b(require|optional|child|parent)\s+\w+\s*:\s*(\w+)/g;
    let dm;
    while ((dm = depRegex.exec(text)) !== null) {
        edges.push({ from: nodeName, to: dm[2], label: dm[1], type: 'require', style: 'solid' });
    }

    return { node, edges, text };
}

async function buildGraph(): Promise<Graph> {
    const nodes: GNode[] = [];
    const edges: GEdge[] = [];
    const knownIds = new Set<string>();
    const fileTexts = new Map<string, string>(); // nodeName → source text

    // Collect all .mn files in workspace
    let allFiles: vscode.Uri[] = [];
    if (vscode.workspace.workspaceFolders)
        allFiles = await vscode.workspace.findFiles('**/*.mn', '**/node_modules/**');
    if (allFiles.length === 0) {
        const editor = vscode.window.activeTextEditor;
        if (editor) findMnSync(path.dirname(editor.document.uri.fsPath)).forEach(f => allFiles.push(vscode.Uri.file(f)));
    }

    // Parse all files into a lookup map
    const allParsed = new Map<string, { node: GNode; edges: GEdge[]; text: string; file: vscode.Uri }>();
    for (const file of allFiles) {
        const result = parseFile(file);
        if (result) allParsed.set(result.node.name, { ...result, file });
    }

    // Determine the focus file (currently active editor)
    const activeEditor = vscode.window.activeTextEditor;
    let focusName: string | null = null;
    if (activeEditor && activeEditor.document.fileName.endsWith('.mn')) {
        const text = activeEditor.document.getText();
        const m = text.match(/\b(component|asset|class|enum)\s+(\w+)/);
        if (m) focusName = m[2];
    }

    // If no .mn file is active, fall back to showing all
    if (!focusName || !allParsed.has(focusName)) {
        // Fallback: full graph
        for (const [, parsed] of allParsed) {
            nodes.push(parsed.node);
            edges.push(...parsed.edges);
            knownIds.add(parsed.node.name);
            fileTexts.set(parsed.node.name, parsed.text);
        }
    } else {
        // Outgoing/upward only: focus file's dependencies, and their dependencies, etc.
        // No incoming — children don't appear unless the focus file references them.
        const connected = new Set<string>();
        const queue: string[] = [focusName];
        connected.add(focusName);

        // BFS upward: follow outgoing edges (inherits, require, uses) recursively
        while (queue.length > 0) {
            const current = queue.shift()!;
            const parsed = allParsed.get(current);
            if (!parsed) continue;

            // Types referenced via edges (inherits, require, optional, etc.)
            for (const edge of parsed.edges) {
                if (!connected.has(edge.to)) {
                    connected.add(edge.to);
                    if (allParsed.has(edge.to)) queue.push(edge.to);
                }
            }

            // Types referenced in source text (only for the focus file itself)
            if (current === focusName) {
                for (const [otherName] of allParsed) {
                    if (connected.has(otherName)) continue;
                    const refRegex = new RegExp('\\b' + otherName + '\\b');
                    if (refRegex.test(parsed.text)) {
                        connected.add(otherName);
                    }
                }
            }
        }

        // Add connected nodes and their upward edges
        for (const name of connected) {
            const parsed = allParsed.get(name);
            if (parsed) {
                nodes.push(parsed.node);
                knownIds.add(name);
                fileTexts.set(name, parsed.text);
                // Include edges whose targets are also in the connected set
                for (const edge of parsed.edges) {
                    if (connected.has(edge.to)) {
                        edges.push(edge);
                    }
                }
            }
        }
    }

    // Cross-file "uses" relationships (only among included nodes)
    for (const node of nodes) {
        const text = fileTexts.get(node.name);
        if (!text) continue;
        for (const otherNode of nodes) {
            if (otherNode.name === node.name) continue;
            const refRegex = new RegExp('\\b' + otherNode.name + '\\b');
            if (refRegex.test(text) && !edges.some(e => e.from === node.name && e.to === otherNode.name)) {
                edges.push({ from: node.name, to: otherNode.name, label: 'uses', type: 'uses', style: 'dash' });
            }
        }
    }

    // Add external/unity nodes for unresolved edge targets
    for (const edge of edges) {
        if (!knownIds.has(edge.to)) {
            const t = getUnityTypes().has(edge.to) ? 'unity' : 'external';
            nodes.push({ id: edge.to, name: edge.to, file: '', type: t as any, usings: [] });
            knownIds.add(edge.to);
        }
    }

    layoutNodes(nodes, edges);
    return { nodes, edges };
}

function layoutNodes(nodes: GNode[], edges: GEdge[]) {
    // Simple: internals left, externals right
    const internals = nodes.filter(n => n.type !== 'external' && n.type !== 'unity');
    const externals = nodes.filter(n => n.type === 'external' || n.type === 'unity');
    let y = 60;
    for (const n of internals) { n.x = 80; n.y = y; y += 80; }
    y = 60;
    for (const n of externals) { n.x = 500; n.y = y; y += 80; }
}

function findMnSync(dir: string): string[] {
    const results: string[] = [];
    let root = dir;
    for (let i = 0; i < 10; i++) {
        if (fs.existsSync(path.join(root, '.mnproject'))) break;
        const p = path.dirname(root); if (p === root) break; root = p;
    }
    (function w(d: string) {
        try { for (const e of fs.readdirSync(d, { withFileTypes: true })) {
            if (e.name.startsWith('.') || e.name === 'node_modules' || e.name === 'Library') continue;
            const f = path.join(d, e.name);
            if (e.isDirectory()) w(f); else if (e.name.endsWith('.mn')) results.push(f);
        }} catch {}
    })(root);
    return results;
}

function generateGraphHtml(graph: Graph): string {
    return `<!DOCTYPE html>
<html><head><meta charset="UTF-8">
<style>
*{margin:0;padding:0;box-sizing:border-box}
body{background:#1a1a1e;overflow:hidden;font-family:'Segoe UI',sans-serif;color:#bcbec4}
#toolbar{position:fixed;top:0;left:0;right:0;height:36px;background:#252528;border-bottom:1px solid #333;display:flex;align-items:center;padding:0 12px;gap:8px;z-index:100;font-size:12px}
#toolbar .title{color:#6e6a8a;font-weight:600;text-transform:uppercase;letter-spacing:1px}
#toolbar button{background:#333;border:1px solid #444;color:#bcbec4;padding:3px 10px;border-radius:3px;cursor:pointer;font-size:11px}
#toolbar button:hover{background:#444}
#toolbar input{background:#1e1e1e;border:1px solid #444;color:#bcbec4;padding:3px 8px;border-radius:3px;font-size:11px;width:180px}
#canvas{position:absolute;top:36px;left:0;right:0;bottom:0}
svg{width:100%;height:100%}
.node-group{cursor:grab}
.node-group:active{cursor:grabbing}
.node-rect{rx:6;ry:6;stroke-width:2;filter:drop-shadow(0 2px 4px rgba(0,0,0,0.3))}
.node-title{font-size:13px;font-weight:700}
.node-badge{font-size:9px;text-transform:uppercase;letter-spacing:0.5px}
.node-chevron{font-size:11px;cursor:pointer;fill:#6e6a8a}
.edge-line{fill:none;stroke-width:2}
.edge-dash{stroke-dasharray:8 4}
.edge-label{font-size:10px;text-anchor:middle;dominant-baseline:central;font-weight:600}
.edge-label-bg{rx:4;ry:4;stroke-width:1.5}
#contextMenu{display:none;position:fixed;background:#2b2d30;border:1px solid #444;border-radius:4px;padding:4px 0;z-index:200;min-width:180px}
#contextMenu div{padding:5px 16px;cursor:pointer;font-size:12px}
#contextMenu div:hover{background:#37393e}
</style></head><body>
<div id="toolbar">
    <span class="title">Moon Graph</span>
    <input id="search" type="text" placeholder="Search..." oninput="onSearch(this.value)">
    <button onclick="fitAll()">Fit All</button>
    <button onclick="resetView()">Reset</button>
</div>
<div id="canvas"><svg id="svg">
    <defs>
        <marker id="ah" markerWidth="8" markerHeight="6" refX="8" refY="3" orient="auto"><polygon points="0 0,8 3,0 6" fill="#6e6a8a"/></marker>
        <marker id="ah-inh" markerWidth="8" markerHeight="6" refX="8" refY="3" orient="auto"><polygon points="0 0,8 3,0 6" fill="#57aaf7"/></marker>
        <marker id="ah-req" markerWidth="8" markerHeight="6" refX="8" refY="3" orient="auto"><polygon points="0 0,8 3,0 6" fill="#f5d76e"/></marker>
        <marker id="ah-use" markerWidth="8" markerHeight="6" refX="8" refY="3" orient="auto"><polygon points="0 0,8 3,0 6" fill="#16baac"/></marker>
    </defs>
    <g id="viewport"><g id="edges-layer"></g><g id="nodes-layer"></g></g>
</svg></div>
<div id="contextMenu"></div>

<script>
const nodes=${JSON.stringify(graph.nodes)};
const edges=${JSON.stringify(graph.edges)};
const vscodeApi=acquireVsCodeApi();
const NW=220,NH=48;

const typeColors={
    component:{fill:'#1e2a2a',stroke:'#16baac',text:'#16baac'},
    asset:{fill:'#1e2433',stroke:'#56a8f5',text:'#56a8f5'},
    class:{fill:'#222225',stroke:'#8a8a8a',text:'#bcbec4'},
    enum:{fill:'#2a2820',stroke:'#b3ae60',text:'#b3ae60'},
    unity:{fill:'#1e2025',stroke:'#7b68ee',text:'#9b8afb'},
    external:{fill:'#1a2020',stroke:'#3a5a5a',text:'#5a8a8a'},
};
const edgeColors={inherits:'#57aaf7',require:'#f5d76e',uses:'#16baac',used_by:'#16baac'};

const svg=document.getElementById('svg'),viewport=document.getElementById('viewport');
const nodesLayer=document.getElementById('nodes-layer'),edgesLayer=document.getElementById('edges-layer');
let scale=1,tx=0,ty=0,dragging=null,dox=0,doy=0,panning=false,psx=0,psy=0,ptx=0,pty=0;
let searchTerm='';

function render(){
    nodesLayer.innerHTML='';edgesLayer.innerHTML='';

    // Edges
    for(const e of edges){
        const fn=nodes.find(n=>n.id===e.from),tn=nodes.find(n=>n.id===e.to);
        if(!fn||!tn)continue;

        // Connection points on node borders
        const fCx=fn.x+NW/2, fCy=fn.y+NH/2;
        const tCx=tn.x+NW/2, tCy=tn.y+NH/2;
        const dx=tCx-fCx, dy=tCy-fCy;

        let x1,y1,x2,y2;

        // Start point: edge of source node
        if(Math.abs(dx)>Math.abs(dy)){
            // Horizontal dominant
            x1=dx>0?fn.x+NW:fn.x; y1=fCy;
            x2=dx>0?tn.x:tn.x+NW; y2=tCy;
        } else {
            // Vertical dominant
            x1=fCx; y1=dy>0?fn.y+NH:fn.y;
            x2=tCx; y2=dy>0?tn.y:tn.y+NH;
        }

        const color=edgeColors[e.type]||'#555';
        const marker=e.type==='inherits'?'url(#ah-inh)':e.type==='require'?'url(#ah-req)':e.type==='uses'?'url(#ah-use)':'url(#ah)';
        const isDash=e.style==='dash';

        let pathD;
        if(Math.abs(y1-y2)<3 || Math.abs(x1-x2)<3){
            // Straight line
            pathD='M '+x1+' '+y1+' L '+x2+' '+y2;
        } else {
            // Single bend: from horizontal exit → vertical to target
            if(Math.abs(dx)>=Math.abs(dy)){
                pathD='M '+x1+' '+y1+' L '+x2+' '+y1+' L '+x2+' '+y2;
            } else {
                pathD='M '+x1+' '+y1+' L '+x1+' '+y2+' L '+x2+' '+y2;
            }
        }

        const p=se('path');p.setAttribute('d',pathD);p.setAttribute('stroke',color);
        p.setAttribute('class','edge-line'+(isDash?' edge-dash':''));
        p.setAttribute('marker-end',marker);
        edgesLayer.appendChild(p);

        // Label: always sits on an actual path segment
        // Compute the segments of the path, pick the longer one, place label at its midpoint
        let lx,ly;
        if(Math.abs(y1-y2)<3){
            // Horizontal straight line
            lx=(x1+x2)/2; ly=y1;
        } else if(Math.abs(x1-x2)<3){
            // Vertical straight line
            lx=x1; ly=(y1+y2)/2;
        } else if(Math.abs(dx)>=Math.abs(dy)){
            // Bent: horizontal (x1,y1)→(x2,y1) then vertical (x2,y1)→(x2,y2)
            const hLen=Math.abs(x2-x1), vLen=Math.abs(y2-y1);
            if(hLen>=vLen){ lx=(x1+x2)/2; ly=y1; }
            else { lx=x2; ly=(y1+y2)/2; }
        } else {
            // Bent: vertical (x1,y1)→(x1,y2) then horizontal (x1,y2)→(x2,y2)
            const vLen=Math.abs(y2-y1), hLen=Math.abs(x2-x1);
            if(vLen>=hLen){ lx=x1; ly=(y1+y2)/2; }
            else { lx=(x1+x2)/2; ly=y2; }
        }
        const lw=e.label.length*6.5+14, lh=20;
        const bg=se('rect');
        bg.setAttribute('x',String(lx-lw/2));bg.setAttribute('y',String(ly-lh/2));
        bg.setAttribute('width',String(lw));bg.setAttribute('height',String(lh));
        bg.setAttribute('class','edge-label-bg');
        bg.setAttribute('fill','#1e1e22');bg.setAttribute('stroke',color);
        edgesLayer.appendChild(bg);
        const lb=se('text');lb.setAttribute('x',String(lx));lb.setAttribute('y',String(ly));
        lb.setAttribute('class','edge-label');lb.setAttribute('fill',color);lb.textContent=e.label;
        edgesLayer.appendChild(lb);
    }

    // Nodes (file-level, compact)
    for(const n of nodes){
        const g=se('g');g.setAttribute('class','node-group');
        g.setAttribute('transform','translate('+n.x+','+n.y+')');
        g.dataset.nodeId=n.id;

        if(searchTerm&&!n.name.toLowerCase().includes(searchTerm))g.setAttribute('opacity','0.2');

        const c=typeColors[n.type]||typeColors.external;

        const r=se('rect');r.setAttribute('width',String(NW));r.setAttribute('height',String(NH));
        r.setAttribute('fill',c.fill);r.setAttribute('stroke',c.stroke);r.setAttribute('class','node-rect');
        if(searchTerm&&n.name.toLowerCase().includes(searchTerm))r.setAttribute('stroke-width','4');
        g.appendChild(r);

        // Badge
        const badge=n.type==='unity'?'UNITY':n.type.toUpperCase();
        const b=se('text');b.setAttribute('x','10');b.setAttribute('y','16');
        b.setAttribute('class','node-badge');b.setAttribute('fill',c.stroke);b.textContent=badge;
        g.appendChild(b);

        // Name
        const t=se('text');t.setAttribute('x','10');t.setAttribute('y','34');
        t.setAttribute('class','node-title');t.setAttribute('fill',c.text);t.textContent=n.name;
        g.appendChild(t);

        // Chevron (right side) for context menu hint
        const ch=se('text');ch.setAttribute('x',String(NW-18));ch.setAttribute('y','30');
        ch.setAttribute('class','node-chevron');ch.textContent='>';
        g.appendChild(ch);

        g.addEventListener('mousedown',ev=>{ev.stopPropagation();dragging=n;dox=(ev.clientX-tx)/scale-n.x;doy=(ev.clientY-ty)/scale-n.y});
        g.addEventListener('contextmenu',ev=>{ev.preventDefault();ev.stopPropagation();showCtx(ev.clientX,ev.clientY,n)});
        g.addEventListener('dblclick',ev=>{ev.stopPropagation();if(n.file){vscodeApi.postMessage({type:'openFile',file:n.file})}});
        nodesLayer.appendChild(g);
    }
    viewport.setAttribute('transform','translate('+tx+','+ty+') scale('+scale+')');
}

function se(tag){return document.createElementNS('http://www.w3.org/2000/svg',tag)}

// Context menu
const ctx=document.getElementById('contextMenu');
function showCtx(x,y,node){
    ctx.innerHTML='';ctx.style.display='block';ctx.style.left=x+'px';ctx.style.top=y+'px';
    if(node.file){
        addCtx('Open File',()=>{vscodeApi.postMessage({type:'openFile',file:node.file})});
        addCtx('Visualize Structure',()=>{vscodeApi.postMessage({type:'visualize',file:node.file})});
    }
    addCtx('Focus',()=>{tx=svg.clientWidth/2-node.x*scale-NW/2*scale;ty=svg.clientHeight/2-node.y*scale-NH/2*scale;viewport.setAttribute('transform','translate('+tx+','+ty+') scale('+scale+')')});
    addCtx('Show Connections',()=>{
        const conn=new Set([node.id]);
        edges.forEach(e=>{if(e.from===node.id)conn.add(e.to);if(e.to===node.id)conn.add(e.from)});
        document.querySelectorAll('.node-group').forEach(g=>{g.setAttribute('opacity',conn.has(g.dataset.nodeId)?'1':'0.15')});
    });
}
function addCtx(label,fn){const d=document.createElement('div');d.textContent=label;d.onclick=()=>{ctx.style.display='none';fn()};ctx.appendChild(d)}
document.addEventListener('click',()=>{ctx.style.display='none'});

function onSearch(v){searchTerm=v.toLowerCase();render()}

svg.addEventListener('mousedown',e=>{if(!dragging){panning=true;psx=e.clientX;psy=e.clientY;ptx=tx;pty=ty}});
svg.addEventListener('mousemove',e=>{
    if(dragging){dragging.x=(e.clientX-tx)/scale-dox;dragging.y=(e.clientY-ty)/scale-doy;render()}
    else if(panning){tx=ptx+(e.clientX-psx);ty=pty+(e.clientY-psy);viewport.setAttribute('transform','translate('+tx+','+ty+') scale('+scale+')')}
});
svg.addEventListener('mouseup',()=>{dragging=null;panning=false});
svg.addEventListener('wheel',e=>{e.preventDefault();const d=e.deltaY>0?0.9:1.1;const ns=Math.max(0.2,Math.min(3,scale*d));tx=e.clientX-(e.clientX-tx)*(ns/scale);ty=e.clientY-(e.clientY-ty)*(ns/scale);scale=ns;viewport.setAttribute('transform','translate('+tx+','+ty+') scale('+scale+')')});

function resetView(){scale=1;tx=0;ty=0;searchTerm='';document.getElementById('search').value='';render()}
function fitAll(){
    if(!nodes.length)return;
    let x0=Infinity,y0=Infinity,x1=-Infinity,y1=-Infinity;
    nodes.forEach(n=>{x0=Math.min(x0,n.x);y0=Math.min(y0,n.y);x1=Math.max(x1,n.x+NW);y1=Math.max(y1,n.y+NH)});
    const w=svg.clientWidth,h=svg.clientHeight-36,gw=x1-x0+80,gh=y1-y0+80;
    scale=Math.min(w/gw,h/gh,1.5);tx=(w-gw*scale)/2-x0*scale;ty=36+(h-gh*scale)/2-y0*scale;
    viewport.setAttribute('transform','translate('+tx+','+ty+') scale('+scale+')');
}
render();setTimeout(fitAll,100);
</script></body></html>`;
}
