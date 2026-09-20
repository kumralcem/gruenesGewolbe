// Keep controller diagnostics available without making every command print its protocol.
export function formatResult(value: any, detailed = false): string {
  if (typeof value === "string") return value;
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
