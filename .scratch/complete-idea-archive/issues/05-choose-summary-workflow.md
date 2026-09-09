Status: ready-for-human
State: open
Assignee: unassigned
Labels: wayfinder:grilling
Type: grilling
Mode: HITL
Parent: ../map.md
Blocked by: 02-define-trustworthy-capture.md

# Choose how summaries become part of capture

## Question

How should configured summarization start, communicate cost and remote processing, recover from failure, and protect user edits while making URL and pasted-text capture feel complete?

## Context

The user expects the app to summarize the text, not require a manually written summary as the finished experience. Existing ADRs choose OpenAI first, budget modes, minimal source context, and user-specific configuration. Existing tests prove provider interfaces and fake-provider rules; no native live integration has been demonstrated.

## Exchange to conduct

Recommend automatic summarization after durable source preservation when the provider and budget are configured, with a retry action and preserved source available immediately. Settle first-run behavior without credentials, whether the existing provider direction still fits, the meaning of the source's central point versus the user's Saving Reason, and handling of edited summaries. Do not make source ownership depend on provider availability.

## Resolution evidence

Agreed default, failure/retry behavior, and configuration requirements. If provider capability facts require external research, create a separately scoped research ticket then; do not guess changing API details here.
