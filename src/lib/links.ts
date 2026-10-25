/** Finding the links in a server's description, so they can be pressed. */

/** a web address, with or without http(s)://: a host name ending in a word of letters, then an optional path */
const LINK = /\b(?:https?:\/\/)?(?:[a-z0-9-]+\.)+([a-z]{2,})(?::\d+)?(?:\/[^\s<>"]*)?/gi;
/** the endings a host name without http(s):// or www. needs to count as a link, so "file.txt" and "Hello.World" don't */
const ENDINGS = new Set(["com", "net", "org", "gg", "io", "app", "dev", "co", "me", "tv", "eu", "de", "uk", "fr", "nl", "se", "pl", "cz",
  "ru", "us", "ca", "au", "xyz", "info", "link", "site", "store", "shop", "online", "games", "fun", "club", "pro"]);

export interface Part { text: string; url?: string }

/** The text cut into plain parts and links; punctuation right after a link isn't part of it. */
export function linkify(text: string): Part[] {
  const parts: Part[] = [];
  let at = 0;
  for (const match of text.matchAll(LINK)) {
    const found = match[0].replace(/[.,;:!?)\]]+$/, "");
    const scheme = /^https?:\/\//i.test(found);
    if (!scheme && !/^www\./i.test(found) && !ENDINGS.has(match[1].toLowerCase())) continue;
    const start = match.index;
    if (start > at) parts.push({ text: text.slice(at, start) });
    parts.push({ text: found, url: scheme ? found.replace(/^https?/i, (s) => s.toLowerCase()) : `https://${found}` });
    at = start + found.length;
  }
  if (at < text.length) parts.push({ text: text.slice(at) });
  return parts;
}
