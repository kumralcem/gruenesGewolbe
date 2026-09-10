# Pi Vault Restart — Design Interview

Started: 2026-09-10
Status: ready-for-agent
Next work: scoped prototype validation; the design is agreed, but no implementation has been tested.

## Agreed direction

- Start again around an agent-led CLI that accepts source links and saves categorized material into the vault.
- The agent may use shell utilities and write/run scripts inside its sandbox. This allowance does not extend to the host, credentials, or unrestricted archive writes.
- Use a sandboxed Pi worker with a trusted GG controller outside it; real model credentials and live archive writes belong to the controller. Start with supported SDK integration rather than a fork.
- Build shared operations beneath the CLI and a later private personal browser app. Once hosted, use one authoritative writable server vault and a local backup, without two-way editing/synchronization.
- Express capture intent through specific commands, initially `gg painting <url>` and `gg idea <url>`. A UI can be built on top of those workflows later.
- Run captures in the foreground with progress initially, and support multiple inputs in one invocation for pasting batches of links. A background job system is not required for the initial CLI.
- Source websites are accessed without authentication in the first version: no website credentials, login flows, imported login cookies, or authenticated browser profiles. Model API keys remain separate configured credentials.
- Inaccessible links are reported with their reason and processing continues; do not create saved items or queue entries for them.
- Visual Capture (initially called Painting Capture) preserves the visual work and descriptive metadata; surrounding discussion is excluded by default.
- Visual Capture may research beyond the supplied page to find a better image and reliable identification as part of the same command.
- The agent distinguishes paintings, photography, sculpture, and other visual kinds and selects the appropriate home subvault. X is the main source; Wikipedia, Wikimedia Commons, and other sites are also in scope as sources, without an exhaustive host list.
- Vaults and subvaults are created or deleted only by the user, including defaults established during setup. The capture agent chooses among existing subvaults and cannot create or delete them.
- If no existing subvault fits, flag the capture in a persistent Capture Queue for later user approval or instructions.
- If a source has multiple plausible images without a clear selection, put it in the Capture Queue for the user to choose.
- Each Visual Capture targets one Selected Image. On X, quoted posts and replies are excluded and require separate explicit captures. Better copies must be of the same image; alternative viewpoints and automatic grouping of multiple depictions are outside the first version.
- Descriptive records contain creator, title, year, style, subjects, mood, tags, source links, and a short visual description where applicable. Factual attribution stays distinct from interpretation.
- If identity cannot be established confidently, save the image automatically, leave uncertain facts explicitly unresolved, and report the gaps.
- For a confidently matched existing Selected Image, choose a demonstrably higher-quality copy automatically; otherwise leave the item unchanged and return its location. Do not create a new saved item for a confirmed duplicate.
- Keep the initially obtained image and a better version together in the same saved item. Higher-quality primary selection preserves earlier saved versions; the additional storage is acceptable to the user.
- Idea Source capture accepts posts, direct articles, news stories, and YouTube videos. The summary is the most important output, backed by source text or transcript and links. A post and its linked article can be preserved together; replies are excluded unless requested.
- Summaries make underlying ideas findable by remembered problems and concepts. For instructional/advice sources, the summary preserves actionable instructions with the details needed to follow them and strips away surrounding narrative.
- Videos without usable existing transcripts are skipped with a notification. No item, queue entry, audio download, or transcription fallback is required for that case.
- Include both `gg search` and `gg ask`. Natural-language retrieval should bring back relevant saved files without requiring manual record editing; retrieval itself leaves files unchanged.
- An optional capture instruction can focus an Idea Source summary; it is not required.
- Support configurable OpenAI and OpenRouter access, including freely editable model identifiers and credentials, with a small CLI settings wizard. Do not hardcode a model list or pin the product to a named model release.
- Make source age visible for potentially dated instructions; distinguish publication from capture. Dates suffice; rechecking, last-checked tracking, and active updating are outside the requested workflow.
- The replacement must account for previously troublesome existing image files, including WebP and large dimensions. Inspection and acceptance fixtures are recorded in the source examples.
- Build a clean replacement. Image-format handling and thumbnail utilities are candidates for selective reuse, not a reason to retain the old architecture. Existing-archive import, migration, and bulk cleanup are outside the initial scope.
- Keep an ordinary-file archive, a shallow item layout, and Markdown item records. Do not introduce a database.
- Use the existing `/home/cem/Gewolbe` organization as a reference. Exact layout and record compatibility are still to be resolved.
- Record consequential decisions and sharpen the glossary as the interview progresses. The user explicitly permits multiple questions together; use small, related batches.

