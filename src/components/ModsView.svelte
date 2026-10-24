<script lang="ts">
  /**
   * The DayZ Workshop mods on this PC: their size, the version's date and whether a newer one exists, which servers
   * joined from the app use each one, and when it was last used. Update one, the chosen ones or all of them; unsubscribe
   * from the chosen ones (Steam then deletes their files). The list needs no Steam connection; updating and
   * unsubscribing go through the Steam helper.
   */
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { onDestroy, onMount } from "svelte";
  import { api, ITEM, message, type ItemProgress, type WorkshopMod } from "../lib/api";
  import { bytes, day } from "../lib/format";
  import { history } from "../lib/history.svelte";
  import Button from "./Button.svelte";

  /** Mods unsubscribed from here: id → when (unix seconds). Steam's file only says so later, so until then these count
      as unsubscribed (for a day at most, in case they were subscribed again somewhere else). */
  const UNSUBSCRIBED = "zatlas.modsLeft";
  /** Mods the Workshop says have a newer version but Steam found up to date: id → the Workshop's date. That date also
      changes when the mod's page is edited. */
  const CHECKED = "zatlas.modsChecked";
  const DAY = 86400;

  /** the list as Steam's file has it */
  let fromSteam = $state.raw<WorkshopMod[] | null>(null);
  let unsubscribedAt = $state.raw<Record<number, number>>(readJson(UNSUBSCRIBED));
  let checked = $state.raw<Record<number, number>>(readJson(CHECKED));
  const now = () => Math.floor(Date.now() / 1000);
  /** the list, with the mods unsubscribed from here marked so */
  const mods = $derived(fromSteam?.map((m) => (m.subscribed && unsubscribedAt[m.id] > now() - DAY ? { ...m, subscribed: false } : m)) ?? null);
  let error = $state("");
  let q = $state("");
  type Sort = "updates" | "name" | "size" | "version" | "used";
  let sort = $state<Sort>("updates");
  /** 1: the column's own order (names A–Z; sizes and dates biggest or newest first), -1: the other way round */
  let dir = $state<1 | -1>(1);
  const COLUMNS: { key: Sort; label: string; cls: string }[] = [
    { key: "name", label: "Mod", cls: "" },
    { key: "size", label: "Size", cls: "c-num" },
    { key: "version", label: "Version", cls: "" },
    { key: "used", label: "Last used", cls: "" },
  ];
  let chosen = $state.raw<Set<number>>(new Set());
  let confirming = $state(false);
  let unsubscribing = $state(false);
  let undoing = $state(false);
  /** mods Steam is downloading: id → progress (null until Steam reports it) */
  let updating = $state.raw<Map<number, ItemProgress | null>>(new Map());
  let timer = 0;

  function readJson(key: string): Record<number, number> {
    try { return JSON.parse(localStorage.getItem(key) || "{}"); } catch { return {}; }
  }
  function saveJson(key: string, value: Record<number, number>) {
    try { localStorage.setItem(key, JSON.stringify(value)); } catch { /* kept for this session only */ }
  }
  function saveUnsubscribed(next: Record<number, number>) {
    unsubscribedAt = next;
    saveJson(UNSUBSCRIBED, next);
  }

  async function load() {
    try {
      fromSteam = await api.workshopMods();
      error = "";
      // forget the mods that Steam's file now lists as unsubscribed, or not at all
      const pending = Object.entries(unsubscribedAt).filter(([id]) => fromSteam!.find((m) => m.id === Number(id))?.subscribed);
      if (pending.length !== Object.keys(unsubscribedAt).length) saveUnsubscribed(Object.fromEntries(pending));
    } catch (e) { error = message(e); }
  }
  onMount(load);
  onDestroy(() => clearTimeout(timer));

  /** when a mod was last used: by a join from the app or in DayZ's logs, whichever is later */
  const lastUsed = (id: number) => Math.max(history.using(id)[0]?.at ?? 0, mods?.find((m) => m.id === id)?.lastUsed ?? 0);
  const needsUpdate = (m: WorkshopMod) => (m.outdated && checked[m.id] !== m.latest) || (m.subscribed && !m.installed);
  const name = (m: WorkshopMod) => m.title ?? `Workshop item ${m.id}`;

  const shown = $derived.by(() => {
    const words = q.trim().toLowerCase();
    const list = (mods ?? []).filter((m) => !words || name(m).toLowerCase().includes(words) || String(m.id).includes(words));
    const byName = (a: WorkshopMod, b: WorkshopMod) => name(a).localeCompare(name(b));
    const order = (a: WorkshopMod, b: WorkshopMod) =>
      sort === "size" ? b.size - a.size
      : sort === "version" ? b.updated - a.updated
      : sort === "used" ? lastUsed(b.id) - lastUsed(a.id)
      : sort === "updates" ? Number(needsUpdate(b)) - Number(needsUpdate(a))
      : byName(a, b);
    // ties go A–Z whichever way the column runs
    return [...list].sort((a, b) => dir * order(a, b) || byName(a, b));
  });

  /** A column's heading sorts by it; pressing it again turns the order round. */
  function sortBy(key: Sort) {
    if (sort === key) dir = dir === 1 ? -1 : 1;
    else { sort = key; dir = 1; }
  }
  /** The heading's aria-sort. Names start A–Z (ascending); the other columns start biggest or newest first. */
  function ariaSort(key: Sort) {
    if (sort !== key) return undefined;
    const ascending = key === "name" ? dir === 1 : dir === -1;
    return ascending ? "ascending" : "descending";
  }
  const subscribed = $derived((mods ?? []).filter((m) => m.subscribed));
  const onDisk = $derived(subscribed.reduce((n, m) => n + (m.installed ? m.size : 0), 0));
  const stale = $derived(subscribed.filter(needsUpdate));
  /** unsubscribed mods whose files Steam hasn't deleted yet */
  const pendingDelete = $derived((mods ?? []).filter((m) => !m.subscribed && m.installed));
  const chosenMods = $derived((mods ?? []).filter((m) => chosen.has(m.id)));
  const choosable = $derived(shown.filter((m) => m.subscribed));
  const allChosen = $derived(choosable.length > 0 && choosable.every((m) => chosen.has(m.id)));

  function choose(id: number, on: boolean) {
    const next = new Set(chosen);
    if (on) next.add(id); else next.delete(id);
    chosen = next;
  }
  function chooseAll(on: boolean) {
    chosen = on ? new Set(choosable.map((m) => m.id)) : new Set();
  }

  async function update(ids: number[]) {
    ids = ids.filter((id) => !updating.has(id));
    if (!ids.length) return;
    try {
      await api.updateMods(ids);
    } catch (e) { error = message(e); return; }
    const next = new Map(updating);
    for (const id of ids) next.set(id, null);
    updating = next;
    clearTimeout(timer);
    poll();
  }

  /** Follows Steam's downloads until each is done, then reads the list again. */
  async function poll() {
    const ids = [...updating.keys()];
    if (!ids.length) return;
    try {
      const items = await api.modProgress(ids);
      const next = new Map(updating);
      const done: number[] = [];
      items.forEach((it, i) => {
        const busy = it.state & (ITEM.needsUpdate | ITEM.downloading | ITEM.pending);
        if (it.state & ITEM.installed && !busy) done.push(ids[i]);
        else next.set(ids[i], it);
      });
      for (const id of done) next.delete(id);
      updating = next;
      if (done.length) {
        await load();
        // Steam says it's up to date although the Workshop's date is newer: the page was edited, not the files
        const nowChecked = { ...checked };
        for (const id of done) {
          const m = mods?.find((x) => x.id === id);
          if (m?.outdated) nowChecked[id] = m.latest;
        }
        checked = nowChecked;
        saveJson(CHECKED, nowChecked);
      }
    } catch (e) {
      error = message(e);
      updating = new Map();
      return;
    }
    if (updating.size) timer = window.setTimeout(poll, 1000);
  }

  async function unsubscribe() {
    unsubscribing = true;
    const ids = [...chosen];
    try {
      await api.unsubscribeMods(ids);
      saveUnsubscribed({ ...unsubscribedAt, ...Object.fromEntries(ids.map((id) => [id, now()])) });
      chosen = new Set();
      confirming = false;
      await load();
    } catch (e) { error = message(e); } finally { unsubscribing = false; }
  }

  /** Subscribes again to mods whose files Steam hasn't deleted yet. */
  async function undo(ids: number[]) {
    undoing = true;
    try {
      await api.subscribeMods(ids);
      saveUnsubscribed(Object.fromEntries(Object.entries(unsubscribedAt).filter(([id]) => !ids.includes(Number(id)))));
      await load();
    } catch (e) { error = message(e); } finally { undoing = false; }
  }

  const progress = (p: ItemProgress | null | undefined) =>
    !p ? "Asking Steam…" : p.total ? `${Math.round((p.done / p.total) * 100)}% of ${bytes(p.total)}` : "Waiting for Steam…";
