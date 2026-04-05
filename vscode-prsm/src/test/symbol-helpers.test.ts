import test from 'node:test';
import assert from 'node:assert/strict';
import { buildPrSMDocumentSymbolTree, filterPrSMWorkspaceSymbols } from '../symbol-helpers';

test('buildPrSMDocumentSymbolTree nests members under top-level declarations', () => {
    const tree = buildPrSMDocumentSymbolTree([
        {
            name: 'jump',
            qualified_name: 'Player.jump',
            container_name: 'Player',
            kind: 'function',
            signature: 'func jump(): Unit',
            file: 'Assets/Player.prsm',
            line: 3,
            col: 10,
        },
        {
            name: 'speed',
            qualified_name: 'Player.speed',
            container_name: 'Player',
            kind: 'serialize-field',
            signature: 'serialize speed: Float',
            file: 'Assets/Player.prsm',
            line: 2,
            col: 15,
        },
        {
            name: 'Player',
            qualified_name: 'Player',
            kind: 'component',
            signature: 'component Player : MonoBehaviour',
            file: 'Assets/Player.prsm',
            line: 1,
            col: 11,
        },
    ]);

    assert.equal(tree.length, 1);
    assert.equal(tree[0].symbol.name, 'Player');
    assert.deepEqual(tree[0].children.map(child => child.symbol.name), ['speed', 'jump']);
});

test('filterPrSMWorkspaceSymbols ranks exact and prefix matches first', () => {
    const matches = filterPrSMWorkspaceSymbols([
        {
            name: 'jump',
            qualified_name: 'Player.jump',
            container_name: 'Player',
            kind: 'function',
            signature: 'func jump(): Unit',
            file: 'Assets/Player.prsm',
            line: 3,
            col: 10,
        },
        {
            name: 'jumpHeight',
            qualified_name: 'Player.jumpHeight',
            container_name: 'Player',
            kind: 'field',
            signature: 'var jumpHeight: Float',
            file: 'Assets/Player.prsm',
            line: 2,
            col: 9,
        },
        {
            name: 'land',
            qualified_name: 'Player.land',
            container_name: 'Player',
            kind: 'function',
            signature: 'func land(): Unit',
            file: 'Assets/Player.prsm',
            line: 6,
            col: 10,
        },
    ], 'jump');

    assert.deepEqual(matches.map(match => match.name), ['jump', 'jumpHeight']);
});

test('filterPrSMWorkspaceSymbols falls back to qualified name and signature search', () => {
    const matches = filterPrSMWorkspaceSymbols([
        {
            name: 'damage',
            qualified_name: 'WeaponData.damage',
            container_name: 'WeaponData',
            kind: 'field',
            signature: 'val damage: Int',
            file: 'Assets/WeaponData.prsm',
            line: 2,
            col: 9,
        },
        {
            name: 'dps',
            qualified_name: 'WeaponData.dps',
            container_name: 'WeaponData',
            kind: 'function',
            signature: 'func dps(): Float',
            file: 'Assets/WeaponData.prsm',
            line: 6,
            col: 10,
        },
    ], 'weapondata');

    assert.deepEqual(matches.map(match => match.name), ['damage', 'dps']);
});