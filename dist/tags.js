// Pure parsing of tags such as "@work" in a commitment. No DOM, so node --test can load it.

// The only tags there are, in the order they're shown.
export const TAGS = ["work", "personal"];

// @work, @Work; at the start, or after a space or opening bracket, and not
// running on into a word (so no bob@work.com or @workshop).
const TAG = new RegExp(`(?<=^|[\\s([{])@(${TAGS.join("|")})(?!\\w)`, "gi");

// -> { text, tags, matches }
// text: the text without its tags. tags: the tags it has, once each, in TAGS order.
// matches: { tag, match, cut } per tag in the text, in order. match is [start, end)
// of the tag; cut is the span that removes it along with one space beside it.
export function parseTags(text) {
  const matches = [];
  let last = 0;
  for (const m of text.matchAll(TAG)) {
    const start = m.index, end = start + m[0].length;
    // Take the space before, unless the previous cut has it; else the space after.
    const cut = start > last && /\s/.test(text[start - 1]) ? [start - 1, end]
      : /\s/.test(text[end] ?? "") ? [start, end + 1] : [start, end];
    matches.push({ tag: m[1].toLowerCase(), match: [start, end], cut });
    last = cut[1];
  }
  const found = new Set(matches.map((m) => m.tag));
  return {
    text: splice(text, cuts(matches)).trim(),
    tags: TAGS.filter((t) => found.has(t)),
    matches,
  };
}

// The splices, as [start, end, insert] to apply in order, that toggle tag in text:
// remove every copy of it if there is one, else append it.
export function toggleTag(text, tag) {
  const matches = parseTags(text).matches.filter((m) => m.tag === tag);
  if (matches.length) return cuts(matches);
  const gap = text === "" || /\s$/.test(text) ? "" : " ";
  return [[text.length, text.length, `${gap}@${tag}`]];
}

// Removal splices for matches, last first so earlier offsets stay valid.
const cuts = (matches) => matches.map(({ cut: [start, end] }) => [start, end, ""]).reverse();

const splice = (text, splices) =>
  splices.reduce((t, [start, end, insert]) => t.slice(0, start) + insert + t.slice(end), text);
