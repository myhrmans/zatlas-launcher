/** Map names, sizes and times in words. */

/** The game's map names (what servers report) → the names players use; others get a capital first letter. */
const MAPS: Record<string, string> = {
  chernarusplus: "Chernarus", enoch: "Livonia", sakhal: "Sakhal", namalsk: "Namalsk", deerisle: "Deer Isle",
  nasdara: "Nasdara", raman: "Raman", banov: "Banov", esseker: "Esseker", takistanplus: "Takistan", pnw: "Pacific Northwest",
  chiemsee: "Chiemsee", rostow: "Rostow", valning: "Valning", iztek: "Iztek", melkart: "Melkart", alteria: "Alteria",
  pripyat: "Pripyat", anastara: "Anastara", sumrakw: "Sumrak", onforin: "Onforin", lux: "Lux", swansisland: "Swans Island",
};

export const mapName = (map: string) => MAPS[map.toLowerCase()] ?? (map ? map[0].toUpperCase() + map.slice(1) : "–");

export function bytes(n: number): string {
  if (n >= 1e9) return `${(n / 1e9).toFixed(1)} GB`;
  if (n >= 1e6) return `${Math.round(n / 1e6)} MB`;
  if (n >= 1e3) return `${Math.round(n / 1e3)} kB`;
  return `${n} B`;
}

/** "today", "yesterday", "3 days ago", "12 Oct 2025" for a unix time in seconds */
export function day(unix: number): string {
  if (!unix) return "–";
  const days = Math.floor((Date.now() / 1000 - unix) / 86400);
  if (days < 1) return "today";
  if (days < 2) return "yesterday";
  if (days < 30) return `${days} days ago`;
  return new Date(unix * 1000).toLocaleDateString(undefined, { day: "numeric", month: "short", year: "numeric" });
}

/** "12 min", "2 h 5 min" since a unix time in seconds */
export function since(unix: number): string {
  const min = Math.max(0, Math.floor((Date.now() / 1000 - unix) / 60));
  return min < 60 ? `${min} min` : `${Math.floor(min / 60)} h ${min % 60} min`;
}

/** Why a player count may be made up, for each reason in the servers API's `doubt` list */
export const DOUBT: Record<string, string> = {
  flat: "Its count has stayed at nearly the same, almost full number for a whole day. Real servers dip at night or with a restart.",
  noqueue: "It has been full for a day and never had a login queue. A server that is really full gets one.",
  cluster: "Several full servers share its IP address, which is how a count gets inflated.",
};
export const DOUBT_TITLE = "This count looks doubtful. Open the server for why.";
export const FAKE_NOTE = "Likely a fake listing: its IP address lists hundreds of empty servers, or copies of one name.";

/** Whether a server runs the same game version as this PC (servers report "1.30.164200"). */
export const sameVersion = (server: string, local: string | null) => !local || !server || server === local;
