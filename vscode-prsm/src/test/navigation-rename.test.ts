import test from 'node:test';
import assert from 'node:assert/strict';
import { getPrSMRenamePlan, getPrSMRenameSupportError, validatePrSMRenameName } from '../navigation-rename';

test('getPrSMRenamePlan includes declaration and references once', () => {
    const plan = getPrSMRenamePlan({
        definition: {
            id: 1,
            name: 'speed',
            qualified_name: 'Player.jump.speed',
            kind: 'local',
            type: 'Int',
            mutable: false,
            file: 'Assets/Player.prsm',
            line: 3,
            col: 13,
            end_line: 3,
            end_col: 17,
        },
        references: [
            {
                name: 'speed',
                kind: 'identifier',
                file: 'Assets/Player.prsm',
                line: 4,
                col: 20,
                end_line: 4,
                end_col: 24,
            },
            {
                name: 'speed',
                kind: 'identifier',
                file: 'Assets/Player.prsm',
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

test('getPrSMRenamePlan rejects lifecycle rename targets', () => {
    const plan = getPrSMRenamePlan({
        definition: {
            id: 2,
            name: 'update',
            qualified_name: 'Player.update',
            kind: 'lifecycle',
            type: 'Unit',
            mutable: false,
            file: 'Assets/Player.prsm',
            line: 5,
            col: 5,
        },
        references: [],
    });

    assert.equal(plan, null);
    assert.match(getPrSMRenameSupportError({
        definition: {
            id: 2,
            name: 'update',
            qualified_name: 'Player.update',
            kind: 'lifecycle',
            type: 'Unit',
            mutable: false,
            file: 'Assets/Player.prsm',
            line: 5,
            col: 5,
        },
        references: [],
    }), /fixed Unity callback names/i);
});

test('validatePrSMRenameName rejects invalid identifiers and keywords', () => {
    assert.match(validatePrSMRenameName('') ?? '', /cannot be empty/i);
    assert.match(validatePrSMRenameName(' badName') ?? '', /whitespace/i);
    assert.match(validatePrSMRenameName('123name') ?? '', /valid PrSM identifier/i);
    assert.match(validatePrSMRenameName('component') ?? '', /reserved PrSM keyword/i);
    assert.equal(validatePrSMRenameName('nextSpeed'), undefined);
});