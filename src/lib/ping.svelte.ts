/**
 * Each server's ping from this PC (src-tauri/src/ping.rs), measured when it's shown and kept for a minute. Pressing a
 * ping measures it again: the old value dims until the answer comes, and the new one flashes.
 */
import { api } from "./api";

const MAX_AGE = 60_000;

class Pings {
  /** server id → milliseconds, or null when it didn't answer */
  #ms = $state.raw(new Map<string, number | null>());
  /** the servers being measured again */
  #busy = $state.raw(new Set<string>());
  /** when each server was last measured again */
  #refreshed = $state.raw(new Map<string, number>());
  /** when each server was last measured */
  #at = new Map<string, number>();

  /** The server's ping: a number, null when it didn't answer, undefined when it hasn't been measured yet. */
  get(id: string) {
    return this.#ms.get(id);
  }

  /** Whether the server is being measured again. */
  busy(id: string) {
    return this.#busy.has(id);
  }

  /** When the server was last measured again (0: never), so its value can flash. */
  refreshed(id: string) {
    return this.#refreshed.get(id) ?? 0;
  }

  /** Measures the servers that haven't been measured in the last minute, or all of them with `again`. */
  async measure(servers: { id: string; ip: string; queryPort?: number | null }[], again = false) {
    const now = Date.now();
    const due = servers.filter((s) => s.queryPort && (again || now - (this.#at.get(s.id) ?? 0) > MAX_AGE));
    if (!due.length) return;
    for (const s of due) this.#at.set(s.id, now);
    if (again) this.#busy = new Set([...this.#busy, ...due.map((s) => s.id)]);
    try {
      const answers = await api.ping(due.map((s) => ({ id: s.id, ip: s.ip, queryPort: s.queryPort! })));
      const ms = new Map(this.#ms);
      for (const a of answers) ms.set(a.id, a.ms);
      this.#ms = ms;
      if (again) {
        const refreshed = new Map(this.#refreshed);
        for (const s of due) refreshed.set(s.id, Date.now());
        this.#refreshed = refreshed;
      }
    } catch {
      for (const s of due) this.#at.delete(s.id);
    } finally {
      if (again) this.#busy = new Set([...this.#busy].filter((id) => !due.some((s) => s.id === id)));
    }
  }
}

export const pings = new Pings();
