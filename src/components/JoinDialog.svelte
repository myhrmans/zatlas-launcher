<script lang="ts">
  /**
   * Joining a server, in a dialog: first what it will do (mods to download and their size, a different game version,
   * DayZ already running, an optional password), then each mod's progress, then "DayZ is starting". A full server can
   * be waited for: once the mods are in, the join waits for a free slot and then starts DayZ. The dialog can be hidden
   * meanwhile; the top bar shows the join until it's done.
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
  /** on a full server: wait for a free slot, then start DayZ */
  let wait = $state(true);
  // a clock for the time spent waiting
  let now = $state(Date.now());
  $effect(() => {
    const timer = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(timer);
  });

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
  const full = $derived(!!detail && detail.server.players >= detail.server.maxPlayers);
  const waitForRoom = $derived(full && wait);
  const waited = $derived.by(() => {
    const s = joining.waitingSince ? Math.max(0, Math.floor((now - joining.waitingSince) / 1000)) : 0;
    return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
  });

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
        {#if full}
          <label class="wait">
            <input type="checkbox" bind:checked={wait} />
            <span>The server is full: wait for a free slot, then start DayZ</span>
          </label>
        {/if}
        {#if s.password !== false}
          <label class="pw"><span>Password <span class="note">{s.password ? "(this server needs one)" : "(only if the server has one)"}</span></span>
            <input class="field" type="password" bind:value={password} autocomplete="off" />
          </label>
        {/if}
        {#if detail.mods.length}
          <p class="note">While mods download, Zatlas is connected to Steam as DayZ, so Steam shows you as playing DayZ.</p>
        {/if}
        {#if otherBusy}<p class="warn">Another join is still running.</p>{/if}
        <div class="buttons">
          <Button onclick={() => joining.close()}>Cancel</Button>
          <Button primary disabled={!s.queryPort || otherBusy} onclick={() => joining.start(target.id, s.name, password, running, waitForRoom)}>
            {todo.length ? `Get ${todo.length} mod${todo.length === 1 ? "" : "s"} and ${waitForRoom ? "wait for a slot" : "join"}` : waitForRoom ? "Wait for a slot" : "Join"}
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
    {:else if p.phase === "waiting"}
      <p>{`Waiting for a free slot: ${p.slots ? `${p.slots.players}/${p.slots.max} players` : "the server didn't answer, asking again"}`}</p>
      <p class="note">{`Waiting ${waited}. Zatlas asks the server every 5 seconds and starts DayZ as soon as there's room; the app flashes in the taskbar when it does. DayZ takes about a minute to get in, so on a busy server the slot can be taken again.`}</p>
      <div class="buttons">
        <Button onclick={() => api.cancelJoin()}>Stop waiting</Button>
        <Button onclick={() => joining.close()}>Hide</Button>
      </div>
    {:else if p.phase === "starting"}
      <p>Starting DayZ…</p>
    {:else if p.phase === "started"}
      <LaunchView launch={joining.launch?.id === target.id ? joining.launch : null} onagain={() => joining.start(target.id, p.name, password, true)} />
      {#if skipped.length}<p class="note">{`Skipped, not on the Workshop: ${skipped.map((m) => m.name).join(", ")}. If the server needs one of them, it says so when you join.`}</p>{/if}
    {:else if p.phase === "cancelled"}
      <p>{p.mods.some((m) => m.state !== "ready" && m.state !== "skipped") ? "Cancelled. Steam keeps downloading the mods it had started." : "Cancelled."}</p>
      <div class="buttons"><Button onclick={() => joining.close()}>Close</Button></div>
    {:else}
      <p class="danger">{p.message}</p>
      <div class="buttons">
        <Button onclick={() => joining.close()}>Close</Button>
        <Button primary onclick={() => joining.start(target.id, p.name, password, running, waitForRoom)}>Try again</Button>
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
  .wait { display: flex; gap: var(--s-3); align-items: center; font-weight: 700; font-size: var(--t-body-small); cursor: pointer; }
  .wait input { accent-color: var(--amber); width: 16px; height: 16px; cursor: pointer; }
  .buttons { display: flex; justify-content: flex-end; gap: var(--s-3); margin-top: var(--s-3); }
  .progress { list-style: none; margin: 0; padding: 0; display: grid; gap: var(--s-3); max-height: 320px; overflow: auto; }
  .progress li { display: grid; grid-template-columns: 1fr auto; gap: 2px var(--s-4); font-size: var(--t-body-small); }
  .pname { font-weight: 700; overflow-wrap: anywhere; }
  .progress .note { font-variant-numeric: tabular-nums; }
  .bar { grid-column: 1 / -1; height: 4px; background: var(--line); }
  .bar i { display: block; height: 100%; background: var(--amber); transition: width .3s; }
</style>
