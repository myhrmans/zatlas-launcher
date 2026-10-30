/**
 * Player counts asked of the servers themselves (src-tauri/src/a2s.rs), for when zatlas.app's count is a few minutes
 * old. Pressing a count asks again: the old value dims until the answer comes, and the new one flashes. A list loaded
 * again brings its own counts, so it drops these.
 */
import { api, type Slots } from "./api";

class Counts {
  /** server id → its players and slots, or null when it didn't answer */
  #slots = $state.raw(new Map<string, Slots | null>());
  /** the servers being asked */
  #busy = $state.raw(new Set<string>());
  /** when each server last answered (or didn't) */
  #at = $state.raw(new Map<string, number>());

  /** The server's count: its players and slots, null when it didn't answer, undefined when it hasn't been asked. */
  get(id: string) {
    return this.#slots.get(id);
  }

  /** Whether the server is being asked. */
  busy(id: string) {
    return this.#busy.has(id);
  }

  /** When the server was last asked (0: never), so its count can flash. */
  refreshed(id: string) {
    return this.#at.get(id) ?? 0;
  }

  /** Asks the server for its count now. */
  async refresh(s: { id: string; ip: string; queryPort?: number | null }) {
    if (!s.queryPort || this.#busy.has(s.id)) return;
    this.#busy = new Set([...this.#busy, s.id]);
    try {
      const slots = await api.slots(s.ip, s.queryPort);
      this.#slots = new Map(this.#slots).set(s.id, slots);
      this.#at = new Map(this.#at).set(s.id, Date.now());
    } catch {
      /* the count stays as it was */
    } finally {
      this.#busy = new Set([...this.#busy].filter((id) => id !== s.id));
    }
  }

  /** Drops the counts asked so far, for a list that has just loaded. */
  forget() {
    if (this.#slots.size) this.#slots = new Map();
  }
}

export const counts = new Counts();
