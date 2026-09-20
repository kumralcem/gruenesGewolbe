// Keep controller diagnostics available without making every command print its protocol.
export function formatResult(value: any, detailed = false): string {
  if (typeof value === "string") return value;
  if (!detailed && value?.usage?.windows) return formatUsage(value);
  if (!detailed && value?.jobs && value?.actions)
    return [
      "Model usage by job (last 7 days):",
      ...value.jobs.map(
        (j: any) =>
          `${new Date(j.lastAt).toLocaleString()} | ${j.requests} requests | ${j.tokens.toLocaleString()} tokens | ${j.job} | ${j.input ?? "older usage"}`,
      ),
      "",
      "Budget changes:",
      ...value.actions
        .slice(-20)
        .map(
          (a: any) =>
            `${new Date(a.at).toLocaleString()} ${a.action}: ${JSON.stringify(a.detail)}`,
        ),
    ].join("\n");
  if (detailed || !value || typeof value !== "object" || !("exitCode" in value))
    return JSON.stringify(value, null, 2);
  const lines: string[] = [];
  if (value.answer) lines.push(value.answer);
  for (const answer of value.answers ?? [])
    lines.push(
      `${answer.title ?? answer.id}\n${answer.path}${answer.reason ? `\n${answer.reason}` : ""}`,
    );
  const outcomes = value.outcomes?.length
    ? value.outcomes
    : value.outcome
      ? [value.outcome]
      : [];
  for (const outcome of outcomes)
    lines.push(
      `${outcome.status}${outcome.path ? `: ${outcome.path}` : ""}${outcome.reason ? ` — ${outcome.reason}` : ""}`,
    );
  // Aggregate partial/failure reasons may contain information absent from individual saves.
  if (value.outcomes?.length && value.outcome?.reason)
    lines.push(`${value.outcome.status}: ${value.outcome.reason}`);
  for (const operation of value.management ?? []) {
    if (operation.requiresConfirmation)
      lines.push(
        `Confirmation required:\n${JSON.stringify(operation.preview, null, 2)}\nRun: gg confirm ${operation.proposalId}`,
      );
  }
  if (
    value.management?.some((operation: any) => operation.operationId) &&
    value.batchId
  )
    lines.push(`Undo: gg undo ${value.batchId}`);
  for (const warning of value.usageWarnings ?? [])
    lines.push(`Warning: ${warning}`);
  for (const problem of value.vaultProblems ?? [])
    lines.push(`Vault problem: ${problem.path} — ${problem.reason}`);
  const toolErrors = (value.events ?? []).filter(
    (event: any) => event.type === "tool_error",
  );
  if (toolErrors.length)
    lines.push(
      `${toolErrors.length} tool attempt(s) failed; use --verbose for details.`,
    );
  if (value.exitCode && !value.outcome)
    lines.push(`Job failed (exit ${value.exitCode}).`);
  if (value.exitCode && value.stderr) lines.push(value.stderr);
  if (!lines.length)
    lines.push(
      value.events?.some((event: any) => event.type === "probe")
        ? "Isolation probe passed."
        : "Done.",
    );
  return lines.join("\n\n");
}

export function startProgress(enabled: boolean): () => void {
  if (!enabled) return () => {};
  const start = Date.now();
  const show = () =>
    process.stderr.write(
      `\r\u001b[2KGG is working… ${Math.floor((Date.now() - start) / 1000)}s`,
    );
  show();
  const timer = setInterval(show, 1000);
  timer.unref();
  return () => {
    clearInterval(timer);
    process.stderr.write("\r\u001b[2K");
  };
}

function formatUsage(value: any): string {
  const u = value.usage,
    p = u.providers[value.provider] ?? {},
    a = p.allowance;
  const lines = [
    `GG usage — ${value.provider} / ${value.model}`,
    "",
    "Application limits (rolling windows):",
  ];
  for (const w of u.windows) {
    lines.push(
      `  ${w.name}: ${w.usedRequests}/${w.requests} requests; ${w.usedTokens.toLocaleString()}/${w.tokens.toLocaleString()} tokens`,
    );
    lines.push(
      `    Remaining: ${Math.max(0, w.requests - w.usedRequests)} requests, ${Math.max(0, w.tokens - w.usedTokens).toLocaleString()} tokens`,
    );
    if (w.nextReset)
      lines.push(
        `    Oldest usage expires: ${new Date(w.nextReset).toLocaleString()} (capacity returns gradually)`,
      );
    if (w.usedRequests >= w.requests || w.usedTokens >= w.tokens)
      lines.push("    LIMIT REACHED");
  }
  lines.push(
    "",
    a
      ? `Subscription: ${100 - a.usedPercent}% remaining (last reported ${new Date(a.observedAt).toLocaleString()})${a.resetAt ? `; provider reset ${new Date(a.resetAt).toLocaleString()}` : ""}`
      : "Subscription: unavailable; GG counters are separate.",
  );
  if (p.paused)
    lines.push(
      `Provider paused: ${p.paused}. After fixing the cause: gg resume`,
    );
  if (u.override)
    lines.push(
      `Temporary grant: ${u.override.requests} extra requests, ${u.override.tokens.toLocaleString()} tokens; expires ${new Date(u.override.expires).toLocaleString()}`,
    );
  for (const [id, g] of Object.entries(u.grants ?? {}) as [string, any][])
    lines.push(
      `Collection grant ${id}: ${g.requests} requests, ${g.tokens.toLocaleString()} tokens, ${g.sources.length} images; ${g.expires > Date.now() ? "expires" : "expired"} ${new Date(g.expires).toLocaleString()}`,
    );
  if (value.grantId)
    lines.push(`Use with: gg import PATH --local --grant ${value.grantId}`);
  lines.push(
    "",
    "Commands:",
    "  gg usage reset --window hour|week|all  (GG counters only; history retained)",
    "  gg usage grant --requests 50 --tokens 1000000 --minutes 60",
    "  gg usage grant --collection PATH --requests N --tokens N --minutes N",
    "  gg usage revoke GRANT_ID",
    "  gg usage limits --hour-requests N --week-requests N --hour-tokens N --week-tokens N",
    "  gg usage history | gg usage --watch | gg usage --json",
    "Limits and grants never reset your provider's allowance. Token totals include conservative reservations until actual usage is reported.",
  );
  return lines.join("\n");
}
