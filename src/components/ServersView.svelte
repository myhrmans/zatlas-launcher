<script lang="ts">
  /**
   * The server list from zatlas.app (the same as zatlas.app/servers): search and filters, favourites, a Join button on
   * the row under the pointer, and the chosen server's panel on the right. While DayZ runs, a line on top names the
   * server last joined from here. The ping filter works on this PC's pings, so it hides servers among those loaded; the
   * other filters go to the servers API.
   */
  import { onMount } from "svelte";
  import { api, message, type Facets, type ServerItem } from "../lib/api";
  import { favourites } from "../lib/favourites.svelte";
  import { DOUBT_TITLE, FAKE_NOTE, mapName, pingTone, sameVersion, since } from "../lib/format";
  import { joining } from "../lib/joining.svelte";
  import { pings } from "../lib/ping.svelte";
  import { live } from "../lib/status.svelte";
  import Button from "./Button.svelte";
  import ServerPanel from "./ServerPanel.svelte";
  import Star from "./Star.svelte";

  const PAGE = 60;
  let q = $state("");
  let map = $state("");
  let person = $state<"" | "first" | "third">("");
  let modded = $state<"" | "0" | "1">("");
  let minPlayers = $state(0);
  let favView = $state(false);
  /** also the listings zatlas.app hides as fake: empty copies and fake listings from server farms */
  let showFake = $state(false);
  // the password and ping filters are kept between starts, as preferences
  const NO_PASSWORD = "zatlas.filter.noPassword", MAX_PING = "zatlas.filter.maxPing";
  let noPassword = $state(readSetting(NO_PASSWORD) === "1");
  /** hide servers with a higher ping, and those that don't answer one; 0: any ping */
  let maxPing = $state(Number(readSetting(MAX_PING)) || 0);
  $effect(() => saveSetting(NO_PASSWORD, noPassword ? "1" : ""));
  $effect(() => saveSetting(MAX_PING, maxPing ? String(maxPing) : ""));

  function readSetting(key: string) {
    try { return localStorage.getItem(key); } catch { return null; }
  }
  function saveSetting(key: string, value: string) {
    try { if (value) localStorage.setItem(key, value); else localStorage.removeItem(key); } catch { /* kept for this session only */ }
  }

  let items = $state.raw<ServerItem[]>([]);
  let total = $state(0);
  let status = $state<"loading" | "ok" | "error">("loading");
  let error = $state("");
  let facets = $state.raw<Facets | null>(null);
  let selected = $state("");
  let more = $state(false);
  /** when a search finds nothing with the filters on: how many it finds without them */
  let unfiltered = $state(0);

  const filtered = () => !!(map || person || modded || minPlayers);
  function clearFilters() { map = ""; person = ""; modded = ""; minPlayers = 0; }

  const query = (offset: number) => {
    const p = new URLSearchParams();
    if (q.trim()) p.set("q", q.trim());
    if (map) p.set("map", map);
    if (person) p.set("person", person);
    if (modded) p.set("modded", modded);
    if (minPlayers) p.set("minPlayers", String(minPlayers));
    if (favView) p.set("ids", favourites.list.map((f) => f.id).join(","));
    if (showFake) p.set("farm", "1");
    if (noPassword) p.set("password", "0");
    p.set("limit", String(favView ? 200 : PAGE));
    if (offset) p.set("offset", String(offset));
    return p.toString();
  };

  // Each load gets a number, and an answer that comes back after a newer load has started is dropped.
  let loads = 0;
  async function load(offset = 0) {
    const thisLoad = ++loads;
    if (favView && !favourites.list.length) { items = []; total = 0; status = "ok"; return; }
    if (offset) more = true; else status = "loading";
    try {
      const r = await api.servers(query(offset));
      if (thisLoad !== loads) return;
      items = offset ? [...items, ...r.servers] : r.servers;
      total = r.total;
      unfiltered = 0;
      status = "ok";
      if (!offset && !r.total && !favView && q.trim() && filtered()) {
        const all = await api.servers(new URLSearchParams({ q: q.trim(), limit: "1" }).toString());
        if (thisLoad === loads) unfiltered = all.total;
      }
    } catch (e) {
      if (thisLoad !== loads) return;
      error = message(e);
      status = "error";
    } finally {
      if (thisLoad === loads) more = false;
    }
  }

  // filters reload at once; typing waits for a pause
  let typingTimer = 0;
  $effect(() => { void [map, person, modded, minPlayers, favView, showFake, noPassword]; load(); });
  function onType() { clearTimeout(typingTimer); typingTimer = window.setTimeout(() => load(), 300); }

  onMount(() => { api.facets().then((f) => (facets = f)).catch(() => {}); });
  // the servers in the list get their ping measured as they show up
  $effect(() => { pings.measure(items); });

  const maps = $derived((facets?.maps ?? []).slice(0, 40));
  // favourites the list doesn't have (offline, or gone), when nothing else narrows the list
  const missingFavs = $derived(favView && status === "ok" && !q.trim() && !filtered()
    ? favourites.list.filter((f) => !items.some((s) => s.id === f.id)) : []);
  const local = $derived(live.now?.dayzVersion ?? null);
  // the "In DayZ on …" line: DayZ is running and was started from here (a join came at most 10 minutes before it started)
  const playing = $derived(live.now?.dayzStarted && live.last && live.last.at >= live.now.dayzStarted - 600 ? live.last : null);
  const selectedItem = $derived(items.find((s) => s.id === selected));
  // with a ping limit: the servers over it, or not answering, are hidden; those not measured yet stay until they are
  const overPing = (s: ServerItem) => {
    const ms = pings.get(s.id);
    return maxPing > 0 && ms !== undefined && (ms === null || ms > maxPing);
  };
  const shown = $derived(items.filter((s) => !overPing(s)));
  const hiddenByPing = $derived(items.length - shown.length);
