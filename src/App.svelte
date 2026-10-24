<script lang="ts">
  /** The window: the brand and tabs on top with Steam's and DayZ's status, the tab's view, and the join dialog. */
  import FriendsView from "./components/FriendsView.svelte";
  import JoinDialog from "./components/JoinDialog.svelte";
  import ModsView from "./components/ModsView.svelte";
  import ServersView from "./components/ServersView.svelte";
  import SettingsView from "./components/SettingsView.svelte";
  import StatusChips from "./components/StatusChips.svelte";
  import Wordmark from "./components/Wordmark.svelte";
  import { joining } from "./lib/joining.svelte";
  import { live } from "./lib/status.svelte";

  type Tab = "servers" | "mods" | "friends" | "settings";
  let tab = $state<Tab>("servers");
  const TABS: [Tab, string][] = [["servers", "Servers"], ["mods", "Mods"], ["friends", "Friends"], ["settings", "Settings"]];

  live.start();
</script>

<div class="shell">
  <header class="top">
    <Wordmark />
    <nav aria-label="Views">
      {#each TABS as [key, label] (key)}
        <button type="button" aria-current={tab === key ? "page" : undefined} onclick={() => (tab = key)}>{label}</button>
      {/each}
    </nav>
    {#if joining.busy && !joining.target && joining.progress}
      <button type="button" class="busy" onclick={() => joining.open(joining.progress!.id, joining.progress!.name)}>Joining {joining.progress.name}…</button>
    {:else if joining.launch && !joining.target && (joining.starting || joining.launch.state === "failed")}
      {@const l = joining.launch}
      <button type="button" class="busy" class:bad={l.state === "failed"} onclick={() => joining.open(l.id, l.name)}>
        {l.state === "failed" ? "DayZ didn't get in: see why" : `Starting DayZ · ${l.elapsed} s`}
      </button>
    {/if}
    <StatusChips />
  </header>
  <main>
    {#if tab === "servers"}<ServersView />{:else if tab === "mods"}<ModsView />{:else if tab === "friends"}<FriendsView />{:else}<SettingsView />{/if}
  </main>
</div>
{#if joining.target}<JoinDialog />{/if}

<style>
  .shell { height: 100%; display: grid; grid-template-rows: auto 1fr; }
  .top { display: flex; align-items: center; gap: var(--s-6); padding: 0 var(--s-5); height: 52px; background: var(--panel); box-shadow: 0 1px 0 var(--edge), inset 0 -1px 0 var(--line); position: relative; z-index: 2; }
  nav { display: flex; gap: var(--s-2); align-self: stretch; }
  nav button { border: 0; background: none; color: var(--muted); padding: 0 var(--s-4); cursor: pointer; font-weight: 700; box-shadow: inset 0 -3px 0 transparent; }
  nav button:hover { color: var(--ink); }
  nav button[aria-current="page"] { color: var(--ink); box-shadow: inset 0 -3px 0 var(--amber); }
  nav button:focus-visible { outline: 2px solid var(--amber); outline-offset: -2px; }
  .busy { border: 0; background: var(--inset); box-shadow: inset 0 0 0 1px var(--amber); color: var(--amber); padding: var(--s-2) var(--s-4); cursor: pointer; font-weight: 700; font-size: var(--t-button-small); border-radius: var(--r-small); }
  .busy.bad { color: var(--danger); box-shadow: inset 0 0 0 1px var(--danger); }
  main { min-height: 0; overflow: hidden; }
</style>
