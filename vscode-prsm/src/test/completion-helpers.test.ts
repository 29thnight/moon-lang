import test from 'node:test';
import assert from 'node:assert/strict';
import {
    extractUsings,
    extractUserSymbols,
    resolveReceiverTypeFromText,
} from '../completion-helpers';

test('extractUserSymbols finds declarations, members, and enum entries', () => {
    const text = `using UnityEngine

component PlayerController : MonoBehaviour {
    require rb: Rigidbody
    serialize speed: Float = 5.0

    func Jump(force: Float) {
    }

    coroutine Flash() {
    }
}

enum State { Idle, Run(speed: Float) }`;

    const symbols = extractUserSymbols(text, 'C:/workspace/PlayerController.prsm');
    const names = symbols.map(symbol => symbol.name);

    assert.ok(names.includes('PlayerController'));
    assert.ok(names.includes('rb'));
    assert.ok(names.includes('speed'));
    assert.ok(names.includes('Jump'));
    assert.ok(names.includes('Flash'));
    assert.ok(names.includes('State'));
    assert.ok(names.includes('State.Idle'));
    assert.ok(names.includes('State.Run'));
});

test('resolveReceiverTypeFromText finds declared member types', () => {
    const text = `component PlayerController : MonoBehaviour {
    require rb: Rigidbody
    var score: Int = 0
}`;

    assert.equal(resolveReceiverTypeFromText(text, 'rb'), 'Rigidbody');
    assert.equal(resolveReceiverTypeFromText(text, 'score'), 'Int');
    assert.equal(resolveReceiverTypeFromText(text, 'missing'), undefined);
});

test('extractUsings returns namespace imports from the file header', () => {
    const text = `using UnityEngine
using UnityEngine.UI

component Demo : MonoBehaviour {}`;

    assert.deepEqual(extractUsings(text), ['UnityEngine', 'UnityEngine.UI']);
});
