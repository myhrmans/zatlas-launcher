<script lang="ts">
  /**
   * DayZ starting, step by step with the seconds each took, from the launch monitor (src-tauri/src/launch.rs): BattlEye,
   * the game, its window, the scripts and mods, connecting and loading the map, in game. A warning while it goes on
   * (not responding, BattlEye waiting); when it fails, why, the crash log's reason, and what to do.
   */
  import { api, type Launch } from "../lib/api";
  import { mapName } from "../lib/format";
  import { joining } from "../lib/joining.svelte";
  import Button from "./Button.svelte";

  let { launch, onagain }: { launch: Launch | null; onagain: () => void } = $props();

  type Key = Launch["steps"][number]["key"];
  const LABEL: Record<Key, string> = {
    battleye: "BattlEye starts DayZ",
    game: "DayZ starts",
    window: "The game window opens",
    scripts: "The game and its mods load",
    connecting: "Connects to the server",
    ingame: "In game",
  };
  /** what's normal while a step is the current one */
  const NORMAL: Partial<Record<Key, string>> = {
    window: "Usually about 20 s, with no window while it reads its files.",
    scripts: "Longer with many mods.",
    connecting: "Then the map loads; a login queue makes this wait longer.",
  };

  const steps = $derived(launch?.steps ?? []);
  // the step being worked on: the first one not reached yet
  const current = $derived(launch?.state === "running" ? steps.find((s) => s.at === null)?.key : undefined);
  const label = (key: Key) => (key === "connecting" && launch?.map ? `Connected, loading ${mapName(launch.map)}` : LABEL[key]);
  const took = (i: number) => {
    const at = steps[i].at;
    if (at === null || i === 0) return "";
    return `${Math.max(0, at - (steps[i - 1].at ?? 0))} s`;
  };
  // seconds spent on the current step so far
  const waited = $derived.by(() => {
    if (!launch || !current) return 0;
    const reached = steps.filter((s) => s.at !== null).map((s) => s.at!);
    return launch.elapsed - (reached.length ? Math.max(...reached) : 0);
  });
</script>

{#if !launch}
  <p>Starting DayZ…</p>
{:else}
  <ol class="steps">
    {#each steps as s, i (s.key)}
      <li class:done={s.at !== null} class:now={s.key === current}>
        <span class="mark" aria-hidden="true">{s.at !== null ? "✓" : s.key === current ? "" : "·"}</span>
        <span class="what">
          {label(s.key)}
          {#if s.key === current}<span class="note">{`${waited} s`}</span>{/if}
          {#if s.key === current && NORMAL[s.key]}<span class="hint note">{NORMAL[s.key]}</span>{/if}
        </span>
        <span class="took note">{took(i)}</span>
      </li>
    {/each}
  </ol>

  {#if launch.warning}<p class="box warn">{launch.warning}</p>{/if}

  {#if launch.state === "ingame"}
    <p>{`In game on ${launch.name}, ${launch.elapsed} s after the start.`}</p>
    <div class="buttons"><Button primary onclick={() => joining.close()}>Done</Button></div>
  {:else if launch.state === "failed"}
    <div class="box bad">
      <p>{launch.problem}</p>
      {#if launch.detail}<p class="note">{`DayZ's crash log: ${launch.detail}`}</p>{/if}
    </div>
    <div class="buttons">
      <Button onclick={() => api.openLogs()}>DayZ's logs</Button>
      {#if launch.gameRunning}<Button onclick={() => api.closeGame()}>Close DayZ</Button>{/if}
      <Button primary onclick={onagain}>Join again</Button>
    </div>
  {:else}
    <div class="buttons">
      <Button onclick={() => api.closeGame()}>Close DayZ</Button>
      <Button onclick={() => joining.close()}>Hide</Button>
    </div>
  {/if}
{/if}

<style>
  p { margin: 0; }
  .steps { list-style: none; margin: 0; padding: 0; display: grid; gap: var(--s-1); }
  li { display: grid; grid-template-columns: 22px 1fr auto; gap: var(--s-3); align-items: baseline; padding: var(--s-2) 0; color: var(--muted); }
  li.done { color: var(--ink); }
  li:not(.done) .mark { color: var(--rim); }
  li.now { color: var(--ink); font-weight: 700; }
  .mark { text-align: center; color: var(--ok); font-weight: 800; }
  li.now .mark { width: 10px; height: 10px; margin: 0 auto; border-radius: 50%; border: 2px solid var(--amber); border-right-color: transparent; animation: spin 1s linear infinite; align-self: center; }
  .what { display: flex; flex-wrap: wrap; gap: 0 var(--s-3); align-items: baseline; }
  .what .note { font-weight: 400; font-variant-numeric: tabular-nums; }
  .hint { flex-basis: 100%; }
  .took { font-variant-numeric: tabular-nums; }
  .box { padding: var(--s-3) var(--s-4); border-radius: var(--r-small); background: var(--inset); display: grid; gap: var(--s-2); }
  .box.warn { box-shadow: inset 3px 0 0 var(--warn); }
  .box.bad { box-shadow: inset 3px 0 0 var(--danger); }
  .buttons { display: flex; justify-content: flex-end; gap: var(--s-3); margin-top: var(--s-3); }
  @keyframes spin { to { transform: rotate(360deg); } }
  @media (prefers-reduced-motion: reduce) { li.now .mark { animation: none; } }
</style>
