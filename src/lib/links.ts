/** Finding the links in a server's description, so they can be pressed. */

/** a web address, with or without http(s)://: a host name ending in a word of letters, then an optional path */
const LINK = /\b(?:https?:\/\/)?(?:[a-z0-9-]+\.)+[a-z]{2,}(?::\d+)?(?:\/[^\s<>"]*)?/gi;

export interface Part { text: string; url?: string }

/** The text cut into plain parts and links; punctuation right after a link isn't part of it. */
export function linkify(text: string): Part[] {
  const parts: Part[] = [];
  let at = 0;
  for (const match of text.matchAll(LINK)) {
    const found = match[0].replace(/[.,;:!?)\]]+$/, "");
    const start = match.index;
    if (start > at) parts.push({ text: text.slice(at, start) });
    parts.push({ text: found, url: /^https?:\/\//i.test(found) ? found : `https://${found}` });
    at = start + found.length;
  }
  if (at < text.length) parts.push({ text: text.slice(at) });
  return parts;
}
