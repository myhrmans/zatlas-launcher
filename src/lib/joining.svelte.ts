/** The join in progress: the dialog's server, and the progress the Rust side reports through `join` events. */
import { api, message, type JoinProgress, type Launch } from "./api";
import { history } from "./history.svelte";
import { live } from "./status.svelte";

class Joining {
  /** the server the join dialog is open for */
  target = $state<{ id: string; name: string } | null>(null);
  progress = $state<JoinProgress | null>(null);
  /** DayZ starting after the join */
  launch = $state<Launch | null>(null);
  /** when the join started waiting for a free slot */
  waitingSince = $state<number | null>(null);

  constructor() {
    api.onJoin((p) => {
      if (p.phase === "waiting" && this.progress?.phase !== "waiting") this.waitingSince = Date.now();
      this.progress = p;
      if (p.phase === "started") {
        live.joined(p.id, p.name);
        history.add(p.id, p.name, p.mods.filter((m) => m.state !== "skipped").map((m) => m.id));
      }
    });
    api.onLaunch((l) => (this.launch = l));
  }

  /** Whether DayZ is starting (the dialog may be closed meanwhile). */
  get starting() { return this.launch?.state === "running"; }

  /** Whether a join is running (the dialog may be closed meanwhile). */
  get busy() { return !!this.progress && ["mods", "waiting", "starting"].includes(this.progress.phase); }

  open(id: string, name: string) {
    this.target = { id, name };
    // a join's progress stays while it runs, and when DayZ didn't get in (to see why), but not once DayZ is in
    const done = this.progress?.phase === "started" && this.launch?.state !== "running" && this.launch?.state !== "failed";
    if (!this.busy && !this.starting && (this.progress?.id !== id || done)) {
      this.progress = null;
      if (done) this.launch = null;
    }
  }

  close() {
    this.target = null;
    if (!this.busy && !this.starting) this.progress = null;
    if (this.launch && !this.starting) this.launch = null;
  }

  /** Starts a join; `waitForRoom`: when the server is full, wait for a free slot before starting DayZ. */
  async start(id: string, name: string, password: string, closeRunning: boolean, waitForRoom = false) {
    this.progress = { id, name, phase: "mods", message: null, mods: [], slots: null };
    this.launch = null;
    try {
      await api.join({ id, password: password || null, closeRunning, waitForRoom });
    } catch (e) {
      this.progress = { id, name, phase: "failed", message: message(e), mods: [], slots: null };
    }
  }
}

export const joining = new Joining();
