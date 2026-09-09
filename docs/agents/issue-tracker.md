# Issue tracker: Local Markdown

Issues and PRDs for this repo live as markdown files in `.scratch/`.

## Conventions

- One feature per directory: `.scratch/<feature-slug>/`
- The PRD is `.scratch/<feature-slug>/PRD.md`
- Implementation issues are `.scratch/<feature-slug>/issues/<NN>-<slug>.md`, numbered from `01`
- Triage state is recorded as a `Status:` line near the top of each issue file (see `triage-labels.md` for the role strings)
- Comments and conversation history append to the bottom of the file under a `## Comments` heading

## When a skill says "publish to the issue tracker"

Create a new file under `.scratch/<feature-slug>/` (creating the directory if needed).

## When a skill says "fetch the relevant ticket"

Read the file at the referenced path. The user will normally pass the path or the issue number directly.

## Wayfinding operations

- Map: `.scratch/<effort>/map.md`, labelled `wayfinder:map`.
- Child decision tickets: `.scratch/<effort>/issues/NN-<slug>.md`; `Parent:` links to the map and `Type:` records `task`, `research`, `prototype`, or `grilling`.
- Keep `Status:` for this repository's canonical triage strings. Use a separate `State: open`, `claimed`, or `resolved` for Wayfinder lifecycle; `Assignee:` records the claim owner.
- `Blocked by:` lists relative ticket paths, or `none`. A ticket is unblocked when all listed tickets have `State: resolved`.
- The frontier is open, unassigned, unblocked children, ordered by filename. The map does not duplicate this list.
- Claim by setting `State: claimed` and `Assignee:` before working. Resolve by appending a resolution under `## Comments`, setting `State: resolved`, and adding a named link with a brief gist to the map's Decisions so far.
- Create tickets before adding dependency links. Refer to maps and tickets by linked title in human-facing text.