</script>

<div class="view" class:open={!!selected}>
  <section class="list">
    {#if playing}
      <p class="playing">
        <span>In DayZ on <button type="button" onclick={() => (selected = playing.id)}>{playing.name}</button></span>
        {#if live.now?.dayzStarted}<span class="note">since {since(live.now.dayzStarted)}</span>{/if}
      </p>
    {/if}
    <div class="filters">
      <input class="field search" type="search" placeholder="Search servers, e.g. a name or an IP" bind:value={q} oninput={onType} aria-label="Search servers" />
      <select class="field" bind:value={map} aria-label="Map">
        <option value="">All maps</option>
        {#each maps as m (m.map)}<option value={m.map}>{mapName(m.map)} ({m.servers})</option>{/each}
      </select>
      <div class="seg" role="group" aria-label="Mods">
        <button type="button" aria-pressed={modded === ""} onclick={() => (modded = "")}>All</button>
        <button type="button" aria-pressed={modded === "0"} onclick={() => (modded = "0")}>Vanilla</button>
        <button type="button" aria-pressed={modded === "1"} onclick={() => (modded = "1")}>Modded</button>
      </div>
      <div class="seg" role="group" aria-label="View">
        <button type="button" aria-pressed={person === ""} onclick={() => (person = "")}>1PP + 3PP</button>
        <button type="button" aria-pressed={person === "first"} onclick={() => (person = "first")}>1PP</button>
        <button type="button" aria-pressed={person === "third"} onclick={() => (person = "third")}>3PP</button>
      </div>
      <select class="field" bind:value={minPlayers} aria-label="Players">
        <option value={0}>Any players</option>
        <option value={1}>1+ players</option>
        <option value={10}>10+ players</option>
        <option value={30}>30+ players</option>
        <option value={60}>60+ players</option>
      </select>
      <select class="field" bind:value={maxPing} aria-label="Ping">
        <option value={0}>Any ping</option>
        <option value={50}>Ping up to 50 ms</option>
        <option value={80}>Ping up to 80 ms</option>
        <option value={100}>Ping up to 100 ms</option>
        <option value={150}>Ping up to 150 ms</option>
      </select>
      <div class="seg" role="group" aria-label="Show">
        <button type="button" aria-pressed={!favView} onclick={() => (favView = false)}>All servers</button>
        <button type="button" aria-pressed={favView} onclick={() => (favView = true)}>Favourites ({favourites.list.length})</button>
      </div>
      {#if facets}
        <div class="totals">
          <span class="note">{`${facets.withPlayers.toLocaleString()} servers with players · ${facets.empty.toLocaleString()} empty · ${facets.hidden.toLocaleString()} hidden as fake · ${facets.players.toLocaleString()} players`}</span>
          {#if facets.hidden > 0 && !favView}
            <label class="toggle">
              <input type="checkbox" bind:checked={showFake} />
              <span>{`Show the ${facets.hidden.toLocaleString()} hidden listings (empty copies and fake listings)`}</span>
            </label>
          {/if}
          <label class="toggle">
            <input type="checkbox" bind:checked={noPassword} />
            <span>Hide servers with a password</span>
          </label>
        </div>
      {/if}
    </div>

    <div class="scroll">
      {#if status === "error"}
        <p class="empty danger">{error}</p>
      {:else if status === "loading" && !items.length}
        <p class="empty note">Loading servers…</p>
      {:else if !items.length && !missingFavs.length}
        <p class="empty note">{favView ? "No favourites yet: press the star on a server to keep it here." : filtered() ? "No server matches with these filters." : "No server matches."}</p>
        {#if unfiltered}
          <p class="hint">
            <span>{`${unfiltered.toLocaleString()} server${unfiltered === 1 ? "" : "s"} match “${q.trim()}” without the filters.`}</span>
            <Button small onclick={clearFilters}>Clear filters</Button>
          </p>
        {/if}
      {:else}
        <p class="count note">
          {favView ? `${items.length} of ${favourites.list.length} favourites online` : `${total.toLocaleString()} server${total === 1 ? "" : "s"}`}
          {#if hiddenByPing}{` · ${hiddenByPing} of those loaded hidden: ping over ${maxPing} ms or no answer`}{/if}
        </p>
        <table>
          <thead>
            <tr><th class="c-name">Server</th><th>Map</th><th class="c-num" title="Measured from this PC">Ping</th><th class="c-num">Players</th><th><span class="sr-only">Join</span></th></tr>
          </thead>
          <tbody>
            {#each shown as s (s.id)}
              {@const ms = pings.get(s.id)}
              <tr class:sel={s.id === selected} onclick={() => (selected = s.id)}>
                <td class="c-name">
                  <div class="name-row">
                    <Star id={s.id} name={s.name} />
                    <button type="button" class="name" onclick={(e) => { e.stopPropagation(); selected = s.id; }}>{s.name}</button>
                  </div>
                  <div class="tags">
                    <span>{s.firstPerson ? "1PP" : "3PP"}</span>
                    <span>{s.modded ? (s.modCount ? `${s.modCount} mods` : "Modded") : "Vanilla"}</span>
                    {#if s.password}<span title="Needs a password to join">Password</span>{/if}
                    {#if s.queue > 0}<span class="warn">Queue {s.queue}</span>{/if}
                    {#if s.farm}<span class="fake" title={FAKE_NOTE}>Likely fake</span>{/if}
                    {#if !sameVersion(s.version, local)}<span class="warn" title="This PC has DayZ {local}">Version {s.version}</span>{/if}
                  </div>
                </td>
                <td>{mapName(s.map)}</td>
                <td class="c-num">
                  {#if ms === undefined}<span class="note">{s.queryPort ? "…" : "–"}</span>
                  {:else}
                    {#key pings.refreshed(s.id)}
                      <button type="button" class="ping {ms === null ? 'none' : pingTone(ms)}" class:busy={pings.busy(s.id)} class:flash={pings.refreshed(s.id) > 0}
                        title={ms === null ? "No answer. Press to try again" : "Press to ping again"}
                        onclick={(e) => { e.stopPropagation(); pings.measure([s], true); }}>{ms === null ? "–" : `${ms} ms`}</button>
                    {/key}
                  {/if}
                </td>
                <td class="c-num">{#if s.doubt.length}<span class="doubt" role="img" aria-label="Count doubtful" title={DOUBT_TITLE}>?</span>{/if}<b>{s.players}</b><span class="note">/{s.maxPlayers}</span>
                  <span class="bar"><i style:width="{Math.min(100, (s.players / Math.max(1, s.maxPlayers)) * 100)}%"></i></span></td>
                <td class="c-join"><Button primary small onclick={(e: MouseEvent) => { e.stopPropagation(); joining.open(s.id, s.name); }}>Join</Button></td>
              </tr>
            {/each}
          </tbody>
        </table>
        {#if items.length && !shown.length}
          <p class="empty note">{`None of the ${items.length} servers loaded so far has a ping up to ${maxPing} ms.`}</p>
        {/if}
        {#if !favView && items.length < total}
          <p class="more"><Button loading={more} onclick={() => load(items.length)}>Show more</Button></p>
        {/if}
        {#if missingFavs.length}
          <h2 class="plate">Offline favourites</h2>
          <ul class="offline">
            {#each missingFavs as f (f.id)}<li><Star id={f.id} name={f.name} /><span>{f.name}</span><code class="note">{f.id}</code></li>{/each}
          </ul>
        {/if}
      {/if}
    </div>
  </section>

  {#if selected}
    {#key selected}
      <ServerPanel id={selected} summary={selectedItem} onclose={() => (selected = "")} />
    {/key}
  {/if}
</div>

<style>
  .view { height: 100%; display: grid; grid-template-columns: 1fr; min-height: 0; }
  .view.open { grid-template-columns: 1fr 420px; }
  .list { display: grid; grid-template-rows: auto auto 1fr; min-height: 0; min-width: 0; }
  .playing { margin: 0; display: flex; gap: var(--s-4); align-items: baseline; padding: var(--s-3) var(--s-5); background: var(--inset); box-shadow: inset 0 -1px 0 var(--line), inset 3px 0 0 var(--ok); }
  .playing button { border: 0; background: none; padding: 0; cursor: pointer; font-weight: 800; color: var(--ink); text-decoration: underline; text-decoration-color: var(--rim); }
  .filters { display: flex; flex-wrap: wrap; gap: var(--s-3); padding: var(--s-4) var(--s-5); box-shadow: inset 0 -1px 0 var(--line); }
  .search { flex: 1 1 240px; min-width: 200px; }
  .totals { flex-basis: 100%; display: flex; flex-wrap: wrap; gap: var(--s-2) var(--s-5); align-items: center; }
  .toggle { display: inline-flex; gap: var(--s-3); align-items: center; font-size: var(--t-body-small); cursor: pointer; }
  .toggle input { accent-color: var(--amber); width: 16px; height: 16px; cursor: pointer; }
  .fake { color: var(--danger); font-weight: 700; }
  .doubt { display: inline-grid; place-items: center; width: 1.25em; height: 1.25em; margin-right: var(--s-2); border-radius: 50%; box-shadow: inset 0 0 0 1.5px var(--warn);
    color: var(--warn); font: 800 11px/1 var(--f-text); cursor: help; vertical-align: middle; }
  .scroll { overflow: auto; min-height: 0; padding: 0 var(--s-5) var(--s-6); }
  .empty { padding: var(--s-6) 0 var(--s-3); }
  .hint { margin: 0; display: flex; gap: var(--s-4); align-items: center; }
  .count { padding: var(--s-4) 0 var(--s-2); }
  table { width: 100%; border-collapse: collapse; }
  th { text-align: left; font-size: var(--t-label); color: var(--muted); font-weight: 700; padding: var(--s-3); border-bottom: 1px solid var(--line); position: sticky; top: 0; background: var(--bg); z-index: 1; }
  td { padding: var(--s-3); border-bottom: 1px solid var(--line); vertical-align: middle; }
  tbody tr { cursor: pointer; }
  tbody tr:hover, tbody tr.sel { background: var(--inset); }
  tbody tr.sel { box-shadow: inset 3px 0 0 var(--amber); }
  .c-name { width: 100%; }
  .name-row { display: flex; align-items: center; gap: var(--s-3); }
  .name { border: 0; background: none; padding: 0; text-align: left; cursor: pointer; color: var(--ink); font-weight: 700; font-size: var(--t-body); overflow-wrap: anywhere; }
  .name:hover { color: var(--amber); }
  .tags { display: flex; gap: var(--s-3); margin: 2px 0 0 26px; font-size: var(--t-label); color: var(--muted); }
  td:not(.c-name) { white-space: nowrap; }
  .c-num { font-variant-numeric: tabular-nums; }
  .bar { display: block; width: 72px; height: 4px; margin-top: 4px; background: var(--line); }
  .bar i { display: block; height: 100%; background: var(--amber); }
  .c-join { width: 1%; }
  .c-join :global(.ui-btn) { opacity: 0; }
  tr:hover .c-join :global(.ui-btn), tr.sel .c-join :global(.ui-btn), tr:focus-within .c-join :global(.ui-btn) { opacity: 1; }
  .more { display: flex; justify-content: center; padding: var(--s-5) 0; margin: 0; }
  h2.plate { margin-top: var(--s-6); font-size: var(--t-label); }
  .offline { list-style: none; margin: 0; padding: 0; }
  .offline li { display: flex; gap: var(--s-3); align-items: center; padding: var(--s-3); border-bottom: 1px solid var(--line); color: var(--muted); }
  .offline code { margin-left: auto; }
</style>
