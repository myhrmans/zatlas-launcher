<script lang="ts">
  /** The logo: a Z tile with stencil cuts in the Z, and ZATLAS in the stencil font beside it. */
</script>

<span class="ui-wordmark">
  <svg viewBox="0 0 64 64" width="26" height="26" aria-hidden="true">
    <rect width="64" height="64" rx="14" fill="var(--panel)" stroke="var(--rim)" stroke-width="2" />
    <path d="M14 14h36v9L27 41h23v9H14v-9l23-18H14z" fill="var(--amber)" />
    <path d="M10 21h44M10 43h44M30 10v44" stroke="var(--panel)" stroke-width="3" />
  </svg>
  <span class="word">Zatlas</span>
</span>

<style>
  .ui-wordmark { display: inline-flex; align-items: center; gap: var(--s-3); vertical-align: middle; }
  svg { flex: none; }
  .word { font: 400 var(--t-wordmark)/1 var(--f-stencil); text-transform: uppercase; letter-spacing: .04em; color: var(--ink); }
</style>
