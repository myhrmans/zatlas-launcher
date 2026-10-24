/** The servers joined from the app and their mods, newest first: the Mods tab says which servers use a mod, and when it
    was last used. Kept in this app's local storage. */
const KEY = "zatlas.joins";
const MAX = 100;

export interface Joined { id: string; name: string; at: number; mods: number[] }

class History {
  list = $state.raw<Joined[]>(read());

  add(id: string, name: string, mods: number[]) {
    const at = Math.floor(Date.now() / 1000);
    this.list = [{ id, name, at, mods }, ...this.list.filter((j) => j.id !== id)].slice(0, MAX);
    try { localStorage.setItem(KEY, JSON.stringify(this.list)); } catch { /* kept for this session only */ }
  }

  /** The joined servers that use this mod, newest first. */
  using(mod: number): Joined[] {
    return this.list.filter((j) => j.mods.includes(mod));
  }
}

function read(): Joined[] {
  try {
    const raw = JSON.parse(localStorage.getItem(KEY) || "[]");
    return Array.isArray(raw)
      ? raw.filter((j) => typeof j?.id === "string" && typeof j?.name === "string" && typeof j?.at === "number" && Array.isArray(j?.mods)).slice(0, MAX)
      : [];
  } catch { return []; }
}

export const history = new History();
