<script lang="ts">
  /**
   * Joining a server, in a dialog: first what it will do (mods to download and their size, a different game version,
   * DayZ already running, an optional password), then each mod's progress, then "DayZ is starting". The dialog can be
   * hidden while mods download; the top bar shows the join until it's done.
   */
  import { api, message, type ModState, type ServerDetail } from "../lib/api";
  import { bytes, sameVersion } from "../lib/format";
  import { joining } from "../lib/joining.svelte";
  import { live } from "../lib/status.svelte";
  import Button from "./Button.svelte";
  import LaunchView from "./LaunchView.svelte";

  const target = $derived(joining.target!);
  let detail = $state.raw<ServerDetail | null>(null);
  let states = $state.raw<ModState[] | null>(null);
  let failed = $state("");
  let password = $state("");

  $effect(() => {
    const id = target.id;
    api.server(id)
      .then((d) => {
        detail = d;
        return d.mods.length ? api.modStates(d.mods.map((m) => m.id)).then((m) => (states = m)) : (states = []);
      })
      .catch((e) => (failed = message(e)));
  });

  const p = $derived(joining.progress?.id === target.id ? joining.progress : null);
  const todo = $derived(states?.filter((m) => m.onWorkshop && (!m.installed || m.outdated)) ?? []);
  const absent = $derived(states?.filter((m) => !m.onWorkshop).length ?? 0);
  const todoSize = $derived(todo.reduce((n, m) => n + m.size, 0));
  const local = $derived(live.now?.dayzVersion ?? null);
  const running = $derived(!!live.now?.dayzStarted);
  const ready = $derived(p ? p.mods.filter((m) => m.state === "ready").length : 0);
  const wanted = $derived(p ? p.mods.filter((m) => m.state !== "skipped").length : 0);
  const skipped = $derived(p ? p.mods.filter((m) => m.state === "skipped") : []);
  const otherBusy = $derived(joining.busy && !p);

  function onkey(e: KeyboardEvent) { if (e.key === "Escape") joining.close(); }
</script>

<svelte:window onkeydown={onkey} />