</script>

<div class="view">
  <div class="inner">
    {#if error}<p class="danger">{error}</p>{/if}
    {#if !mods}
      {#if !error}<p class="note">Reading Steam's Workshop files…</p>{/if}
    {:else}
      <header class="sum">
        <p><b>{subscribed.length} mods</b> · {bytes(onDisk)} on disk · {stale.length ? `${stale.length} with an update` : "all up to date"}</p>
        {#if pendingDelete.length}
          <p class="pending note">{`${pendingDelete.length} unsubscribed (${bytes(pendingDelete.reduce((n, m) => n + m.size, 0))}): Steam deletes them once DayZ isn't running.`}</p>
        {/if}
        <div class="buttons">
          {#if stale.length}<Button primary onclick={() => update(stale.map((m) => m.id))}>{`Update all (${stale.length})`}</Button>{/if}
          {#if pendingDelete.length}<Button loading={undoing} onclick={() => undo(pendingDelete.map((m) => m.id))}>{`Subscribe again (${pendingDelete.length})`}</Button>{/if}
          <Button onclick={load}>Refresh</Button>
        </div>
      </header>

      <div class="tools">
        <input class="field search" type="search" placeholder="Find a mod" bind:value={q} aria-label="Find a mod" />
        <select class="field" value={sort} onchange={(e) => { sort = e.currentTarget.value as Sort; dir = 1; }} aria-label="Sort">
          <option value="updates">Updates first</option>
          <option value="name">Name</option>
          <option value="size">Size</option>
          <option value="version">Version</option>
          <option value="used">Last used</option>
        </select>
        {#if chosen.size}
          <span class="note">{`${chosen.size} chosen`}</span>
          <Button small onclick={() => update(chosenMods.filter(needsUpdate).map((m) => m.id))} disabled={!chosenMods.some(needsUpdate)}>Update</Button>
          <Button small onclick={() => (confirming = true)}>Unsubscribe</Button>
        {/if}
      </div>

      {#if confirming && chosen.size}
        {@const size = chosenMods.reduce((n, m) => n + (m.installed ? m.size : 0), 0)}
        {@const names = chosenMods.map(name)}
        {@const gone = chosenMods.filter((m) => m.gone).map(name)}
        <div class="confirm">
          <p>
            {`Unsubscribe from ${names.length > 3 ? `${names.slice(0, 3).join(", ")} and ${names.length - 3} more` : names.join(", ")}${size ? ` (${bytes(size)})` : ""}? Steam deletes the files once DayZ isn't running; until then this can be undone. A server that needs one gets it again when you join it.`}
            {#if gone.length}<span class="warn">{` ${gone.join(", ")} ${gone.length === 1 ? "is" : "are"} gone from the Workshop and can't be downloaded again.`}</span>{/if}
          </p>
          <div class="buttons">
            <Button small onclick={() => (confirming = false)}>Cancel</Button>
            <Button small primary loading={unsubscribing} onclick={unsubscribe}>Unsubscribe</Button>
          </div>
        </div>
      {/if}

      {#if !shown.length}
        <p class="note">{mods.length ? "No mod matches." : "No Workshop mods for DayZ on this PC yet."}</p>
      {:else}
        <table>
          <thead>
            <tr>
              <th class="c-pick"><input type="checkbox" checked={allChosen} disabled={!choosable.length} onchange={(e) => chooseAll(e.currentTarget.checked)} aria-label="Choose all" /></th>
              {#each COLUMNS as c (c.key)}
                {@const by = ariaSort(c.key)}
                <th class={c.cls} aria-sort={by}>
                  <button type="button" class="sort" class:on={!!by} onclick={() => sortBy(c.key)}>
                    {c.label}<span class="arrow" aria-hidden="true">{by === "ascending" ? "▲" : by === "descending" ? "▼" : "↕"}</span>
                  </button>
                </th>
              {/each}
              <th><span class="sr-only">Update</span></th>
            </tr>
          </thead>
          <tbody>
            {#each shown as m (m.id)}
              {@const users = history.using(m.id)}
              {@const p = updating.get(m.id)}
              <tr class:off={!m.subscribed}>
                <td class="c-pick">
                  {#if m.subscribed}<input type="checkbox" checked={chosen.has(m.id)} onchange={(e) => choose(m.id, e.currentTarget.checked)} aria-label="Choose {name(m)}" />{/if}
                </td>
                <td class="c-name">
                  <button type="button" class="name" onclick={() => openUrl(`https://steamcommunity.com/sharedfiles/filedetails/?id=${m.id}`)}>{name(m)}</button>
                  {#if !m.subscribed}<span class="note">{m.installed ? "Unsubscribed: Steam deletes it once DayZ isn't running" : "Unsubscribed"}</span>
                  {:else if users.length}<span class="note">{`Used by ${users[0].name}${users.length > 1 ? ` and ${users.length - 1} more` : ""}`}</span>
                  {:else if !lastUsed(m.id)}<span class="note">Not in any of DayZ's logs on this PC</span>{/if}
                  {#if m.gone}<span class="warn tagline">Gone or private on the Workshop: it gets no updates</span>{/if}
                </td>
                <td class="c-num">{m.size ? bytes(m.size) : "–"}</td>
                <td class="c-ver">
                  {#if updating.has(m.id)}
                    <span class="note">{progress(p)}</span>
                    <span class="bar"><i style:width="{p?.total ? (p.done / p.total) * 100 : 0}%"></i></span>
                  {:else if m.subscribed && !m.installed}<span class="warn">Not downloaded</span>
                  {:else}{day(m.updated)}{#if needsUpdate(m)}<span class="badge">Update</span>{/if}{/if}
                </td>
                <td class="c-used note">{lastUsed(m.id) ? day(lastUsed(m.id)) : "–"}</td>
                <td class="c-act">
                  {#if m.subscribed && needsUpdate(m) && !updating.has(m.id)}
                    <Button small onclick={() => update([m.id])}>{m.installed ? "Update" : "Download"}</Button>
                  {:else if !m.subscribed && m.installed}
                    <Button small loading={undoing} onclick={() => undo([m.id])}>Undo</Button>
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
        <p class="note foot">Updating and unsubscribing connect to Steam as DayZ for a moment, so Steam shows you as playing DayZ until it's done.</p>
      {/if}
    {/if}
  </div>
</div>

<style>
  .view { height: 100%; overflow: auto; }
  .inner { max-width: 1100px; margin: 0 auto; padding: var(--s-6) var(--s-5); display: flex; flex-direction: column; gap: var(--s-4); }
  .sum { display: flex; gap: var(--s-4); align-items: center; justify-content: space-between; flex-wrap: wrap; }
  .sum p { margin: 0; font-size: var(--t-lead); }
  .buttons { display: flex; gap: var(--s-3); align-items: center; }
  .tools { display: flex; gap: var(--s-3); align-items: center; flex-wrap: wrap; }
  .search { flex: 1 1 240px; }
  .confirm { display: flex; gap: var(--s-4); align-items: center; justify-content: space-between; padding: var(--s-3) var(--s-4); background: var(--inset); box-shadow: inset 3px 0 0 var(--warn); border-radius: var(--r-small); }
  .confirm p { margin: 0; }
  table { width: 100%; border-collapse: collapse; }
  th { text-align: left; font-size: var(--t-label); color: var(--muted); font-weight: 700; padding: var(--s-3); border-bottom: 1px solid var(--line); }
  td { padding: var(--s-3); border-bottom: 1px solid var(--line); vertical-align: middle; }
  .sort { display: inline-flex; gap: var(--s-2); align-items: baseline; border: 0; background: none; padding: 0; cursor: pointer; color: inherit; font: inherit; white-space: nowrap; }
  .sort:hover, .sort.on { color: var(--ink); }
  .sort:focus-visible { outline: 2px solid var(--amber); outline-offset: 2px; }
  .arrow { font-size: 10px; color: var(--rim); }
  .sort.on .arrow { color: var(--amber); }
  tr.off td:not(.c-act) { opacity: .55; }
  .pending { flex-basis: 100%; margin: 0; }
  .c-pick { width: 1%; }
  input[type="checkbox"] { accent-color: var(--amber); width: 16px; height: 16px; cursor: pointer; }
  .c-name { width: 100%; }
  .c-name .note, .tagline { display: block; font-size: var(--t-label); }
  .name { border: 0; background: none; padding: 0; text-align: left; cursor: pointer; color: var(--ink); font-weight: 700; font-size: var(--t-body); overflow-wrap: anywhere; }
  .name:hover { color: var(--amber); }
  .c-num, .c-ver, .c-used { white-space: nowrap; font-variant-numeric: tabular-nums; }
  .c-ver { min-width: 150px; }
  .badge { margin-left: var(--s-3); padding: 0 var(--s-3); border-radius: var(--r-small); background: var(--amber); color: var(--amber-ink); font-size: var(--t-label); font-weight: 800; }
  .bar { display: block; width: 140px; height: 4px; margin-top: 4px; background: var(--line); }
  .bar i { display: block; height: 100%; background: var(--amber); transition: width .3s; }
  .c-act { width: 1%; white-space: nowrap; }
  .foot { margin-top: var(--s-2); }
</style>
