<script lang="ts">
  /**
   * Steam friends who are online: in DayZ first, with the server when Steam knows it (named from zatlas.app's list, with
   * Join and the server's right-click menu), then those in other games, then the rest. Refreshed while the tab is open; that keeps the app connected to
   * Steam as DayZ, which the note says.
   */
  import { onDestroy, onMount } from "svelte";
  import { api, message, type Friends, type ServerItem } from "../lib/api";
  import { mapName } from "../lib/format";
  import { joining } from "../lib/joining.svelte";
  import { menu } from "../lib/menu.svelte";
  import { serverMenu } from "../lib/serverMenu";
  import Button from "./Button.svelte";

  const EVERY = 20_000;
  let data = $state.raw<Friends | null>(null);
  let servers = $state.raw<Map<string, ServerItem>>(new Map());
  let error = $state("");
  let timer = 0;

  async function refresh() {
    try {
      const f = await api.friends();
      data = f;
      error = "";
      const ids = [...new Set(f.online.flatMap((x) => (x.server ? [x.server] : [])))];
      if (ids.length) {
        const r = await api.servers(`ids=${ids.join(",")}&limit=200`);
        servers = new Map(r.servers.map((s) => [s.id, s]));
      }
    } catch (e) {
      error = message(e);
    }
  }

  onMount(() => {
    const tick = () => refresh().finally(() => (timer = window.setTimeout(tick, EVERY)));
    tick();
  });
  onDestroy(() => clearTimeout(timer));

  const dayz = $derived(data?.online.filter((f) => f.inDayz) ?? []);
  const games = $derived(data?.online.filter((f) => !f.inDayz && f.app) ?? []);
  const rest = $derived(data?.online.filter((f) => !f.app) ?? []);
  const initial = (name: string) => [...name.trim()][0]?.toUpperCase() ?? "?";
</script>

<div class="view">
  <div class="inner">
    {#if error}
      <p class="danger">{error}</p>
    {:else if !data}
      <p class="note">Asking Steam…</p>
    {:else}
      <p class="note">Signed in as <b>{data.me}</b> · {data.online.length} of {data.total} friends online. While this tab is open, Steam shows you as playing DayZ.</p>

      <h2 class="plate">In DayZ ({dayz.length})</h2>
      {#if !dayz.length}<p class="note">No friend is in DayZ right now.</p>{/if}
      <ul>
        {#each dayz as f (f.id)}
          {@const s = f.server ? servers.get(f.server) : undefined}
          <li oncontextmenu={f.server ? (e) => menu.show(e, serverMenu(s ?? { id: f.server!, name: f.server! }), { title: s?.name ?? f.server! }) : undefined}>
            {#if f.avatar}<img class="av dayz" src={f.avatar} alt="" />{:else}<span class="av dayz">{initial(f.name)}</span>{/if}
            <span class="who"><b>{f.name}</b>
              <span class="note">
                {#if s}{s.name} · {mapName(s.map)} · {s.players}/{s.maxPlayers}
                {:else if f.server}On {f.server}
                {:else}{f.status ?? "In DayZ (Steam doesn't say which server)"}{/if}
              </span>
            </span>
            {#if f.server}<Button primary small onclick={() => joining.open(f.server!, s?.name ?? f.server!)}>Join</Button>{/if}
          </li>
        {/each}
      </ul>

      {#if games.length}
        <h2 class="plate">In other games ({games.length})</h2>
        <ul>
          {#each games as f (f.id)}<li>{#if f.avatar}<img class="av" src={f.avatar} alt="" />{:else}<span class="av">{initial(f.name)}</span>{/if}<span class="who"><b>{f.name}</b><span class="note">{f.game ? `In ${f.game}` : "In a game"}</span></span></li>{/each}
        </ul>
      {/if}
      {#if rest.length}
        <h2 class="plate">Online ({rest.length})</h2>
        <ul class="compact">
          {#each rest as f (f.id)}<li>{#if f.avatar}<img class="av" src={f.avatar} alt="" />{:else}<span class="av">{initial(f.name)}</span>{/if}<span class="who"><b>{f.name}</b><span class="note">{f.state === "online" ? "Online" : f.state === "away" ? "Away" : f.state === "busy" ? "Busy" : "Snoozing"}</span></span></li>{/each}
        </ul>
      {/if}
    {/if}
  </div>
</div>

<style>
  .view { height: 100%; overflow: auto; }
  .inner { max-width: 760px; margin: 0 auto; padding: var(--s-6) var(--s-5); display: flex; flex-direction: column; gap: var(--s-4); }
  h2.plate { font-size: var(--t-label); margin-top: var(--s-4); }
  ul { list-style: none; margin: 0; padding: 0; }
  li { display: flex; align-items: center; gap: var(--s-4); padding: var(--s-3) 0; border-bottom: 1px solid var(--line); }
  .compact { display: grid; grid-template-columns: repeat(auto-fill, minmax(220px, 1fr)); gap: 0 var(--s-5); }
  .av { flex: none; width: 32px; height: 32px; display: grid; place-items: center; border-radius: var(--r-small); background: var(--plate); color: var(--plate-ink); font-weight: 800; }
  .av.dayz { background: var(--amber); color: var(--amber-ink); }
  img.av { object-fit: cover; }
  img.av.dayz { box-shadow: 0 0 0 2px var(--amber); }
  .who { display: grid; min-width: 0; flex: 1; }
  .who b, .who .note { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
