<script lang="ts">
  /** The favourite star: an outline, filled amber when the server is a favourite. */
  import { favourites } from "../lib/favourites.svelte";

  let { id, name }: { id: string; name: string } = $props();
  const on = $derived(favourites.has(id));
</script>

<button type="button" class="star" class:on aria-pressed={on} aria-label="Favourite" title={on ? "Remove from favourites" : "Add to favourites"}
  onclick={(e) => { e.stopPropagation(); favourites.toggle({ id, name }); }}>
  <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true"><path d="M12 2.8l2.8 5.9 6.4.8-4.7 4.4 1.2 6.4L12 17.2l-5.7 3.1 1.2-6.4-4.7-4.4 6.4-.8z" /></svg>
</button>

<style>
  .star { flex: none; display: inline-grid; border: 0; background: none; padding: 0; cursor: pointer; color: var(--rim); }
  .star:hover { color: var(--muted); }
  svg { fill: none; stroke: currentColor; stroke-width: 1.8; stroke-linejoin: round; }
  .on { color: var(--amber); }
  .on svg { fill: currentColor; }
  .star:focus-visible { outline: 2px solid var(--amber); outline-offset: 2px; }
</style>
