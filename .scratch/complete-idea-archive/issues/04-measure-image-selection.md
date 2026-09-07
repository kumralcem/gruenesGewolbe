Status: ready-for-agent
State: open
Assignee: unassigned
Labels: wayfinder:task
Type: task
Mode: AFK
Parent: ../map.md
Blocked by: 01-establish-safe-measurement.md

# Locate the delay after selecting an image

## Question

Which part of native image selection accounts for the perceived wait, and what measured boundary should the responsiveness fix target?

## Work needed to unblock the decision

Using the bounded procedure, distinguish click feedback, native command time, record reads, preview load/decode, and paint. Compare first/repeat selection and selection while thumbnail preparation runs. Inspect cancellation and stale-result handling. The present UI already has pending-selection and thumbnail-preview code, so do not assume it always decodes originals or lacks immediate feedback. Existing controlled-response browser tests cannot establish native latency.

## Resolution evidence

Measurements with fixture size and cold/warm conditions, a reproducible narrow symptom where possible, and a proposed user-visible latency criterion. If representative native reproduction requires user participation, record the missing evidence and keep the ticket open rather than inventing a cause. The test freeze remains a separate symptom unless evidence connects them.