See [CLI restart](../../docs/adr/0040-agent-led-cli-restart.md), [visual classification](../../docs/adr/0041-agent-classified-visual-capture.md), [one selected image](../../docs/adr/0042-one-selected-image-per-visual-capture.md), [agent isolation](../../docs/adr/0043-isolated-agent-with-trusted-controller.md), and [private hosting](../../docs/adr/0044-private-hosted-interface-over-file-vault.md). No implementation or live-vault mutation has taken place.

## Evidence gathered

Read-only inspection found a format-v2 vault with 132 Paintings item records and 4 Idea Sources item records. Items live at `subvaults/<area>/items/<readable name>/record.md`, with preserved images under `files/` or source text under `source-copies/`. “Flat” is interpreted provisionally as shallow organization with sibling item folders rather than a literal single directory for every file.

Sample records preserve media and source text independently of the app, but also carry workflow machinery such as review reasons and metadata suggestions. Preserving the organizational idea does not yet decide whether all these fields survive the restart.

The existing image extractor selects hard-coded Wikimedia or X handling and falls back for other hosts. This supports investigating agent-selected retrieval strategies, but does not establish that Pi can access a blocked source: browser and retrieval capabilities still need to be supplied and tested.

## Pi findings and provisional recommendation

The upstream repository formerly at `badlogic/pi-mono` now redirects to [earendil-works/pi](https://github.com/earendil-works/pi). Its [official website](https://pi.dev/) presents interactive, print/JSON, RPC, and SDK integration modes and customization through extensions, skills, and prompts.

The [SDK documentation](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/sdk.md) documents `createAgentSession`, custom tools, system-prompt replacement, selectable built-in tools, and persistent or in-memory sessions. This makes a small TypeScript CLI embedding Pi feasible without an upstream fork. The [extension documentation](https://pi.dev/docs/latest/extensions) describes custom commands and tools, event hooks, and terminal interaction; an extension package is another viable starting point if Pi's own terminal interface is sufficient.

Accepted integration direction: embed upstream Pi through its SDK, letting the agent choose retrieval and classification steps while a trusted controller enforces archive operations. A fork becomes relevant only if a demonstrated requirement cannot be met through supported integration points. The controller/worker prototype must establish the concrete integration.

These are documentation findings, not a tested integration. No dependency has been installed and no capture has been run through Pi.

## Resolved: capture intent

The user prefers specific commands such as `gg painting` and `gg idea`, with a UI built on top later. Painting Capture almost never needs to preserve surrounding discussion. The explicit command resolves the artwork-versus-idea ambiguity; the earlier recommendation to infer that choice from a bare URL is no longer the default proposal. Later discussion fixes one Selected Image per Visual Capture; handling an ambiguous multi-image source remains open.

An optional Saving Reason can still refine a request, but is not required to distinguish the two commands. Reading a caption as evidence for artist or title is different from preserving the discussion; the precise evidence policy remains to be discussed.

## Resolved: research during Painting Capture

The user confirms that finding a better copy and reliable attribution beyond the supplied page was the original intended behavior. This research belongs within Painting Capture. The user does not yet have a preference for how to bound the research.

The Best Available File and Painting Capture glossary entries now reflect research during capture. ADR-0040 records the change to the earlier image-search deferral in ADR-0016 for URL capture; local-file import remains a separate undecided workflow.

Proposed stopping policy, not yet an accepted constraint: stop once a confidently matched, good-quality copy and credible attribution have been found, or further attempts stop producing useful evidence. Apply an outer elapsed-time and/or cost limit in the CLI so an unproductive search cannot run indefinitely. Prompt instructions guide research effort; the outer limit must be enforced by code. Calibrate defaults against real captures instead of choosing arbitrary numbers during the interview. Exact model costs, provider accounting, cancellation, and partial-result behavior remain untested.

## Resolved: records, uncertainty, and visual kinds

The user accepted the proposed metadata content and automatic preservation with unresolved facts when identification fails. They supplied five representative links, including photography and sculpture, and want the agent to distinguish those and choose home subvaults. The canonical workflow term is now Visual Capture; `gg art <url>` is a proposed broader command name, not yet an accepted spelling.

See [representative sources](source-examples.md) for links, observed source behavior, and proposed evaluation checks. All three X posts were readable in the available in-app browser despite failures through the web reader. This validates browser inspection in this environment only; it does not demonstrate a Pi browser integration or reliable downloads.

## Resolved: user-controlled areas and one Selected Image

The user alone creates and deletes vaults and subvaults, including setup defaults. Their examples of “vaults” were Paintings, Photography, Sculptures, and possibly Memes; these are Subvaults in the current glossary. The restriction is recorded for both vault roots and their subvaults. Capture chooses from the existing set.

The user will supply the intended image on X; quoted posts require separate captures. A capture is singular rather than automatically expanding a post's media into multiple saved items. Research must improve the exact Selected Image, including for sculpture; alternative views and grouping multiple instances of one artwork are deferred.

This replaces the earlier suggestions to let the agent create new broad areas, save every main-post attachment, or add other sculpture viewpoints. It does not yet resolve storage of original-plus-better copies of the same image, repeated captures of the same image, or ambiguous sources.

Implementation implication for the future prototype: expose capture operations that can save into existing permitted areas, and keep vault/subvault setup operations outside the agent's tool authority. The prohibition must not rely solely on a prompt if unrestricted file or shell tools could bypass it. Exact tooling remains undecided.

## Resolved: queued decisions, repeated images, and Idea Sources

The user wants captures with no suitable existing subvault flagged for a later queue, where they can approve or give the agent instructions. This replaces the earlier recommendation to pause for an immediate destination choice. Durable queue layout, retention of any fetched material, and the scope of other queue reasons remain open; the general no-database constraint still applies.

On a repeated capture, the user wants a better-quality version chosen if available and the existing item's location returned otherwise. The agent must establish that candidates are the same Selected Image; dimensions or byte size alone do not establish either identity or quality. Automatic upgrades are authorized for a demonstrably better copy of the same image. The later retention decision keeps earlier preserved files alongside an improved primary; uncertain identity and quality comparisons still need detail. This reopens the old blanket approval requirement for changing a Primary File in that narrow case.

Idea Sources are not restricted to X: direct articles, news, and YouTube videos are explicit use cases. The user generally accepts preserving a post and a linked article together, with source text and links, but stresses that the Summary is the most important part. The later summary/transcript decision below resolves the previously open fallback behavior.

The user recalled problems with the previous agent's treatment of existing paintings, possibly WebP or file size, and asked that this inform the replacement. Read-only inspection found concrete historical preview-limit failures and separate incomplete provider responses; see [existing-archive acceptance fixtures](source-examples.md#existing-archive-acceptance-fixtures). This turn did not reproduce the full historical workflow, change the old implementation, or run a model request. The scope is gathering regression evidence for the replacement, so the diagnosis skill's reproduction/fix phases remain deferred rather than being claimed complete.

## Resolved: findable ideas, usable instructions, and a clean restart

The user wants to retrieve something they read when a specific issue arises or a concept comes to mind. Summaries therefore need concrete problem and concept vocabulary. For instructions and advice, the summary should contain the instructions themselves with narrative removed, retaining steps, prerequisites, parameters, conditions, and verification details when needed to carry them out. For non-instructional material, retain the central ideas; do not invent a procedure that the source does not contain. The preserved source text remains available separately.

When a YouTube video has no usable transcript, skip and notify the user. Do not transcribe audio, create an incomplete item, or add it to the Capture Queue. This explicitly replaces the prior proposed transcription fallback and retry queue for that case.

The user is not requesting an existing-archive importer or cleanup project. They want to discard most of the old implementation and rebuild, potentially carrying forward image-format handling and small-thumbnail utilities. Treat existing files as useful compatibility fixtures, not a migration obligation or reason to retain the Rust core and Tauri shell. Actual repository replacement remains a later implementation step; no implementation files or live archive files have been removed during the design interview.

## Resolved: retrieval, configurable providers, and source age

The user requires both `gg search` and `gg ask`. They want to describe a topic naturally and receive the saved files that touch on it, including conceptually related material expressed with different words. Their example is finding files about managing customers using email-management agents. They do not want to edit existing files to support this experience, and retrieval leaves those files unchanged. Proposed implementation: a deterministic local search over records, exposed as a human CLI and bounded machine-readable results containing paths, snippets, and dates. GG's Pi session or an existing coding agent can expand query terms and inspect candidate records and source text where needed. An existing agent using the search tool need not launch another GG model session. Literal search alone can miss synonyms; retrieval quality needs evaluation before claiming that query expansion suffices. These mechanisms are recommendations, not a built or evaluated search system.

The user wants OpenAI plus their existing OpenRouter credits, with editable model slugs and credentials through a settings wizard. [Pi providers](https://pi.dev/docs/latest/providers) already include both. Its [model configuration](https://pi.dev/docs/latest/models) supports custom endpoints, API formats, and declared input capabilities. [OpenRouter documents OpenAI SDK compatibility](https://openrouter.ai/docs/guides/community/openai-sdk), although GG can use Pi's integration instead of building another SDK adapter. This supersedes the OpenAI-only first implementation in ADR-0028; credentials remain outside the vault under ADR-0027.

The earlier model-specific documentation lookup was exploratory evidence, not a product decision. The user clarifies that no Luna or DeepSeek release should be hardcoded: provider, API key, and model identifier are editable configuration. Manual model entry must remain available even when a catalog has not caught up with a release. A newly released model using a supported provider interface should not require an application-code change. Different tool and image capabilities still need handling; compatibility should be evaluated for the configured model rather than inferred from a fixed list of names. Automatic fallback to a different model has not been agreed or tested.

The supplied [YouTube video](https://www.youtube.com/watch?v=xJaMTo2YgO8) was inspected through the browser. Its expanded description reports publication on 2026-05-27. Existing English auto-generated captions were retrieved and read; no audio transcription was needed. A [short sample summary](idea-summary-example.md) is for discussion only, not a saved vault item or a validated current workflow. Transcript availability in this environment does not establish a standalone Pi retrieval implementation.

The user wants timestamps or an age indicator so they can judge the age of captured advice, particularly in fast-changing areas such as AI. Visible publication and capture dates suffice, with unknown source dates kept unknown. They are not actively updating these ideas. Rechecking, last-checked fields, automatic expiry, and updating old summaries are outside the requested workflow.

Optional summary scoping is accepted. A user can supply a concern to focus a new capture, but need not annotate or edit existing notes. Scoping the summary does not truncate the preserved source text; the proposed retrieval tool should also search source text when the relevant concept was omitted from a summary. The previous sample was intended to illustrate generated detail, not request manual curation; its exact prose is not an approved output template. Evaluate the pipeline by whether it can retrieve useful saved files for a natural-language problem, as well as whether instructional summaries preserve usable steps.

## Next work

The user's agreement to the recommended architecture resolves the main design interview. Use the build sequence below to obtain evidence, and reopen a product decision only when implementation reveals a material tradeoff. Runtime selection, concrete record fields, command spelling, limits, and deployment details do not require more hypothetical interview rounds before the prototype.

## Resolved: foreground batches, ambiguous images, and public access

The user accepts foreground CLI execution for simplicity, while expecting to paste multiple links, especially once a GUI exists. Batch input is therefore required from the beginning. Proposed syntax is multiple URL arguments and a newline-separated `--stdin` mode on each capture command. Each command keeps its explicit capture intent; a mixed visual/idea batch format has not been requested. Sequential processing, individual outcomes, continued processing after a per-entry failure or queued decision, and structured progress events are implementation recommendations for a first version. They do not imply a daemon, persistent background scheduler, or all-or-nothing batch transaction. Interrupted-batch behavior and exact progress/result formats remain to be decided.

The user confirms that an ambiguous image selection goes to the Capture Queue. Proposed queue presentation includes source link, reason, and candidate previews so the user can choose and continue later; the layout and resume commands remain implementation work. Queuing one ambiguous input should allow the remaining batch entries to proceed.

The user explicitly declines source-site authentication initially, acknowledging that some sources will be inaccessible. Do not implement website login flows, manage source credentials, import browser cookies, or reuse a logged-in browser profile. The OpenAI/OpenRouter model credentials remain in scope. Earlier browser access observations are not a guarantee that a fresh unauthenticated Pi integration can access the same content. The user confirms inaccessible-source behavior: report a clear per-link failure, include the URL in the batch result, and continue without creating an incomplete saved item or Capture Queue entry. The existing no-transcript policy remains skip-and-notify without a queue entry.

## Resolved: preserve both image versions

The user wants to retain the initially obtained image and a higher-quality version of the same Selected Image. The expected archive size does not justify discarding those versions to save storage. Keep them in one saved item, with the chosen best file as Primary File; do not create multiple artwork items just to represent quality variants. Earlier preserved files survive subsequent upgrades. Exact filenames, provenance fields, and quality-comparison logic remain implementation details to resolve.

## Resolved: agent isolation and credentials

The user accepts the recommended architecture after discussing container isolation and hosted use. The following describes the agreed separation; its implementation and effectiveness have not been tested.

Run Pi and its browsing/download/image tools in one unprivileged worker container, with a small trusted GG controller outside it. The worker gets a disposable writable scratch area, without mounts of the user's home, credentials, or live vault. The controller owns user settings, model credentials, archive search/read operations, and validated archive writes. The controller can run as the ordinary host CLI process initially; if it is packaged in a container later, use separately configured sibling containers rather than a Docker daemon inside the worker.

Model requests cross a narrow gateway: the controller supplies the real API key only when forwarding to the user-configured provider endpoint. The worker receives at most a per-job gateway credential, never the real provider key in environment variables, files, prompts, or tool results. The gateway must limit each job's request scope and consumption, prohibit agent-selected upstream destinations, strip upstream credentials from errors/logs, and support the relevant streaming API formats. Hiding the raw key does not by itself prevent spending through the gateway. Provider/model selection stays user configuration rather than a fixed model list.

Saving also crosses a narrow interface. The worker submits staged files and a proposed record referring to an existing allowed destination. The controller validates records and files and performs the ordinary automatic save; this does not add a human approval step to successful captures. It rejects destination creation/deletion, arbitrary paths, symlinks, and changes outside the specific operation. `gg ask` gets search/read operations only and no archive-write capability. The worker must not control the container runtime socket, host processes, the gateway configuration, or the controller's configuration. Only bundled trusted Pi extensions/configuration should load; downloaded content must not become executable host configuration.

Container isolation and network access require separate controls. Public retrieval should not give browser pages or scripts access to private host/LAN services. Network restrictions and the gateway transport remain prototype work. Containers do not prevent disclosure of data legitimately supplied to a network-capable worker or to the configured model provider, and they are not a VM-strength isolation guarantee. Avoid privileged containers, host namespaces, or mounting a runtime socket into the worker.

Documentation checked on 2026-09-10: [Pi's SDK](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/sdk.md) allows replacing/selecting tools and controlling resource discovery, and [custom model configuration](https://pi.dev/docs/latest/models) supports custom provider endpoints. Those are promising integration points for the controller without forking Pi; gateway streaming and compatibility have not been tested. [Docker rootless mode](https://docs.docker.com/engine/security/rootless/) reduces daemon/runtime privilege. [Docker security](https://docs.docker.com/engine/security/) explains configuration and kernel risks; [container runtime privileges](https://docs.docker.com/engine/containers/run/#runtime-privilege-and-linux-capabilities) explain why a privileged nested runtime is unsuitable here. [Playwright's Docker guidance](https://playwright.dev/docs/docker) distinguishes its default testing image from browser execution against untrusted sites and calls for a separate user and a suitable seccomp profile when crawling. Do not disable Chromium's sandbox merely to get a prototype running.

This adds meaningful but bounded implementation work: a reproducible worker image/launcher, the model gateway, the archive interface, lifecycle/cancellation handling, and isolation tests. Browsing/image dependencies and provider streaming are the largest unknowns, so do not promise a time estimate before the first integration test. Host inspection found Docker, Podman, and bubblewrap executables on this Linux machine, but did not verify a running daemon or rootless configuration. No runtime installation, image build, network policy change, or credential access was performed.

Suggested first prototype: one public capture through Pi in a fresh worker, a model request through the gateway, and a validated save into a disposable test vault. Verify that worker code cannot read the host home, obtain the real API key, write directly to the vault, create/delete subvaults through the controller, or exceed the enforced job limits. Repeat with the existing WebP/large-JPEG fixtures and a failed link followed by a successful batch entry. These are future checks, not completed tests.

## Resolved: sandboxed shell tools and scripts

The user accepts both ordinary shell utilities such as sed/awk and agent-written scripts. For example, the agent can parse a retrieved page or process downloaded data with a short Python or JavaScript program in its temporary workspace. Script execution remains inside the proposed sandbox; this acceptance does not settle the container runtime, gateway transport, networking rules, package installation policy, or production isolation strength.

## Resolved: private hosted browser access

The user agrees with the recommended private personal browser application and one authoritative writable server vault with a local backup. The browser supports capture, search/ask, viewing, and resolving queued decisions, consistent with the earlier GUI-over-CLI direction. This is architectural planning, not authorization to deploy, publish archive content, or move the live vault. The hosting provider, private-access mechanism, and backup procedure remain implementation choices.

The controller/worker split will also run on the server: the browser communicates with a GG service, that service holds model credentials and controls filesystem writes, and sandboxed Pi workers handle capture or retrieval. Keep capture/search/ask operations in reusable code beneath both the CLI and server interface. The GUI must not construct unrestricted shell commands from user input, and it must not receive provider API keys. Source-site authentication remains excluded; private access to the user's own GG service is required separately.

For full capture and ask from the browser, the main additions are an authenticated web interface, media/record endpoints, independent processing jobs, progress delivery, and deployment/backup management. Closing a tab or losing a connection should not cancel a submitted capture. Persist job identifiers, status, and outcomes as small files so results can be retrieved after reconnecting; on server interruption, report/recover unfinished work without silently duplicating completed items. This processing state is distinct from the Capture Queue, which is reserved for user decisions. A single GG service with a serialized vault writer and bounded workers is a reasonable initial hosted design; it does not require a database or distributed job system. These job mechanics are proposals for the hosted phase, not an expansion of the initial foreground CLI into a daemon.

The vault remains an ordinary directory on persistent server storage, with the same shallow layout and Markdown records. Worker containers must never own the sole durable copy. If the controller is containerized, explicitly mount the archive's persistent directory and back it up independently. Use one authoritative writable server vault plus a local backup/export; hosted CLI and GUI mutations go through the server controller. Bidirectional editing between independent local and server copies is outside the design. No migration of `/home/cem/Gewolbe` is authorized by this discussion.

Public publishing and separate accounts/archives for other people are outside this personal hosted scope.

Checked documentation on 2026-09-10: [Pi's SDK](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/sdk.md) supports application embedding and event streaming. [Docker storage documentation](https://docs.docker.com/engine/storage/volumes/) distinguishes persistent mounts from a container's disposable writable layer; a bind-mounted ordinary directory is suitable when host file access is required. [Server-sent events](https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events/Using_server-sent_events) are one candidate for browser progress delivery, but do not themselves make jobs durable. [Browser XSS guidance](https://developer.mozilla.org/en-US/docs/Web/Security/Attacks/XSS) informs safe rendering of captured text/Markdown/media; archive content is untrusted input to the GUI. The implementation, authentication mechanism, deployment target, and retrieval success from a hosted IP remain untested.

## Build sequence and evidence gates

1. **Prove one complete isolated capture.** Build a small disposable test setup using Pi's SDK, a worker container, the credential gateway, and the controller's save operation. Start with a public Wikimedia image and a test vault. Demonstrate image plus Markdown preservation, streaming model requests, cancellation, and actual inability to read host secrets or bypass destination rules. Confirm both supported provider integrations without embedding a fixed model list. This establishes the integration before replacing the repository implementation.
2. **Test the difficult inputs.** Run the supplied X, Wikipedia, Wikimedia, and YouTube cases through the standalone worker. Distinguish inaccessible inputs from broken retrieval. Exercise WebP and large-JPEG image preparation, the missing-transcript skip, ambiguous-image queuing, same-image upgrades with version retention, and failed entries followed by successful batch entries. Calibrate time and token limits from these runs; do not claim a currency cap is exact without reliable accounting for the configured model.
3. **Prove rediscovery.** On a disposable set of captured or synthetic items, implement deterministic file search plus `gg ask`. Try natural-language queries using different wording from the saved summaries, including the user's customer/email-agent example with appropriately relevant fixtures. Return existing paths and relevance explanations, reading preserved source text where needed. Ensure the agent reports no useful match when appropriate and leaves the archive unchanged. Retrieval quality may justify revisiting the search implementation, without presupposing a database or embedding store.
4. **Build the clean CLI.** After integration evidence is satisfactory, retain a recoverable snapshot of the existing repository, carry forward the accepted docs/fixtures and only useful utilities, and replace the old implementation. Add the settings wizard, user-controlled vault/subvault setup, explicit capture intents, multiple URL arguments and pasted-list input, queue resolution, stable result formats, and repeatable/cancellable operations. Settle a minimal record format and readable file layout from the tested examples. Existing-vault import or bulk migration remains outside the first version.
5. **Add the private hosted application.** Reuse the same operations behind a server and GUI, add durable processing status/progress, private access, file previews, and persistent server storage. Test tab disconnection, duplicate submission, interrupted saves, backup restoration, and public-source access from the chosen host. Keep one writable server authority and a local backup. Deployment and any data movement are separate future actions.

The sequence above was the agreed plan at the end of the interview. The prototype update below records subsequent implementation evidence; clean CLI replacement and hosted application remain future work.


## Prototype implementation update — 2026-09-10

The user authorized a branch and sustained work through a finished prototype. Branch `codex/pi-vault-prototype` adds the independent package at [prototypes/pi-vault](../../prototypes/pi-vault/README.md). Pi 0.85.1 is embedded through its SDK without a fork. Rootless Podman/crun runs the browser, shell tools, and image processing; an inherited private descriptor connects to the trusted controller. Provider credentials and the archive stay outside the worker. Earlier “untested” statements above describe the design-stage state; current results are in [prototype findings](../../prototypes/pi-vault/NOTES.md) and [recorded results](../../prototypes/pi-vault/reports/results.json).

The prototype demonstrated live OpenAI Responses streaming, both supplied Wikimedia/Wikipedia painting captures, article capture, deterministic search, and natural-language retrieval of the three relevant customer/email fixture notes with a no-match control. Three real archive image fixtures, including 8000×8000 WebP and a 31 MB JPEG, passed byte-preservation and bounded-preview checks. Automated checks cover controller validation, read-only ask, private-network rejection, real Pi protocol integration, and deadline cancellation. OpenRouter used real Pi against a mock upstream; live provider validation remains open.

All three supplied X posts returned HTTP 403 in the standalone public worker. The YouTube page loaded but its transcript was not retrieved; earlier interactive caption access means this is an extraction/access limitation rather than proof of no captions. These inputs were skipped without items or queue entries. No source-site authentication was introduced.

The existing repository implementation and `/home/cem/Gewolbe` were preserved. The clean CLI settings wizard, queue resolution, repository replacement, production artifact validation, and private hosted GUI remain subsequent work. No live-vault import, deployment, or destructive reset was performed.
