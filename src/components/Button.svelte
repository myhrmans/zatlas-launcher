<script lang="ts">
  /**
   * A button with two cut corners. `primary` makes it amber; `small` is the tighter size. `disabled` dims it;
   * `loading` shows a moving stripe and blocks clicks (the label stays, so the button keeps its width).
   */
  import type { Snippet } from "svelte";
  import type { HTMLButtonAttributes } from "svelte/elements";

  let { primary = false, small = false, loading = false, disabled = false, children, ...rest }:
    { primary?: boolean; small?: boolean; loading?: boolean; disabled?: boolean; children: Snippet } & HTMLButtonAttributes = $props();
</script>

<button type="button" class="ui-btn" class:primary class:small class:loading disabled={disabled || loading} aria-busy={loading || undefined} {...rest}>{@render children()}</button>

<style>
  .ui-btn { position: relative; display: inline-flex; align-items: center; gap: 6px; cursor: pointer; border: 0; clip-path: var(--cut-6); padding: 10px 15px;
    background: var(--inset); color: var(--ink); box-shadow: inset 0 0 0 1px var(--line); font: 700 var(--t-button)/1 var(--f-text); }
  .primary { background: var(--amber); color: var(--amber-ink); box-shadow: none; }
  .small { padding: var(--s-3) var(--s-4); font-size: var(--t-button-small); }
  .ui-btn:hover:not(:disabled) { filter: brightness(1.1); }
  .ui-btn:focus-visible { outline: 2px solid var(--amber); outline-offset: 2px; background: var(--ink); color: var(--panel); }
  .ui-btn:disabled { opacity: var(--disabled-opacity); cursor: not-allowed; }
  .loading:disabled { opacity: 1; cursor: progress; }
  .loading::after { content: ""; position: absolute; inset: auto 0 0; height: 3px; background: repeating-linear-gradient(-45deg, var(--amber) 0 6px, transparent 6px 12px);
    background-size: 17px 100%; animation: ui-busy var(--skeleton-ms) linear infinite; }
  .primary.loading::after { background: repeating-linear-gradient(-45deg, var(--amber-ink) 0 6px, transparent 6px 12px); }
  @keyframes ui-busy { to { background-position: 17px 0; } }
  @media (prefers-reduced-motion: reduce) { .loading::after { animation: none; } }
</style>
