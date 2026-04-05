import test from 'node:test';
import assert from 'node:assert/strict';
import { getNavigationCSharpTarget } from '../csharp-navigation';
import { getNavigationCSharpHoverSection, getNavigationFallbackTarget, getNavigationHoverText } from '../navigation-helpers';

test('getNavigationHoverText formats symbol hover content', () => {
    const text = getNavigationHoverText({
        symbol_at: {
            name: 'jump',
            qualified_name: 'Player.jump',
            container_name: 'Player',
            kind: 'function',
            signature: 'public func jump(): Unit',
            file: 'Assets/Player.prsm',
            line: 2,
            col: 10,
        },
    }, {
        id: 1,
        name: 'jump',
        qualified_name: 'Player.jump',
        kind: 'function',
        type: 'Unit',
        mutable: false,
        file: 'Assets/Player.prsm',
        line: 2,
        col: 10,
    }, {
        typeName: 'Player',
        memberName: 'jump',
        generatedFile: 'build-output/Player.cs',
        hoverText: 'Unity member\n\n```csharp\npublic void jump()\n```',
    });

    assert.match(text ?? '', /public func jump\(\): Unit/);
    assert.match(text ?? '', /\[Generated C#\]/);
    assert.match(text ?? '', /public void jump\(\)/);
    assert.match(text ?? '', /Unity member/);
    assert.doesNotMatch(text ?? '', /Status:/);
    assert.doesNotMatch(text ?? '', /Definition:/);
    assert.doesNotMatch(text ?? '', /Lookup:/);
    assert.doesNotMatch(text ?? '', /File:/);
});

test('getNavigationHoverText formats resolved reference hover content', () => {
    const text = getNavigationHoverText({
        reference_at: {
            name: 'WeaponData',
            kind: 'type',
            file: 'Assets/Player.prsm',
            line: 4,
            col: 22,
            resolved_symbol: {
                name: 'WeaponData',
                qualified_name: 'WeaponData',
                kind: 'type',
                signature: 'data class WeaponData(damage: Int)',
                file: 'Assets/WeaponData.prsm',
                line: 1,
                col: 12,
            },
        },
    }, {
        id: 2,
        name: 'WeaponData',
        qualified_name: 'WeaponData',
        kind: 'type',
        type: 'WeaponData',
        mutable: false,
        file: 'Assets/WeaponData.prsm',
        line: 1,
        col: 12,
    }, {
        typeName: 'WeaponData',
        generatedFile: 'build-output/WeaponData.cs',
        hoverText: 'Unity script\n\n```csharp\npublic class WeaponData : ScriptableObject\n```',
    });

    assert.match(text ?? '', /data class WeaponData\(damage: Int\)/);
    assert.match(text ?? '', /\[Generated C#\]/);
    assert.match(text ?? '', /public class WeaponData : ScriptableObject/);
    assert.match(text ?? '', /Unity script/);
    assert.doesNotMatch(text ?? '', /Status:/);
    assert.doesNotMatch(text ?? '', /Definition:/);
    assert.doesNotMatch(text ?? '', /Lookup:/);
    assert.doesNotMatch(text ?? '', /File:/);
});

test('getNavigationHoverText formats unresolved reference hover content', () => {
    const text = getNavigationHoverText({
        reference_at: {
            name: 'MonoBehaviour',
            kind: 'type',
            file: 'Assets/Player.prsm',
            line: 1,
            col: 20,
        },
    }, undefined, {
        typeName: 'MonoBehaviour',
        hoverText: '```csharp\npublic class MonoBehaviour : Behaviour\n```',
    });

    assert.match(text ?? '', /```prsm\nMonoBehaviour\n```/);
    assert.match(text ?? '', /MonoBehaviour : Behaviour/);
    assert.doesNotMatch(text ?? '', /Status:/);
});

test('getNavigationCSharpHoverSection formats supplemental C# hover content', () => {
    const text = getNavigationCSharpHoverSection({
        typeName: 'MonoBehaviour',
        hoverText: '```csharp\npublic class MonoBehaviour : Behaviour\n```',
    });

    assert.match(text ?? '', /\[Generated C#\]/);
    assert.match(text ?? '', /MonoBehaviour : Behaviour/);
});

test('getNavigationCSharpTarget maps lifecycle definitions to generated C# members', () => {
    const target = getNavigationCSharpTarget({
        symbol_at: {
            name: 'update',
            qualified_name: 'PlayerController.update',
            container_name: 'PlayerController',
            kind: 'lifecycle',
            signature: 'update()',
            file: 'Assets/PlayerController.prsm',
            line: 6,
            col: 5,
        },
    }, {
        id: 3,
        name: 'update',
        qualified_name: 'PlayerController.update',
        kind: 'lifecycle',
        type: 'Unit',
        mutable: false,
        file: 'Assets/PlayerController.prsm',
        line: 6,
        col: 5,
    });

    assert.deepEqual(target, {
        typeName: 'PlayerController',
        memberName: 'Update',
    });
});

test('getNavigationCSharpTarget falls back to unresolved type references', () => {
    const target = getNavigationCSharpTarget({
        reference_at: {
            name: 'MonoBehaviour',
            kind: 'type',
            file: 'Assets/Player.prsm',
            line: 1,
            col: 20,
        },
    });

    assert.deepEqual(target, {
        typeName: 'MonoBehaviour',
    });
});

test('getNavigationFallbackTarget prefers resolved references over symbol_at', () => {
    const target = getNavigationFallbackTarget({
        symbol_at: {
            name: 'Player',
            qualified_name: 'Player',
            kind: 'component',
            signature: 'component Player : MonoBehaviour',
            file: 'Assets/Player.prsm',
            line: 1,
            col: 11,
        },
        reference_at: {
            name: 'WeaponData',
            kind: 'type',
            file: 'Assets/Player.prsm',
            line: 2,
            col: 20,
            resolved_symbol: {
                name: 'WeaponData',
                qualified_name: 'WeaponData',
                kind: 'type',
                signature: 'data class WeaponData(damage: Int)',
                file: 'Assets/WeaponData.prsm',
                line: 1,
                col: 12,
            },
        },
    });

    assert.equal(target?.qualified_name, 'WeaponData');
});