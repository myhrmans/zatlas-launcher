<script lang="ts">
  /** Steam and DayZ in a word each, top right: running or not, signed in, the game's version, how long it has run. */
  import { since } from "../lib/format";
  import { live } from "../lib/status.svelte";

  const s = $derived(live.now);
  const steam = $derived.by(() => {
    if (!s) return { tone: "idle", text: "Steam…" };
    if (!s.steamRunning) return { tone: "bad", text: "Steam isn't running" };
    if (!s.steamSignedIn) return { tone: "warn", text: "Steam: not signed in" };
    return { tone: "ok", text: s.steamConnected ? "Steam · connected as DayZ" : "Steam" };
  });
  const dayz = $derived.by(() => {
    if (!s) return { tone: "idle", text: "DayZ…" };
    if (!s.dayzDir) return { tone: "bad", text: "DayZ isn't installed" };
    if (s.dayzStarted) return { tone: "ok", text: `DayZ running · ${since(s.dayzStarted)}` };
    return { tone: "idle", text: `DayZ ${s.dayzVersion ?? ""}`.trim() };
  });
</script>

<div class="chips">
  <span class="chip {steam.tone}" title={s?.steamConnected ? "While Zatlas is connected, Steam shows you as playing DayZ" : undefined}><i></i>{steam.text}</span>
  <span class="chip {dayz.tone}" title={s?.dayzDir ?? undefined}><i></i>{dayz.text}</span>
</div>

<style>
  .chips { margin-left: auto; display: flex; gap: var(--s-3); }
  .chip { display: inline-flex; align-items: center; gap: var(--s-3); padding: var(--s-2) var(--s-4); border-radius: var(--r-small); background: var(--inset); box-shadow: inset 0 0 0 1px var(--line);
    font-size: var(--t-label); font-weight: 700; color: var(--muted); white-space: nowrap; font-variant-numeric: tabular-nums; }
  i { width: 8px; height: 8px; border-radius: 50%; background: var(--rim); }
  .ok { color: var(--ink); }
  .ok i { background: var(--ok); }
  .warn i { background: var(--warn); }
  .bad { color: var(--danger); }
  .bad i { background: var(--danger); }
</style>
