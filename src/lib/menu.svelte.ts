/** The right-click menu: one at a time, at the pointer, drawn by ContextMenu.svelte in place of the browser's own. */

export interface MenuAction {
  label: string;
  run: () => unknown;
  /** what the item says for a moment after it ran (e.g. "Address copied"), before the menu closes */
  done?: string;
}

/** an action, or "-" for a line between groups */
export type MenuItem = MenuAction | "-";

interface Open {
  x: number;
  y: number;
  /** a heading, e.g. the server's name */
  title?: string;
  /** what the menu is for, so it can be marked (the server's row) */
  key?: string;
  items: MenuItem[];
}

class Menu {
  open = $state.raw<Open | null>(null);

  /** Opens the menu for a right-click, or the keyboard's menu key, on the element with the handler. */
  show(e: MouseEvent, items: MenuItem[], { title, key }: { title?: string; key?: string } = {}) {
    e.preventDefault();
    let { clientX: x, clientY: y } = e;
    // from the keyboard the event has no pointer place: the menu goes under the element
    if (!x && !y && e.currentTarget instanceof Element) {
      const r = e.currentTarget.getBoundingClientRect();
      x = r.left + 16;
      y = r.bottom;
    }
    this.open = { x, y, title, key, items };
  }

  /** Whether the menu is open for `key`. */
  isFor(key: string) {
    return this.open?.key === key;
  }

  close() {
    this.open = null;
  }
}

export const menu = new Menu();
