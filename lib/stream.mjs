// The env a nested `claude -p` must not inherit, a fold of its stream-json events into per-run
// metrics, and the answer a run ends on.

// Nested `claude` refuses to start, or attaches to this session, with these set.
export const STRIP_ENV = [
  "CLAUDECODE", "CLAUDE_CODE_ENTRYPOINT", "CLAUDE_CODE_SESSION_ID", "CLAUDE_CODE_CHILD_SESSION",
  "CLAUDE_CODE_BRIDGE_SESSION_ID", "CLAUDE_CODE_MESSAGING_SOCKET", "CLAUDE_CODE_MESSAGING_TOKEN",
  "CLAUDE_CODE_SESSION_ATTENDED", "CLAUDE_CODE_EXECPATH", "CLAUDE_PID", "CLAUDE_EFFORT", "AI_AGENT",
];

export function childEnv(extra = {}) {
  const env = { ...process.env };
  for (const k of STRIP_ENV) delete env[k];
  return { ...env, ...extra };
}

// Folds stream-json events into the per-run metrics. Events may carry `_t` (ms since the run
// started, stamped by the reader) for timing fields.
export function summarize(events) {
  const m = {
    model: null, per_turn_effort_active: null,
    tool_calls: 0, tool_calls_by_tool: {}, repeated_calls: 0, retries_after_error: 0, first_tool_ms: null,
    thinking_blocks: 0, tool_errors: 0, tool_result_chars: 0, turns: null, assistant_messages: 0,
    usage: null, model_usage: null, cost_usd: null, duration_ms: null, duration_api_ms: null,
    result_subtype: null, is_error: null, final_text: null,
  };
  let lastSig = null;
  let lastName = null;
  let lastErrored = false;
  for (const e of events) {
    if (e.type === "system" && e.subtype === "init") {
      m.model = e.model;
      m.per_turn_effort_active = e.per_turn_effort_active ?? null;
    } else if (e.type === "assistant") {
      m.assistant_messages++;
      for (const b of e.message?.content ?? []) {
        if (b.type === "thinking" || b.type === "redacted_thinking") m.thinking_blocks++;
        if (b.type !== "tool_use") continue;
        m.tool_calls++;
        if (m.first_tool_ms == null && e._t != null) m.first_tool_ms = e._t;
        m.tool_calls_by_tool[b.name] = (m.tool_calls_by_tool[b.name] ?? 0) + 1;
        // The same call with the same input as the one just before it (a retry or a stuck loop).
        // retries_after_error: the call right after a failed call used the same tool.
        const sig = b.name + JSON.stringify(b.input ?? {});
        if (sig === lastSig) m.repeated_calls++;
        if (lastErrored && b.name === lastName) m.retries_after_error++;
        lastSig = sig;
        lastName = b.name;
        lastErrored = false;
      }
    } else if (e.type === "user") {
      const content = e.message?.content;
      for (const b of Array.isArray(content) ? content : []) {
        if (b.type !== "tool_result") continue;
        // Size of what the tools fed back into the context.
        for (const c of Array.isArray(b.content) ? b.content : [{ type: "text", text: String(b.content ?? "") }]) m.tool_result_chars += (c.text ?? "").length;
        if (b.is_error) {
          m.tool_errors++;
          lastErrored = true;
        }
      }
    } else if (e.type === "result") {
      m.turns = e.num_turns;
      m.usage = e.usage
        ? {
            input_tokens: e.usage.input_tokens ?? 0,
            output_tokens: e.usage.output_tokens ?? 0,
            cache_creation_input_tokens: e.usage.cache_creation_input_tokens ?? 0,
            cache_read_input_tokens: e.usage.cache_read_input_tokens ?? 0,
          }
        : null;
      m.model_usage = e.modelUsage ?? null;
      m.cost_usd = e.total_cost_usd ?? null;
      m.duration_ms = e.duration_ms ?? null;
      m.duration_api_ms = e.duration_api_ms ?? null;
      m.result_subtype = e.subtype;
      m.is_error = e.is_error;
      m.final_text = typeof e.result === "string" ? e.result : null;
    }
  }
  return m;
}

// The run's top-level tool calls, which the lookup guard reads.
export const traceOf = (events) =>
  events.filter((e) => e.type === "assistant" && !e.parent_tool_use_id).flatMap((e) => (e.message?.content ?? []).filter((b) => b.type === "tool_use").map((b) => ({ name: b.name, input: b.input })));

// The last JSON object in the text (every prompt asks for one on the final line).
export function lastJson(text) {
  if (!text) return null;
  const fenced = [...text.matchAll(/```(?:json)?\s*([\s\S]*?)```/g)].map((m) => m[1]);
  for (const body of fenced.reverse()) {
    try {
      const v = JSON.parse(body.trim());
      if (v && typeof v === "object") return v;
    } catch {}
  }
  for (let end = text.lastIndexOf("}"); end >= 0; end = text.lastIndexOf("}", end - 1)) {
    let depth = 0;
    for (let i = end; i >= 0; i--) {
      if (text[i] === "}") depth++;
      else if (text[i] === "{" && --depth === 0) {
        try {
          const v = JSON.parse(text.slice(i, end + 1));
          if (v && typeof v === "object") return v;
        } catch {}
        break;
      }
    }
  }
  return null;
}

// The answer is the last JSON in the final message, or in an earlier message when the final one
// has none.
export function answerOf(finalText, events) {
  const fromFinal = lastJson(finalText);
  if (fromFinal) return fromFinal;
  let answer = null;
  for (const e of events)
    if (e.type === "assistant" && !e.parent_tool_use_id)
      for (const b of e.message?.content ?? []) if (b.type === "text") answer = lastJson(b.text) ?? answer;
  return answer;
}
