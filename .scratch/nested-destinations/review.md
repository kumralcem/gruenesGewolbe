# Implementation review

Baseline: e6ce759c1699cb183bb7e7d8bd415a2f56605a78. Reviewed working changes before commit.

## Standards

Independent review found parent renames could push descendants beyond supported path depth/length. Fixed with preflight validation of all relocated discovered destinations and a regression test. Readability suggestion to rename the destination validator was applied.

## Spec

Independent review found older folder journals cannot safely undo create/rename under filesystem discovery. These operations now fail explicitly before writes; ordinary file history remains supported. Regression test and upgrade documentation added.

## Validation

- Typecheck passed.
- Full unit suite: 42 passed.
- Real Pi fixture tests passed, including automatic exposure of a manually created nested destination.
- Rootless worker rebuilt successfully.
- Container suite: 10 passed. Subsequent controller history/rename review fixes received focused tests and another image rebuild.
- Directory cleanup also preserves pre-existing empty directories during undo.

Both review findings addressed; arbitrary manual rearrangement reconciliation remains out of scope.
