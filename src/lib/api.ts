/** The Rust side's commands (src-tauri/src/main.rs) and what they return. */
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface Status {
  steamRunning: boolean;
  steamSignedIn: boolean;
  dayzDir: string | null;
  dayzVersion: string | null;
  /** unix seconds, while DayZ runs */
  dayzStarted: number | null;
  steamConnected: boolean;
}

export interface ServerItem {
  id: string;
  name: string;
  ip: string;
  gamePort: number;
  /** the Steam query port, for the ping */
  queryPort: number;
  map: string;
  version: string;
  players: number;
  maxPlayers: number;
  firstPerson: boolean;
  modded: boolean;
  battleye: boolean;
  queue: number;
  dayAccel: number;
  nightAccel: number;
  gameTime: string;
  /** why the player count may be made up (format.ts has the texts) */
  doubt: string[];
  /** likely a fake listing */
  farm: boolean;
  /** a password is needed to join; null when the server hasn't said */
  password: boolean | null;
  modCount: number | null;
}

export interface ServerList { total: number; servers: ServerItem[] }

export interface ServerDetail {
  /** `description`: what the server's owner wrote about it; null until zatlas.app has read it */
  server: ServerItem & { description: string | null };
  mods: { id: number; name: string }[];
  history: { t: number; players: number }[];
}

export interface Facets {
  maps: { map: string; servers: number; players: number }[];
  versions: { version: string; servers: number }[];
  /** real listings, those with players, empty ones, and those hidden as fake (farms: empty copies and fake listings) */
  servers: number;
  withPlayers: number;
  empty: number;
  hidden: number;
  /** players the real listings claim */
  players: number;
}

/** `onWorkshop` false: gone or private on the Workshop (servers list such mods, often server-side ones); a join skips it */
export interface ModState { id: number; title: string | null; size: number; installed: boolean; outdated: boolean; onWorkshop: boolean }

export interface JoinProgress {
  id: string;
  name: string;
  /** "waiting": for a free slot on a full server */
  phase: "mods" | "waiting" | "starting" | "started" | "failed" | "cancelled";
  message: string | null;
  mods: { id: number; name: string; state: "ready" | "subscribing" | "waiting" | "downloading" | "skipped"; done: number; total: number }[];
  /** while waiting: the server's players and slots, or null when it didn't answer */
  slots: { players: number; max: number } | null;
}

export interface Friend {
  id: string;
  name: string;
  state: "online" | "away" | "busy" | "snooze";
  app: number | null;
  /** the game's name, for a game other than DayZ */
  game: string | null;
  inDayz: boolean;
  server: string | null;
  queryPort: number | null;
  status: string | null;
  /** a PNG data URL */
  avatar: string | null;
}

export interface Friends { me: string; total: number; online: Friend[] }

/** DayZ starting after a join, from its processes and logs (src-tauri/src/launch.rs); seconds since the start */
export interface Launch {
  id: string;
  name: string;
  elapsed: number;
  state: "running" | "ingame" | "failed";
  steps: { key: "battleye" | "game" | "window" | "scripts" | "connecting" | "ingame"; at: number | null }[];
  map: string | null;
  warning: string | null;
  problem: string | null;
  detail: string | null;
  gameRunning: boolean;
}

/** A Workshop mod on this PC (src-tauri/src/mods.rs); times in unix seconds */
export interface WorkshopMod {
  id: number;
  title: string | null;
  size: number;
  installed: boolean;
  subscribed: boolean;
  updated: number;
  latest: number;
  outdated: boolean;
  gone: boolean;
  linked: boolean;
  /** when DayZ last ran with it, from its logs; 0 if they don't say */
  lastUsed: number;
}

/** Steam's state bits for a Workshop item, and its download progress */
export interface ItemProgress { state: number; done: number; total: number }
export const ITEM = { subscribed: 1, installed: 4, needsUpdate: 8, downloading: 16, pending: 32 } as const;

export interface Settings { name: string; params: string }

/** A server's ping in milliseconds, or null when it didn't answer */
export interface Ping { id: string; ms: number | null }

export const api = {
  status: () => invoke<Status>("status"),
  servers: (query: string) => invoke<ServerList>("servers", { query }),
  facets: () => invoke<Facets>("facets"),
  server: (id: string, hours = 24) => invoke<ServerDetail>("server", { id, hours }),
  modStates: (ids: number[]) => invoke<ModState[]>("mod_states", { ids }),
  ping: (servers: { id: string; ip: string; queryPort: number }[]) => invoke<Ping[]>("ping", { servers }),
  join: (req: { id: string; password: string | null; closeRunning: boolean; waitForRoom: boolean }) => invoke<void>("join", { req }),
  cancelJoin: () => invoke<void>("cancel_join"),
  friends: () => invoke<Friends>("friends"),
  getSettings: () => invoke<Settings>("get_settings"),
  setSettings: (settings: Settings) => invoke<void>("set_settings", { settings }),
  onJoin: (f: (p: JoinProgress) => void): Promise<UnlistenFn> => listen<JoinProgress>("join", (e) => f(e.payload)),
  onLaunch: (f: (l: Launch) => void): Promise<UnlistenFn> => listen<Launch>("launch", (e) => f(e.payload)),
  openLogs: () => invoke<void>("open_logs"),
  /** opens a web link in the browser */
  openLink: (url: string) => invoke<void>("open_link", { url }),
  workshopMods: () => invoke<WorkshopMod[]>("workshop_mods"),
  modProgress: (ids: number[]) => invoke<ItemProgress[]>("mod_progress", { ids }),
  updateMods: (ids: number[]) => invoke<void>("update_mods", { ids }),
  subscribeMods: (ids: number[]) => invoke<void>("subscribe_mods", { ids }),
  unsubscribeMods: (ids: number[]) => invoke<void>("unsubscribe_mods", { ids }),
  closeGame: () => invoke<void>("close_game"),
};

/** A command's error as text: commands reject with a string. */
export const message = (e: unknown) => (typeof e === "string" ? e : e instanceof Error ? e.message : String(e));
