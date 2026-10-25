<script lang="ts">
  /**
   * The right-click menu (lib/menu.svelte.ts): at the pointer, kept inside the window. Escape, a click elsewhere,
   * scrolling or leaving the window closes it; the arrow keys move between its items.
   */
  import { menu, type MenuAction } from "../lib/menu.svelte";

  let box = $state<HTMLElement>();
  let x = $state(0);
  let y = $state(0);
  /** the item that ran and now says so, just before the menu closes */
  let done = $state.raw<MenuAction | null>(null);
  let timer = 0;
  /** what had the focus before the menu opened */
  let before: HTMLElement | null = null;

  // at the pointer, or left of it or above it where it would leave the window
  $effect(() => {
    const open = menu.open;
    clearTimeout(timer);
    done = null;
    if (!open || !box) return;
    before = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const r = box.getBoundingClientRect();
    x = open.x + r.width > innerWidth - 4 ? Math.max(4, open.x - r.width) : open.x;
    y = open.y + r.height > innerHeight - 4 ? Math.max(4, open.y - r.height) : open.y;
    box.focus();
  });

  // scrolling anything closes it (a scroll doesn't bubble, so it's caught on the way down)
  $effect(() => {
    if (!menu.open) return;
    const close = () => menu.close();
    addEventListener("scroll", close, true);
    return () => removeEventListener("scroll", close, true);
  });

  /** Closes the menu and gives the focus back. */
  function close() {
    menu.close();
    before?.focus();
  }

  async function run(item: MenuAction) {
    if (!item.done) close();
    try {
      await item.run();
    } catch {
      close();
      return;
    }
    if (item.done) {
      done = item;
      timer = window.setTimeout(close, 800);
    }
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "Escape" || e.key === "Tab") {
      e.preventDefault();
      close();
    } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const items = [...box!.querySelectorAll<HTMLElement>("[role=menuitem]")];
      const i = items.indexOf(document.activeElement as HTMLElement);
      const next = i < 0 ? (e.key === "ArrowDown" ? 0 : items.length - 1) : (i + (e.key === "ArrowDown" ? 1 : items.length - 1)) % items.length;
      items[next]?.focus();
    }
  }
</script>

<svelte:window
  onpointerdown={(e) => { if (menu.open && !box?.contains(e.target as Node)) menu.close(); }}
  onblur={() => menu.close()}
  onresize={() => menu.close()}
/>

{#if menu.open}
  <div class="menu" role="menu" tabindex="-1" aria-label={menu.open.title ?? "Menu"} bind:this={box} style:left="{x}px" style:top="{y}px" {onkeydown}>
    {#if menu.open.title}<p class="title">{menu.open.title}</p>{/if}
    {#each menu.open.items as item, i (i)}
      {#if item === "-"}<hr />
      {:else}
        <button type="button" role="menuitem" class="item" class:done={done === item} onclick={() => run(item)}>{done === item ? `✓ ${item.done}` : item.label}</button>
      {/if}
    {/each}
  </div>
{/if}

<style>
  .menu { position: fixed; z-index: 20; width: max-content; min-width: 210px; max-width: 320px; padding: var(--s-2) 0; background: var(--panel); box-shadow: var(--shadow);
    border-radius: var(--r-surface); outline: none; }
  .title { margin: 0; padding: var(--s-2) var(--s-4) var(--s-3); color: var(--muted); font-size: var(--t-label); font-weight: 700; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .item { display: block; width: 100%; border: 0; background: none; padding: var(--s-3) var(--s-4); text-align: left; cursor: pointer; font-size: var(--t-body-small); }
  .item:hover, .item:focus-visible { background: var(--plate); outline: none; }
  .item.done { color: var(--ok); }
  hr { margin: var(--s-2) 0; border: 0; border-top: 1px solid var(--line); }
</style>
