import test from 'node:test';
import assert from 'node:assert/strict';
import { getMoonRenamePlan, getMoonRenameSupportError, validateMoonRenameName } from '../navigation-rename';

test('getMoonRenamePlan includes declaration and references once', () => {
    const plan = getMoonRenamePlan({
        definition: {
            id: 1,
            name: 'speed',
            qualified_name: 'Player.jump.speed',
            kind: 'local',
            type: 'Int',
            mutable: false,
            file: 'Assets/Player.mn',
            line: 3,
            col: 13,
            end_line: 3,
            end_col: 17,
        },
        references: [
            {
                name: 'speed',
                kind: 'identifier',
                file: 'Assets/Player.mn',
                line: 4,
                col: 20,
                end_line: 4,
                end_col: 24,
            },
            {
                name: 'speed',
                kind: 'identifier',
                file: 'Assets/Player.mn',
                line: 4,
                col: 20,
                end_line: 4,
                end_col: 24,
            },
        ],
    });

    assert.equal(plan?.placeholder, 'speed');
    assert.equal(plan?.locations.length, 2);
    assert.equal(plan?.locations[0].line, 3);
    assert.equal(plan?.locations[1].line, 4);
});

test('getMoonRenamePlan rejects lifecycle rename targets', () => {
    const plan = getMoonRenamePlan({
        definition: {
            id: 2,
            name: 'update',
            qualified_name: 'Player.update',
            kind: 'lifecycle',
            type: 'Unit',
            mutable: false,
            file: 'Assets/Player.mn',
            line: 5,
            col: 5,
        },
        references: [],
    });

    assert.equal(plan, null);
    assert.match(getMoonRenameSupportError({
        definition: {
            id: 2,
            name: 'update',
            qualified_name: 'Player.update',
            kind: 'lifecycle',
            type: 'Unit',
            mutable: false,
            file: 'Assets/Player.mn',
            line: 5,
            col: 5,
        },
        references: [],
    }), /fixed Unity callback names/i);
});

test('validateMoonRenameName rejects invalid identifiers and keywords', () => {
    assert.match(validateMoonRenameName('') ?? '', /cannot be empty/i);
    assert.match(validateMoonRenameName(' badName') ?? '', /whitespace/i);
    assert.match(validateMoonRenameName('123name') ?? '', /valid Moon identifier/i);
    assert.match(validateMoonRenameName('component') ?? '', /reserved Moon keyword/i);
    assert.equal(validateMoonRenameName('nextSpeed'), undefined);
});