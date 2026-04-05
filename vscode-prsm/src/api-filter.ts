import * as vscode from 'vscode';

/**
 * Filters out dangerous, unsupported, or obsolete APIs
 * from C# IntelliSense results before showing to PrSM users.
 */

// ── Blocked namespaces (entire namespace blocked) ────────────

const BLOCKED_NAMESPACES: string[] = [
    'System.Threading',
    'System.Threading.Tasks',
    'System.Runtime.InteropServices',
    'System.Runtime.CompilerServices',
    'System.Runtime.Serialization',
    'System.Reflection',
    'System.Reflection.Emit',
    'System.Security',
    'System.Security.Cryptography',
    'System.CodeDom',
    'System.Diagnostics.Process',
    'System.AppDomain',
];

// ── Blocked types ────────────────────────────────────────────

const BLOCKED_TYPES: Set<string> = new Set([
    // Threading (dangerous in Unity)
    'Thread', 'ThreadPool', 'Mutex', 'Semaphore', 'Monitor',
    'Task', 'Task`1', 'TaskFactory', 'TaskScheduler',
    'CancellationToken', 'CancellationTokenSource',

    // IO (restricted on mobile/WebGL)
    'File', 'Directory', 'FileStream', 'StreamWriter', 'StreamReader',
    'FileInfo', 'DirectoryInfo', 'Path',

    // Reflection (IL2CPP incompatible)
    'Assembly', 'MethodInfo', 'FieldInfo', 'PropertyInfo',
    'ConstructorInfo', 'Type', 'Activator',
    'DynamicMethod', 'ILGenerator',

    // Process (not available on most platforms)
    'Process', 'ProcessStartInfo',

    // Networking (use UnityWebRequest instead)
    'WebClient', 'HttpClient', 'WebRequest', 'HttpWebRequest',
    'TcpClient', 'TcpListener', 'UdpClient', 'Socket',

    // WinForms / WPF (not Unity)
    'Form', 'Control', 'Window', 'Application',

    // Obsolete Unity types
    'WWW', 'UnityEngine.WWW',
]);

// ── Blocked members (type.member) ────────────────────────────

const BLOCKED_MEMBERS: Set<string> = new Set([
    // GameObject
    'GameObject.Find',           // Performance: use references instead
    'GameObject.FindWithTag',    // Performance: cache results

    // Object
    'Object.FindObjectOfType',   // Deprecated
    'Object.FindObjectsOfType',  // Deprecated

    // Unity obsolete patterns
    'Application.LoadLevel',     // Use SceneManager
    'Application.LoadLevelAsync',
    'GUIText',
    'GUITexture',

    // Dangerous at runtime
    'GC.Collect',               // Never manually call GC in Unity
    'Resources.UnloadUnusedAssets', // Use Addressables instead

    // Rigidbody deprecated
    'Rigidbody.velocity',       // Not blocked, just note: use linearVelocity in 6000+
]);

// ── Blocked member name patterns ─────────────────────────────

const BLOCKED_MEMBER_PATTERNS: RegExp[] = [
    /^__(.*)/,                   // Internal/compiler-generated members
    /^op_(.*)/,                  // Operator overloads (noisy)
    /^get_(.*)/, /^set_(.*)/,   // Property accessors (redundant)
    /^add_(.*)/, /^remove_(.*)/,// Event accessors
    /^\.ctor$/,                  // Raw constructor
    /^\.cctor$/,                 // Static constructor
    /^Finalize$/,                // Destructor
    /^MemberwiseClone$/,         // Object internals
    /^GetHashCode$/,             // Rarely needed directly
    /^ReferenceEquals$/,
    /^obj_address$/,
];

// ── Obsolete detection patterns ──────────────────────────────

const OBSOLETE_PATTERNS: string[] = [
    '[Obsolete',
    'Obsolete(',
    'is obsolete',
    'has been deprecated',
    'Use ',  // Common in obsolete messages
];

/**
 * Filter a completion item. Returns true if it should be KEPT.
 */
export function shouldKeepCompletion(item: vscode.CompletionItem): boolean {
    const label = typeof item.label === 'string' ? item.label : item.label.label;
    const detail = item.detail || '';
    const doc = getDocString(item.documentation);

    // 1. Check blocked member name patterns
    for (const pattern of BLOCKED_MEMBER_PATTERNS) {
        if (pattern.test(label)) return false;
    }

    // 2. Check blocked types in detail
    for (const blockedType of BLOCKED_TYPES) {
        if (detail.includes(blockedType) && detail.includes('(via C#)')) {
            return false;
        }
    }

    // 3. Check blocked namespaces in detail
    for (const ns of BLOCKED_NAMESPACES) {
        if (detail.includes(ns)) return false;
    }

    // 4. Check obsolete
    if (isObsolete(detail, doc)) return false;

    return true;
}

/**
 * Filter a list of completion items.
 */
export function filterCompletions(items: vscode.CompletionItem[]): vscode.CompletionItem[] {
    return items.filter(shouldKeepCompletion);
}

/**
 * Check if a specific type.member is blocked.
 */
export function isMemberBlocked(typeName: string, memberName: string): boolean {
    const key = `${typeName}.${memberName}`;
    if (BLOCKED_MEMBERS.has(key)) return true;

    for (const pattern of BLOCKED_MEMBER_PATTERNS) {
        if (pattern.test(memberName)) return true;
    }

    return false;
}

/**
 * Check if a type name is blocked.
 */
export function isTypeBlocked(typeName: string): boolean {
    return BLOCKED_TYPES.has(typeName);
}

/**
 * Mark an item as deprecated (strikethrough) instead of hiding it.
 */
export function markDeprecated(item: vscode.CompletionItem): vscode.CompletionItem {
    item.tags = [vscode.CompletionItemTag.Deprecated];
    return item;
}

// ── Helpers ──────────────────────────────────────────────────

function isObsolete(detail: string, doc: string): boolean {
    const combined = (detail + ' ' + doc).toLowerCase();
    return OBSOLETE_PATTERNS.some(p => combined.includes(p.toLowerCase()));
}

function getDocString(doc: string | vscode.MarkdownString | undefined): string {
    if (!doc) return '';
    if (typeof doc === 'string') return doc;
    return doc.value;
}
