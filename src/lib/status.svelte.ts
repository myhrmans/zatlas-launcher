/** Steam and DayZ on this PC, asked every few seconds; and the last server joined from the app. */
import { api, type Status } from "./api";

const EVERY = 4000;
const LAST = "zatlas.last";

class Live {
  now = $state<Status | null>(null);
  /** the last server joined from the app, and when (unix seconds) */
  last = $state<{ id: string; name: string; at: number } | null>(readLast());

  start() {
    const tick = () => api.status().then((s) => (this.now = s)).catch(() => {}).finally(() => window.setTimeout(tick, EVERY));
    tick();
  }

  joined(id: string, name: string) {
    this.last = { id, name, at: Math.floor(Date.now() / 1000) };
    try { localStorage.setItem(LAST, JSON.stringify(this.last)); } catch { /* kept for this session only */ }
  }
}

function readLast() {
  try { return JSON.parse(localStorage.getItem(LAST) || "null"); } catch { return null; }
}

export const live = new Live();
