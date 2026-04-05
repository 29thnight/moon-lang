import * as vscode from 'vscode';

const LIFECYCLE_BLOCKS: { label: string; snippet: string; description: string }[] = [
    { label: 'awake', snippet: 'awake {\n    $0\n}', description: 'Called when the script instance is being loaded' },
    { label: 'start', snippet: 'start {\n    $0\n}', description: 'Called before the first frame update' },
    { label: 'update', snippet: 'update {\n    $0\n}', description: 'Called once per frame' },
    { label: 'fixedUpdate', snippet: 'fixedUpdate {\n    $0\n}', description: 'Called at fixed time intervals (physics)' },
    { label: 'lateUpdate', snippet: 'lateUpdate {\n    $0\n}', description: 'Called after all Update functions' },
    { label: 'onEnable', snippet: 'onEnable {\n    $0\n}', description: 'Called when the component becomes enabled' },
    { label: 'onDisable', snippet: 'onDisable {\n    $0\n}', description: 'Called when the component becomes disabled' },
    { label: 'onDestroy', snippet: 'onDestroy {\n    $0\n}', description: 'Called when the component is destroyed' },
    { label: 'onTriggerEnter', snippet: 'onTriggerEnter(other: Collider) {\n    $0\n}', description: 'Called when a collider enters the trigger' },
    { label: 'onTriggerExit', snippet: 'onTriggerExit(other: Collider) {\n    $0\n}', description: 'Called when a collider exits the trigger' },
    { label: 'onTriggerStay', snippet: 'onTriggerStay(other: Collider) {\n    $0\n}', description: 'Called every frame a collider stays in trigger' },
    { label: 'onCollisionEnter', snippet: 'onCollisionEnter(collision: Collision) {\n    $0\n}', description: 'Called when a collision starts' },
    { label: 'onCollisionExit', snippet: 'onCollisionExit(collision: Collision) {\n    $0\n}', description: 'Called when a collision ends' },
    { label: 'onCollisionStay', snippet: 'onCollisionStay(collision: Collision) {\n    $0\n}', description: 'Called every frame during collision' },
];

/**
 * Shows a quick pick to insert a lifecycle block at cursor position.
 */
export async function insertLifecycleBlock() {
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.languageId !== 'prsm') {
        vscode.window.showWarningMessage('No .prsm file is open');
        return;
    }

    const text = editor.document.getText();

    // Check which lifecycles already exist
    const existing = new Set<string>();
    for (const lc of LIFECYCLE_BLOCKS) {
        const regex = new RegExp(`\\b${lc.label}\\s*(?:\\(|\\{)`);
        if (regex.test(text)) existing.add(lc.label);
    }

    // Build quick pick items
    const items: vscode.QuickPickItem[] = LIFECYCLE_BLOCKS.map(lc => ({
        label: existing.has(lc.label) ? `$(check) ${lc.label}` : lc.label,
        description: existing.has(lc.label) ? 'already exists' : lc.description,
        detail: existing.has(lc.label) ? undefined : `Inserts: ${lc.label} { }`,
        _snippet: lc.snippet,
        _exists: existing.has(lc.label),
        _label: lc.label,
    } as any));

    const selected = await vscode.window.showQuickPick(items, {
        placeHolder: 'Select lifecycle block to insert',
        matchOnDescription: true,
    });

    if (!selected) return;

    const data = selected as any;
    if (data._exists) {
        vscode.window.showInformationMessage(`'${data._label}' already exists in this file`);
        return;
    }

    // Find insertion point: before the last `}` of the component/class body
    const lastBrace = text.lastIndexOf('}');
    if (lastBrace < 0) return;

    const insertPos = editor.document.positionAt(lastBrace);

    // Detect indentation
    const indent = '    ';

    await editor.edit(editBuilder => {
        editBuilder.insert(insertPos, `\n${indent}`);
    });

    // Move cursor and insert snippet
    const newPos = new vscode.Position(insertPos.line + 1, indent.length);
    editor.selection = new vscode.Selection(newPos, newPos);

    await vscode.commands.executeCommand('editor.action.insertSnippet', {
        snippet: data._snippet.split('\n').map((line: string, i: number) =>
            i === 0 ? line : indent + line
        ).join('\n')
    });
}
