/** A server's player count over time, drawn as an SVG path (the same chart as on zatlas.app). */

export interface Chart { line: string; area: string }

/**
 * The count over time as a step line: a sample holds until the next one (the API keeps a sample when the count changes
 * and every half hour), and the last holds to `to`. Samples before `from` are clipped to it. `max` is the top of the
 * chart; times in milliseconds.
 */
export function chart(history: { t: number; players: number }[], from: number, to: number, max: number, width: number, height: number): Chart {
  const top = Math.max(1, max, ...history.map((h) => h.players));
  const x = (t: number) => ((Math.min(to, Math.max(from, t)) - from) / (to - from)) * width;
  const y = (n: number) => height - (n / top) * height;
  const pts = history.filter((h) => h.t <= to).sort((a, b) => a.t - b.t);
  if (!pts.length) return { line: "", area: "" };
  // the sample in force at `from` starts the line at the left edge
  const inside = pts.filter((p) => p.t >= from);
  const before = pts.filter((p) => p.t < from).at(-1);
  const seq = before ? [{ t: from, players: before.players }, ...inside] : inside;
  let line = `M${x(seq[0].t).toFixed(1)} ${y(seq[0].players).toFixed(1)}`;
  for (let i = 1; i < seq.length; i++) line += ` H${x(seq[i].t).toFixed(1)} V${y(seq[i].players).toFixed(1)}`;
  line += ` H${width}`;
  return { line, area: `${line} V${height} H${x(seq[0].t).toFixed(1)} Z` };
}

/** The count in force at time `t` (the last sample at or before it). */
export function countAt(history: { t: number; players: number }[], t: number): number | null {
  let found: { t: number; players: number } | null = null;
  for (const h of history) if (h.t <= t && (!found || h.t > found.t)) found = h;
  return found?.players ?? null;
}
