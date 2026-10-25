<script lang="ts">
  /**
   * One server: the facts, Join, the favourite star, a link to it on zatlas.app, what its owner wrote about it (with
   * links that open in the browser), its player count over the last day or
   * week (from zatlas.app's servers API, as on the site's server panel), and its mods with what this PC has:
   * downloaded, out of date, or still to download (and how big). The row's data shows at once while the rest loads.
   */
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api, message, type ModState, type ServerDetail, type ServerItem } from "../lib/api";
  import { favourites } from "../lib/favourites.svelte";
  import { chart, countAt } from "../lib/chart";
  import { bytes, DOUBT, FAKE_NOTE, mapName, pingTone, sameVersion } from "../lib/format";
  import { joining } from "../lib/joining.svelte";
  import { linkify } from "../lib/links";
  import { pings } from "../lib/ping.svelte";
  import { inviteLink } from "../lib/serverMenu";
  import { live } from "../lib/status.svelte";
  import Button from "./Button.svelte";

  let { id, summary, onclose }: { id: string; summary?: ServerItem; onclose: () => void } = $props();

  let detail = $state.raw<ServerDetail | null>(null);
  let states = $state.raw<Map<number, ModState> | null>(null);
  let error = $state("");
  let copied = $state(false);
  let invited = $state(false);
  let hours = $state(24);
  /** the pointer's place on the chart, 0 to 1 */
  let hover = $state<number | null>(null);
  const W = 360, H = 90;

  $effect(() => {
    let current = true;
    error = "";
    api.server(id, hours)
      .then((d) => {
        if (!current) return;
        detail = d;
        if (d.mods.length && !states) api.modStates(d.mods.map((m) => m.id)).then((m) => (states = new Map(m.map((x) => [x.id, x])))).catch(() => {});
      })
      .catch((e) => { if (current) error = message(e); });
    return () => { current = false; };
  });

  const now = $derived(detail ? Math.max(Date.now(), detail.history.at(-1)?.t ?? 0) : Date.now());
  const from = $derived(now - hours * 3_600_000);
  const line = $derived(detail?.server ? chart(detail.history, from, now, detail.server.maxPlayers, W, H) : null);
  const short = $derived(!!detail && detail.history.length < 3);
  const firstSeen = $derived(detail?.history.length ? Math.min(...detail.history.map((h) => h.t)) : null);
  // the highest and lowest count in the window, the count in force at its start included
  const counts = $derived.by(() => {
    if (!detail) return [];
    const start = countAt(detail.history, from);
    return [...(start === null ? [] : [start]), ...detail.history.filter((h) => h.t >= from).map((h) => h.players)];
  });
  const hoverAt = $derived(hover === null ? null : from + hover * (now - from));
  const hoverCount = $derived(hoverAt === null || !detail ? null : countAt(detail.history, hoverAt));
  const clock = (t: number) => new Date(t).toLocaleString(undefined, hours > 24 ? { weekday: "short", hour: "2-digit", minute: "2-digit" } : { hour: "2-digit", minute: "2-digit" });
  function onmove(e: PointerEvent) {
    const r = (e.currentTarget as SVGElement).getBoundingClientRect();
    hover = Math.min(1, Math.max(0, (e.clientX - r.left) / r.width));
  }

  const s = $derived(detail?.server ?? summary);
  const ping = $derived(pings.get(id));
  $effect(() => { if (s) pings.measure([s]); });
  const fav = $derived(favourites.has(id));
  const local = $derived(live.now?.dayzVersion ?? null);
  const todo = $derived(detail && states ? detail.mods.filter((m) => { const st = states!.get(m.id); return st?.onWorkshop !== false && (!st?.installed || st.outdated); }) : []);
  const todoSize = $derived(todo.reduce((n, m) => n + (states?.get(m.id)?.size ?? 0), 0));
  const address = $derived(s ? `${s.ip}:${s.gamePort}` : id);

  async function copyInvite() {
    try { await navigator.clipboard.writeText(inviteLink(id)); invited = true; setTimeout(() => (invited = false), 2000); } catch { /* no clipboard */ }
  }

  async function copy() {
    try { await navigator.clipboard.writeText(address); copied = true; setTimeout(() => (copied = false), 1500); } catch { /* no clipboard */ }
  }
</script>

