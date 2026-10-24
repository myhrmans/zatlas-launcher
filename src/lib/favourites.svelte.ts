/** Favourite servers, kept in this app's local storage: id and name, so a favourite that's offline still shows. */
const KEY = "zatlas.fav";
const MAX = 200;

export interface Favourite { id: string; name: string }

class Favourites {
  list = $state.raw<Favourite[]>(read());
  #ids = $derived(new Set(this.list.map((f) => f.id)));

  has(id: string) { return this.#ids.has(id); }

  toggle(s: Favourite) {
    const list = this.has(s.id) ? this.list.filter((f) => f.id !== s.id) : [...this.list, { id: s.id, name: s.name }].slice(-MAX);
    this.list = list;
    try { localStorage.setItem(KEY, JSON.stringify(list)); } catch { /* kept for this session only */ }
  }
}

function read(): Favourite[] {
  try {
    const raw = JSON.parse(localStorage.getItem(KEY) || "[]");
    return Array.isArray(raw) ? raw.filter((f) => typeof f?.id === "string" && typeof f?.name === "string").slice(0, MAX) : [];
  } catch { return []; }
}

export const favourites = new Favourites();
