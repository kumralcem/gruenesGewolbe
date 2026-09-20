# CLI and usage budgets

[Home](../README.md) · [Setup](getting-started.md) · [File transfers](files.md)

## Commands

| Command                                                  | Purpose                                                                                                          |
| -------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| `init`, `configure`                                      | Create a vault and save controller settings                                                                      |
| `login`, `logout`, `models`                              | Pi provider credentials and model catalog                                                                        |
| `serve`                                                  | Run the loopback receiver and background capture queue                                                           |
| `pair --scope capture\|manage`, `devices`, `revoke ID`   | Pair and revoke individual devices                                                                               |
| `connect URL`                                            | Pair a remote management CLI; prompts for a code                                                                 |
| `capture URL... [--instructions TEXT] [--stdin]`         | Public URL capture; browser capture handles signed-in pages                                                      |
| `download RECORD_ID --to FILE.tar.gz [--originals-only]` | Download a portable record bundle locally or from the paired server; existing output files are never overwritten |
| `import PATH... [--instructions TEXT]`                   | Import supported UTF-8 text and JPEG/PNG/WebP files or directories, locally or to the paired server              |
| `capture-file FILE...`                                   | Process saved browser snapshots locally                                                                          |
| `search QUERY`, `ask QUESTION`                           | File search or model-assisted read-only retrieval                                                                |
| `do INSTRUCTIONS`, `chat`                                | Natural-language vault management                                                                                |
| `list`, `history`, `undo [ID]`, `confirm ID`             | Inspect records, history, restoration and approvals                                                              |
| `usage`, `resume`                                        | Inspect limits or clear a provider pause after fixing its cause                                                  |
| `override --requests N --tokens N [--minutes N]`         | Explicit temporary extra budget, up to 50 requests / 1M tokens / 60 minutes                                      |
| `provider PROVIDER MODEL`                                | Explicitly switch provider and model                                                                             |
| `index`, `probe`, `queue`                                | Rebuild Obsidian index, test isolation, inspect legacy decision queue                                            |

A paired CLI routes archive commands to the server; `--local` selects local operations. Setup, sign-in, device management, `capture-file`, `index`, `probe` and legacy `art`/`painting`/`idea` operations run locally. Legacy commands retain the previous single-image/idea semantics; use `capture` for new behavior. `gg help` lists flags. Agent commands print concise answers, saved paths, undo commands, confirmation previews and warnings. Add `--verbose` or `--json` for the full diagnostic result. Interactive terminals show elapsed time while waiting; `--json` suppresses that indicator for scripts. Other administrative commands retain structured output. The daemon keeps the controller running but starts a fresh isolated worker for each agent job; model requests and worker startup still take time.

## Usage controls

`gg usage` shows readable rolling limits, remaining capacity, the last subscription observation and recovery commands. `--json` retains structured output; `--watch` refreshes every two seconds. Subscription readings are observations, not a live promise of remaining capacity. GG cannot reset a provider's subscription allowance.

```sh
gg usage reset --window hour              # or week / all; preserves request history
gg usage grant --requests 50 --tokens 1000000 --minutes 60
gg usage limits --hour-requests 100 --week-requests 1000
gg usage history                         # requests/tokens by job, plus budget changes
```

Limits persist in controller state and apply to running controllers on the next reservation. Resets affect GG's chosen window across providers; they do not clear provider-error pauses or erase accounting. Request history retains seven days; older requests lack job attribution. Failed attempts retain their conservative token reservation. A job may need more tokens than the displayed remainder to start its next model request.

For a substantial import, authorize a **collection-specific allowance** on the server:

```sh
gg usage grant --local --collection ~/Imports/Collection \
  --requests 100 --tokens 5000000 --minutes 120
# Copy the returned grant ID:
gg import ~/Imports/Collection --local --grant GRANT_ID \
  --instructions "Organize these under References" --continue-on-error
gg usage revoke GRANT_ID
```

This allowance accepts only the exact file hashes scanned from that collection. It consumes its own bounded request/token pool instead of the normal hourly/weekly enforcement, while retaining ordinary accounting, provider pauses, deadlines and per-job limits. Successful requests settle against actual reported tokens; failed attempts keep their reservation. Maximum grant: 10,000 requests, 2B reserved tokens, 24 hours, 2,000 file hashes. Collection grants currently work with server-local imports; ordinary remote imports remain supported. Grants are never renewed automatically. `--continue-on-error` continues past unsuccessful model results but still stops on usage/provider pauses or input/transport errors. Check the log for unfinished files before deleting staging files.

## Limits

All model work in one controller state directory shares a durable single-job lock and rolling budgets: **50 requests/hour, 300/week, 500,000 tokens/hour, 3,000,000/week**. Warnings start at 80%; new requests pause at the cap. Counters include retries and survive restarts. At most two delayed transient retries; authentication/quota failures pause immediately, and three consecutive provider failures open the circuit breaker. Resume explicitly after correcting the cause. No model calls occur simply to read usage.

Application counters are not subscription percentages. When Codex reports allowance headers, GG displays their last observation; otherwise it says **unavailable**. Usage outside GG is not included in application counters. API calls reserve estimated input plus the requested output cap; the current Pi Codex adapter does not send an output-token cap, so subscription calls reserve the model's full catalog output allowance. Successful reported usage replaces the reservation; failures retain it. Estimates and timeouts limit exposure, not exact spend.

`--config /private/settings.json` supports `provider`, `model`, `vision`, `maxRequests` (10/job), `maxSeconds` (180/job), `maxInputTokens` (64,000 estimated), `maxOutputTokens` (4,096 where supported), and `limits: {hourRequests, weekRequests, hourTokens, weekTokens}`. Run `gg configure --config FILE` to persist settings without an API key. Use the same state directory for every controller command; separate state directories have separate budgets.

Workers have no direct network, provider credentials, vault mounts or runtime socket. Resources are bounded at 2 CPUs, 2 GiB RAM, 256 processes, 150 MB fetched content and 64 MB per image. Image dimensions/fingerprints remain worker-supplied with controller structural validation; this is not a proof against arbitrary compromised-worker behavior.

Receiver uploads are capped at 90 MB, two concurrent uploads, 20 queued jobs and 256 MB retained inputs. Pending jobs resume after restart; interrupted jobs require retry. Cancellation stops unfinished work and retains already-saved records. Paused jobs stay on disk. Failed/partial/cancelled snapshots also remain for recovery; while stopped, remove unneeded job directories to reclaim retained-input space. After a hard crash, verify the writer/receiver is stopped before removing stale vault locks. Also inspect stale state-directory `.reclaim` gates before removing them; never remove locks held by live processes. Inspect incomplete history entries before manual recovery.

New folders during capture/import require `--create-destination PATH` (repeatable, at most 16 paths; list missing parents first). This permission is separate from `--instructions`. New vault artifacts use private file/directory permissions regardless of shell umask. Existing shared files are not automatically chmodded.