<div class="scrim" role="presentation" onclick={(e) => { if (e.target === e.currentTarget) joining.close(); }}>
  <div class="dialog" role="dialog" aria-modal="true" aria-labelledby="jd-title">
    <h2 id="jd-title"><span class="plate">Join</span><span>{p?.name ?? detail?.server.name ?? target.name}</span></h2>

    {#if !p}
      {#if failed}
        <p class="danger">{failed}</p>
      {:else if !detail || !states}
        <p class="note">Checking the server and your mods…</p>
      {:else}
        {@const s = detail.server}
        <ul class="facts">
          <li>{`${s.players}/${s.maxPlayers} players${s.queue > 0 ? `, queue ${s.queue}` : ""}`}</li>
          {#if detail.mods.length}
            <li>{detail.mods.length} mods: {todo.length ? `${todo.length} to download or update${todoSize ? ` (${bytes(todoSize)})` : ""}` : "you have them all"}</li>
          {:else}
            <li>No mods</li>
          {/if}
          {#if absent}<li>{`${absent} not on the Workshop (gone, private or server-side): skipped`}</li>{/if}
          {#if !sameVersion(s.version, local)}<li class="warn">The server runs DayZ {s.version}; this PC has {local}. It may not let you in.</li>{/if}
          {#if running}<li class="warn">DayZ is running: Zatlas closes it before joining.</li>{/if}
          {#if !s.queryPort}<li class="danger">zatlas.app doesn't know this server's query port yet, so it can't be joined from here.</li>{/if}
        </ul>
        <label class="pw"><span>Password <span class="note">(only if the server has one)</span></span>
          <input class="field" type="password" bind:value={password} autocomplete="off" />
        </label>
        {#if detail.mods.length}
          <p class="note">While mods download, Zatlas is connected to Steam as DayZ, so Steam shows you as playing DayZ.</p>
        {/if}
        {#if otherBusy}<p class="warn">Another join is still running.</p>{/if}
        <div class="buttons">
          <Button onclick={() => joining.close()}>Cancel</Button>
          <Button primary disabled={!s.queryPort || otherBusy} onclick={() => joining.start(target.id, s.name, password, running)}>
            {todo.length ? `Get ${todo.length} mod${todo.length === 1 ? "" : "s"} and join` : "Join"}
          </Button>
        </div>
      {/if}
    {:else if p.phase === "mods"}
      <p>{p.mods.length ? `Mods: ${ready} of ${wanted} ready` : "Getting the server's details…"}</p>
      <ul class="progress">
        {#each p.mods.filter((m) => m.state !== "ready" && m.state !== "skipped") as m (m.id)}
          <li>
            <span class="pname">{m.name}</span>
            <span class="note">{m.state === "downloading" && m.total ? `${bytes(m.done)} / ${bytes(m.total)}` : m.state === "subscribing" ? "subscribing" : "waiting for Steam"}</span>
            <span class="bar"><i style:width="{m.total ? (m.done / m.total) * 100 : 0}%"></i></span>
          </li>
        {/each}
      </ul>
      <div class="buttons">
        <Button onclick={() => api.cancelJoin()}>Cancel join</Button>
        <Button onclick={() => joining.close()}>Hide</Button>
      </div>
    {:else if p.phase === "starting"}
      <p>Starting DayZ…</p>
    {:else if p.phase === "started"}
      <LaunchView launch={joining.launch?.id === target.id ? joining.launch : null} onagain={() => joining.start(target.id, p.name, password, true)} />
      {#if skipped.length}<p class="note">{`Skipped, not on the Workshop: ${skipped.map((m) => m.name).join(", ")}. If the server needs one of them, it says so when you join.`}</p>{/if}
    {:else if p.phase === "cancelled"}
      <p>Cancelled. Steam keeps downloading the mods it had started.</p>
      <div class="buttons"><Button onclick={() => joining.close()}>Close</Button></div>
    {:else}
      <p class="danger">{p.message}</p>
      <div class="buttons">
        <Button onclick={() => joining.close()}>Close</Button>
        <Button primary onclick={() => joining.start(target.id, p.name, password, running)}>Try again</Button>
      </div>
    {/if}
  </div>
</div>

<style>
  .scrim { position: fixed; inset: 0; background: var(--dim); display: grid; place-items: center; z-index: 10; }
  .dialog { width: min(560px, calc(100vw - 32px)); max-height: calc(100vh - 64px); overflow: auto; background: var(--panel); box-shadow: var(--shadow); border-radius: var(--r-surface);
    padding: var(--s-6); display: flex; flex-direction: column; gap: var(--s-4); position: relative; }
  .dialog::before { content: ""; position: absolute; inset: 0 0 auto; height: var(--hazard-h); background: repeating-linear-gradient(-45deg, var(--amber) 0 8px, var(--dark) 8px 16px); }
  h2 { margin: var(--s-2) 0 0; font: 800 var(--t-section)/1.3 var(--f-text); display: flex; gap: var(--s-4); align-items: baseline; overflow-wrap: anywhere; }
  h2 .plate { font-size: var(--t-label); flex: none; }
  p { margin: 0; }
  .facts { margin: 0; padding-left: 18px; display: grid; gap: var(--s-2); }
  .pw { display: grid; gap: var(--s-2); font-weight: 700; font-size: var(--t-body-small); }
  .buttons { display: flex; justify-content: flex-end; gap: var(--s-3); margin-top: var(--s-3); }
  .progress { list-style: none; margin: 0; padding: 0; display: grid; gap: var(--s-3); max-height: 320px; overflow: auto; }
  .progress li { display: grid; grid-template-columns: 1fr auto; gap: 2px var(--s-4); font-size: var(--t-body-small); }
  .pname { font-weight: 700; overflow-wrap: anywhere; }
  .progress .note { font-variant-numeric: tabular-nums; }
  .bar { grid-column: 1 / -1; height: 4px; background: var(--line); }
  .bar i { display: block; height: 100%; background: var(--amber); transition: width .3s; }
</style>
