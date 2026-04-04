import test from 'node:test';
import assert from 'node:assert/strict';
import { getNavigationCSharpTarget } from '../csharp-navigation';
import { getNavigationFallbackTarget, getNavigationHoverText } from '../navigation-helpers';

test('getNavigationHoverText formats symbol hover content', () => {
    const text = getNavigationHoverText({
        symbol_at: {
            name: 'jump',
            qualified_name: 'Player.jump',
            container_name: 'Player',
            kind: 'function',
            signature: 'public func jump(): Unit',
            file: 'Assets/Player.mn',
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
        file: 'Assets/Player.mn',
        line: 2,
        col: 10,
    }, {
        typeName: 'Player',
        memberName: 'jump',
        generatedFile: 'build-output/Player.cs',
        hoverText: '```csharp\npublic void jump()\n```',
    });

    assert.match(text ?? '', /Player\.jump/);
    assert.match(text ?? '', /public func jump\(\): Unit/);
    assert.match(text ?? '', /Status:\*\* Defined/);
    assert.match(text ?? '', /Type:\*\* Unit/);
    assert.match(text ?? '', /Assets\/Player\.mn:2:10/);
    assert.match(text ?? '', /Generated C#/);
    assert.match(text ?? '', /build-output\/Player\.cs/);
    assert.match(text ?? '', /public void jump\(\)/);
});

test('getNavigationHoverText formats resolved reference hover content', () => {
    const text = getNavigationHoverText({
        reference_at: {
            name: 'WeaponData',
            kind: 'type',
            file: 'Assets/Player.mn',
            line: 4,
            col: 22,
            resolved_symbol: {
                name: 'WeaponData',
                qualified_name: 'WeaponData',
                kind: 'type',
                signature: 'data class WeaponData(damage: Int)',
                file: 'Assets/WeaponData.mn',
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
        file: 'Assets/WeaponData.mn',
        line: 1,
        col: 12,
    }, {
        typeName: 'WeaponData',
        generatedFile: 'build-output/WeaponData.cs',
        hoverText: '```csharp\npublic class WeaponData : ScriptableObject\n```',
    });

    assert.match(text ?? '', /type reference/i);
    assert.match(text ?? '', /Status:\*\* Resolved/);
    assert.match(text ?? '', /WeaponData/);
    assert.match(text ?? '', /data class WeaponData\(damage: Int\)/);
    assert.match(text ?? '', /Type:\*\* WeaponData/);
    assert.match(text ?? '', /Assets\/WeaponData\.mn:1:12/);
    assert.match(text ?? '', /build-output\/WeaponData\.cs/);
    assert.match(text ?? '', /public class WeaponData : ScriptableObject/);
});

test('getNavigationHoverText formats unresolved reference hover content', () => {
    const text = getNavigationHoverText({
        reference_at: {
            name: 'MonoBehaviour',
            kind: 'type',
            file: 'Assets/Player.mn',
            line: 1,
            col: 20,
        },
    }, undefined, {
        typeName: 'MonoBehaviour',
        hoverText: '```csharp\npublic class MonoBehaviour : Behaviour\n```',
    });

    assert.match(text ?? '', /Status:\*\* Unresolved/);
    assert.match(text ?? '', /Not found in the current Moon project index/);
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
            file: 'Assets/PlayerController.mn',
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
        file: 'Assets/PlayerController.mn',
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
            file: 'Assets/Player.mn',
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
            file: 'Assets/Player.mn',
            line: 1,
            col: 11,
        },
        reference_at: {
            name: 'WeaponData',
            kind: 'type',
            file: 'Assets/Player.mn',
            line: 2,
            col: 20,
            resolved_symbol: {
                name: 'WeaponData',
                qualified_name: 'WeaponData',
                kind: 'type',
                signature: 'data class WeaponData(damage: Int)',
                file: 'Assets/WeaponData.mn',
                line: 1,
                col: 12,
            },
        },
    });

    assert.equal(target?.qualified_name, 'WeaponData');
});