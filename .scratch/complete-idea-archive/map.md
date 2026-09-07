Status: ready-for-agent
Labels: wayfinder:map

# Close browser tabs with confidence

## Destination

A decision-complete specification and implementation route for a responsive desktop archive: paste a blog, tweet/post, or website URL into a visible Idea Sources area, preserve readable content and a useful summary locally, and retrieve it after the original disappears. The route must also address test-induced machine freezes and delayed image selection, with evidence-based acceptance gates for the whole product.

## Notes

- Execution override, 2026-09-06: the user asked to start implementing step by step with lower-cost subagents and code review. Continue through implementation with reasonable defaults grounded in the stated workflow; do not re-request approval for the accepted URL/text fallback or visible Idea Sources destination. Planned features still cannot be reported as delivered without verification.
- User input, 2026-09-06: many open browser tabs are the current holding system. URL paste should be the normal entry; pasted source text followed by app summarization is an acceptable fallback. Posts, blogs, and web pages need an obvious area in the left navigation. Image archiving already works, but image clicks feel delayed. Test execution freezes the machine; neither OOM nor Playwright has been proved as the cause.
- Product acceptance includes both images and Idea Sources. A link-only record must never be presented as preserved source content. Source preservation and summary generation can fail independently and need honest visible outcomes.
- Retain this repo and its canonical Vault model unless an investigation establishes a concrete reason to change them. Existing text-preservation and provider boundaries are useful foundations; a rewrite is not the default route.
- Consult `CONTEXT.md`, relevant ADRs, and `docs/agents/issue-tracker.md`. Use Wayfinder from `/home/cem/Sync/Projects/DianeClaw/.agents/skills/wayfinder/SKILL.md`, with Grilling and Domain Modeling for decisions, Prototype for UI questions, and Diagnosing Bugs for freeze/performance investigations. Preserve user input and do not substitute an agent's preference for a HITL answer.
- [Initial repository assessment](PRD.md) contains observations and a provisional delivery outline; it is not the approved specification. Resolved tickets take precedence over its proposals.
- Default Playwright concurrency has been reduced to one worker. Type checking and discovery of 31 browser tests passed without browser launch. This is not evidence that freezes are fixed. Keep verification jobs sequential and do not reproduce a machine-wide freeze as an ordinary test.
- Existing ADRs favor cleaned text, optional OpenAI enhancement, ordinary local files, and no surrounding discussion by default. UI naming, capture states, summary automation, and provider readiness remain ticket decisions.

## Decisions so far

- [Establish a safe measurement path](issues/01-establish-safe-measurement.md): bounded Linux verification runner validated with success, failure propagation, and focused core capture tests; historical freeze cause remains unknown.

## Not yet specified

- The exact implementation slices and narrow module changes depend on capture-state and UI decisions; derive them after those decisions, rather than parceling out disconnected backend work now.
- Handling old link-only Idea Sources may require a targeted completion workflow once capture recovery semantics are defined.
- Additional platform-specific investigations may emerge from the user's representative URLs; do not promise universal extraction before seeing those cases.

## Out of scope

- Shipping the implementation during this map-charting session.
- Universal website mirroring, authenticated crawling, a browser extension, and importing every open browser tab in bulk. The agreed entry is URL paste with text fallback.
- Vector search, image identification, unrelated codebase rewrites, packaging, and broader platform support.
- Replacing preserved originals or migrating existing Vault files without a demonstrated need.
