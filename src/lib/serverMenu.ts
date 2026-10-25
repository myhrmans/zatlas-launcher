/**
 * A server's right-click menu, in the server list, the favourites and Friends: join it, share it, copy its address,
 * favourite it, ping it again, or see it on zatlas.app.
 */
import { openUrl } from "@tauri-apps/plugin-opener";
import type { ServerItem } from "./api";
import { favourites } from "./favourites.svelte";
import { joining } from "./joining.svelte";
import type { MenuItem } from "./menu.svelte";
import { pings } from "./ping.svelte";

/** The invite link for a server: a page on zatlas.app that opens the server in the app, or offers the app. */
export const inviteLink = (id: string) => `https://zatlas.app/join?s=${encodeURIComponent(id)}`;

/** The menu for a server. With only its id and name (an offline favourite), the items that need more are left out. */
export function serverMenu(s: Pick<ServerItem, "id" | "name"> & Partial<ServerItem>): MenuItem[] {
  const copy = (text: string) => navigator.clipboard.writeText(text);
  const { ip, gamePort, queryPort } = s;
  return [
    { label: "Join", run: () => joining.open(s.id, s.name) },
    { label: "Copy invite link", done: "Invite link copied", run: () => copy(inviteLink(s.id)) },
    ...(ip && gamePort ? [{ label: "Copy address", done: "Address copied", run: () => copy(`${ip}:${gamePort}`) }] : []),
    "-",
    { label: favourites.has(s.id) ? "Remove from favourites" : "Add to favourites", run: () => favourites.toggle({ id: s.id, name: s.name }) },
    ...(ip && queryPort ? [{ label: "Ping again", run: () => pings.measure([{ id: s.id, ip, queryPort }], true) }] : []),
    { label: "Open on zatlas.app", run: () => openUrl(`https://zatlas.app/servers?s=${encodeURIComponent(s.id)}`) },
  ];
}
