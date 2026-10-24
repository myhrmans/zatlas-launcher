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

  constructor() {
    api.onJoin((p) => {
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
  get busy() { return !!this.progress && (this.progress.phase === "mods" || this.progress.phase === "starting"); }

  open(id: string, name: string) {
    this.target = { id, name };
    if (!this.busy && !this.starting && this.progress?.id !== id) this.progress = null;
  }

  close() {
    this.target = null;
    if (!this.busy && !this.starting) this.progress = null;
    if (this.launch && !this.starting) this.launch = null;
  }

  async start(id: string, name: string, password: string, closeRunning: boolean) {
    this.progress = { id, name, phase: "mods", message: null, mods: [] };
    this.launch = null;
    try {
      await api.join({ id, password: password || null, closeRunning });
    } catch (e) {
      this.progress = { id, name, phase: "failed", message: message(e), mods: [] };
    }
  }
}

export const joining = new Joining();
