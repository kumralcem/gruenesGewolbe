Status: ready-for-human
State: open
Assignee: unassigned
Labels: wayfinder:grilling
Type: grilling
Mode: HITL
Parent: ../map.md
Blocked by: none

# Define when a browser tab is safe to close

## Question

What visible capture outcomes tell the user that the content is preserved, that only its summary is pending, or that pasted source text is still required?

## Starting requirements

URL paste is the normal workflow. Pasting source text followed by app summarization is accepted when extraction fails. A URL alone is insufficient. The user wants blog, tweet/post, and website content locally reachable in the future, with an obvious Idea Sources destination. Existing ADRs prefer full cleaned main text and exclude surrounding discussion by default.

## Exchange to conduct

Use concrete cases: an ordinary article, a text-only tweet, a post with an image, a blocked URL, and a provider failure after source preservation. Propose automatic summary generation when configured, source-first persistence, and visible separate source/summary states. Clarify how much manual fallback is tolerable and what incomplete captures retain on restart. Ask one question at a time and do not reopen the already accepted text fallback.

## Resolution evidence

Agreed outcomes for each case and an explicit safe-to-close indication grounded in local persistence. Keep user requirements distinct from proposed defaults and label partial preservation honestly.
