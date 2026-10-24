<script lang="ts">
  /** Launch settings (the in-game name and more launch parameters), and where DayZ and the mod links are. */
  import { onMount } from "svelte";
  import { api, message, type Settings } from "../lib/api";
  import { live } from "../lib/status.svelte";
  import Button from "./Button.svelte";

  let s = $state<Settings>({ name: "", params: "" });
  let note = $state("");
  let saving = $state(false);

  onMount(() => { api.getSettings().then((x) => (s = x)).catch((e) => (note = message(e))); });

  async function save() {
    saving = true;
    try { await api.setSettings($state.snapshot(s)); note = "Saved."; } catch (e) { note = message(e); } finally { saving = false; }
  }
</script>

<div class="view">
  <form class="inner" onsubmit={(e) => { e.preventDefault(); save(); }}>
    <h2 class="plate">Launch</h2>
    <label>In-game name <span class="note">(empty keeps DayZ's own)</span>
      <input class="field" bind:value={s.name} maxlength="40" autocomplete="off" />
    </label>
    <label>More launch parameters <span class="note">e.g. <code>-nosplash -skipIntro -world=empty</code></span>
      <input class="field" bind:value={s.params} autocomplete="off" spellcheck="false" />
    </label>
    <div class="buttons"><Button primary type="submit" loading={saving}>Save</Button>{#if note}<span class="note">{note}</span>{/if}</div>

    <h2 class="plate">This PC</h2>
    <dl>
      <dt>DayZ</dt><dd>{live.now?.dayzDir ?? "not found"}</dd>
      <dt>Version</dt><dd>{live.now?.dayzVersion ?? "–"}</dd>
      <dt>Mod links</dt><dd>{live.now?.dayzDir ? `${live.now.dayzDir}\\!zatlas` : "–"}</dd>
    </dl>
    <p class="note">Joining links each server's mods from Steam's Workshop folder into <code>!zatlas</code> and starts DayZ with them. Nothing is copied, and the links can be deleted at any time.</p>
    <p class="note">Zatlas connects to Steam as DayZ only while it downloads mods for a join and while the Friends tab is open; Steam then shows you as playing DayZ.</p>
  </form>
</div>

<style>
  .view { height: 100%; overflow: auto; }
  .inner { max-width: 640px; margin: 0 auto; padding: var(--s-6) var(--s-5); display: flex; flex-direction: column; gap: var(--s-4); }
  h2.plate { font-size: var(--t-label); margin-top: var(--s-3); }
  label { display: grid; gap: var(--s-2); font-weight: 700; font-size: var(--t-body-small); }
  .buttons { display: flex; gap: var(--s-4); align-items: center; }
  dl { display: grid; grid-template-columns: auto 1fr; gap: var(--s-2) var(--s-5); margin: 0; font-size: var(--t-body-small); }
  dt { color: var(--muted); }
  dd { margin: 0; overflow-wrap: anywhere; }
  code { font: var(--t-code) var(--f-mono); }
</style>