<aside class="panel">
  <header>
    <h2>{s?.name ?? id}</h2>
    <button type="button" class="x" aria-label="Close" onclick={onclose}>✕</button>
  </header>

  {#if error}
    <p class="danger">{error}</p>
  {:else if s}
    <p class="count"><b>{s.players}</b> / {s.maxPlayers} players{#if s.queue > 0}<span class="warn">{` · queue ${s.queue}`}</span>{/if}</p>
    {#if s.farm}<p class="fake">{FAKE_NOTE}</p>{/if}
    {#if s.doubt.length}
      <div class="doubts">
        <p><b>The player count may be made up:</b></p>
        <ul>{#each s.doubt as d (d)}<li>{DOUBT[d] ?? d}</li>{/each}</ul>
        <p class="note">Counts are what the server reports about itself. These signals are first hints, not proof, and the server stays listed.</p>
      </div>
    {/if}
    <div class="buttons">
      <Button primary onclick={() => joining.open(id, s.name)}>Join</Button>
      <Button aria-pressed={fav} onclick={() => favourites.toggle({ id, name: s.name })}>{fav ? "★ Favourite" : "☆ Favourite"}</Button>
      <Button onclick={copyInvite} title="A link for Discord or a friend: it opens this server in Zatlas">{invited ? "Invite link copied" : "Copy invite link"}</Button>
      <Button onclick={() => openUrl(`https://zatlas.app/servers?s=${encodeURIComponent(id)}`)}>On zatlas.app ↗</Button>
    </div>

    <dl>
      <dt>Address</dt><dd><code>{address}</code> <button type="button" class="link" onclick={copy}>{copied ? "Copied" : "Copy"}</button></dd>
      <dt>Ping</dt>
      <dd>
        {#if ping === undefined}<span class="note">Measuring…</span>
        {:else}
          {#key pings.refreshed(id)}
            <button type="button" class="ping {ping === null ? 'none' : pingTone(ping)}" class:busy={pings.busy(id)} class:flash={pings.refreshed(id) > 0}
              title="Press to ping again" onclick={() => pings.measure([s], true)}>{ping === null ? "No answer" : `${ping} ms`}</button>
          {/key}
          <span class="note">from this PC</span>
        {/if}
      </dd>
      <dt>Map</dt><dd>{mapName(s.map)}</dd>
      <dt>Version</dt>
      <dd>{s.version}{#if !sameVersion(s.version, local)}<span class="warn">{` · this PC has ${local}`}</span>{/if}</dd>
      <dt>View</dt><dd>{s.firstPerson ? "First person only" : "First and third person"}</dd>
      {#if s.gameTime}<dt>Time</dt><dd>{s.gameTime}{s.dayAccel ? ` · days ×${s.dayAccel}, nights ×${s.nightAccel}` : ""}</dd>{/if}
      <dt>BattlEye</dt><dd>{s.battleye ? "On" : "Off"}</dd>
      {#if s.password != null}<dt>Password</dt><dd>{s.password ? "Needed to join" : "None"}</dd>{/if}
    </dl>

    {#if detail?.server.description}
      <h3 class="plate">About</h3>
      <p class="about">{#each linkify(detail.server.description) as part, i (i)}{#if part.url}<button type="button" class="link" title={part.url} onclick={() => api.openLink(part.url!)}>{part.text}</button>{:else}{part.text}{/if}{/each}</p>
    {/if}

    <h3 class="plate">Players over time (claimed)</h3>
    <div class="range" role="group" aria-label="Players over time">
      <button type="button" aria-pressed={hours === 24} onclick={() => (hours = 24)}>24 hours</button>
      <button type="button" aria-pressed={hours === 168} onclick={() => (hours = 168)}>7 days</button>
      {#if counts.length}<span class="note">{`Peak ${Math.max(...counts)} · low ${Math.min(...counts)}`}</span>{/if}
    </div>
    {#if line?.line}
      <svg class="chart" viewBox="0 0 {W} {H + 16}" role="img" aria-label={`Players claimed by ${s.name} over the last ${hours} hours`}
        onpointermove={onmove} onpointerleave={() => (hover = null)}>
        <line x1="0" y1="0" x2={W} y2="0" class="grid" />
        <line x1="0" y1={H} x2={W} y2={H} class="grid" />
        <path d={line.area} class="area" />
        <path d={line.line} class="line" />
        {#if hover !== null}<line x1={hover * W} y1="0" x2={hover * W} y2={H} class="cursor" />{/if}
        <text x="0" y={H + 13} class="axis">{`${hours} h ago`}</text>
        <text x={W} y={H + 13} class="axis" text-anchor="end">now</text>
        <text x={W} y="11" class="axis" text-anchor="end">{`${s.maxPlayers} slots`}</text>
      </svg>
      <p class="note readout">{hoverAt !== null && hoverCount !== null ? `${hoverCount} players at ${clock(hoverAt)}` : " "}</p>
    {:else if !detail}
      <p class="note">Loading…</p>
    {/if}
    {#if short}<p class="note">Too little history yet: the count is followed from when Zatlas first saw the server.</p>
    {:else if firstSeen && firstSeen > from + 3_600_000}<p class="note">{`Followed since ${new Date(firstSeen).toLocaleDateString(undefined, { day: "numeric", month: "short" })}, when Zatlas first saw the server.`}</p>{/if}

    {#if s.modded}
      <h3 class="plate">{detail ? `Mods (${detail.mods.length})` : "Mods"}</h3>
      {#if !detail}
        <p class="note">Loading the mod list…</p>
      {:else if !detail.mods.length}
        <p class="note">The server doesn't list its mods.</p>
      {:else}
        <p class="note">
          {#if !states}Checking which you have…
          {:else if !todo.length}You have all of them, up to date.
          {:else}{`${todo.length} to download${todoSize ? ` (${bytes(todoSize)})` : ""}; Join gets them through Steam.`}{/if}
        </p>
        <ul class="mods">
          {#each detail.mods as m (m.id)}
            {@const st = states?.get(m.id)}
            <li>
              <button type="button" class="link" onclick={() => openUrl(`https://steamcommunity.com/sharedfiles/filedetails/?id=${m.id}`)}>{m.name}</button>
              {#if st && !st.onWorkshop}
                <span class="tag" title="Gone or private on the Workshop, often a server-side mod. A join skips it.">Not on Workshop</span>
              {:else if st}
                <span class="tag" class:have={st.installed && !st.outdated} class:old={st.outdated}>
                  {st.installed ? (st.outdated ? "Update" : "Have") : st.size ? bytes(st.size) : "Get"}
                </span>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
    {/if}
  {/if}
</aside>

<style>
  .panel { background: var(--panel); box-shadow: inset 1px 0 0 var(--line); padding: var(--s-5); overflow: auto; display: flex; flex-direction: column; gap: var(--s-4); min-height: 0; }
  /* a long mod list scrolls the panel; nothing in it shrinks (the chart would go to 0 px) */
  .panel > * { flex-shrink: 0; }
  header { display: flex; gap: var(--s-3); align-items: start; justify-content: space-between; }
  h2 { margin: 0; font: 800 var(--t-section)/1.25 var(--f-text); overflow-wrap: anywhere; }
  .x { flex: none; border: 0; background: var(--inset); box-shadow: inset 0 0 0 1px var(--line); padding: var(--s-3) var(--s-4); cursor: pointer; font-weight: 700; }
  .x:hover { background: var(--plate); }
  .count { margin: 0; font-size: var(--t-lead); font-variant-numeric: tabular-nums; }
  .fake { margin: 0; color: var(--danger); font-size: var(--t-body-small); }
  .doubts { color: var(--warn); font-size: var(--t-body-small); display: grid; gap: var(--s-2); }
  .doubts p, .doubts ul { margin: 0; }
  .doubts ul { padding-left: 18px; }
  .buttons { display: flex; flex-wrap: wrap; gap: var(--s-3); }
  dl { display: grid; grid-template-columns: auto 1fr; gap: var(--s-2) var(--s-5); margin: 0; font-size: var(--t-body-small); }
  dt { color: var(--muted); }
  dd { margin: 0; overflow-wrap: anywhere; }
  code { font: var(--t-code) var(--f-mono); }
  .link { border: 0; background: none; padding: 0; color: var(--amber); cursor: pointer; text-align: left; font-size: inherit; }
  .link:hover { text-decoration: underline; }
  h3.plate { font-size: var(--t-label); margin-top: var(--s-3); }
  .about { margin: 0; white-space: pre-line; overflow-wrap: anywhere; font-size: var(--t-body-small); }
  .about .link { display: inline; }
  .range { display: flex; gap: var(--s-2); align-items: center; }
  .range button { border: 0; background: var(--inset); color: var(--muted); box-shadow: inset 0 0 0 1px var(--line); padding: var(--s-2) var(--s-4); cursor: pointer; font: 700 var(--t-button-small)/1.4 var(--f-text); }
  .range button[aria-pressed="true"] { background: var(--amber); color: var(--amber-ink); box-shadow: none; }
  .range button:focus-visible { outline: 2px solid var(--amber); outline-offset: 2px; }
  .range .note { margin-left: auto; font-variant-numeric: tabular-nums; }
  .chart { width: 100%; height: auto; display: block; background: var(--inset); cursor: crosshair; }
  .grid { stroke: var(--line); stroke-width: 1; }
  .area { fill: var(--amber); opacity: .18; }
  .line { fill: none; stroke: var(--amber); stroke-width: 1.5; vector-effect: non-scaling-stroke; }
  .cursor { stroke: var(--ink); stroke-width: 1; opacity: .5; vector-effect: non-scaling-stroke; }
  .axis { fill: var(--muted); font-size: 10px; }
  .readout { margin: calc(-1 * var(--s-3)) 0 0; min-height: 1.4em; font-variant-numeric: tabular-nums; }
  .mods { list-style: none; margin: 0; padding: 0; font-size: var(--t-body-small); }
  .mods li { display: flex; justify-content: space-between; gap: var(--s-3); padding: var(--s-2) 0; border-bottom: 1px solid var(--line); }
  .mods .link { color: var(--ink); overflow-wrap: anywhere; }
  .tag { flex: none; padding: 0 var(--s-3); border-radius: var(--r-small); background: var(--inset); color: var(--muted); font-size: var(--t-label); font-weight: 700; }
  .tag.have { color: var(--ok); }
  .tag.old { color: var(--warn); }
</style>
