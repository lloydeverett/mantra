// Pure parsing of a commitment such as "Wash the dishes 20m". No DOM, so node --test can load it.

export const MIN_MINUTES = 1;
export const MAX_MINUTES = 4 * 60;

// 20m, 20min, 1h, 1h30m, 1h 30min; an optional space before each unit.
// Not inside a word or a number (so no 1.5h, 20mins or a20m).
const DURATION = /(?<![\w.])(\d+)\s?(?:h(?:\s?(\d+)\s?m(?:in)?)?|(m)(?:in)?)(?!\w)/gi;

// -> { commitment, duration, match }
// duration: minutes, or null if the text has no in-range duration.
// match: [start, end) of the duration in the text, or null.
// The first in-range duration wins; the commitment is the text without it.
export function parseCommitment(text) {
  for (const m of text.matchAll(DURATION)) {
    const [, n, mins, isMinutes] = m;
    const duration = isMinutes ? +n : +n * 60 + +(mins ?? 0);
    if (duration < MIN_MINUTES || duration > MAX_MINUTES) continue;
    const start = m.index, end = start + m[0].length;
    const before = text.slice(0, start), after = text.slice(end);
    const gap = /\s$/.test(before) || /^\s/.test(after) ? " " : "";
    const commitment = `${before.trimEnd()}${gap}${after.trimStart()}`.trim();
    return { commitment, duration, match: [start, end] };
  }
  return { commitment: text.trim(), duration: null, match: null };
}

// 20 -> "20m", 60 -> "1h", 90 -> "1h 30m"
export function formatDuration(minutes) {
  const h = Math.floor(minutes / 60), m = minutes % 60;
  return [h && `${h}h`, m && `${m}m`].filter(Boolean).join(" ");
}

// A commitment is one line: pasted or dropped line breaks become a single space.
export const singleLine = (text) => text.replace(/[^\S\r\n]*[\r\n]+\s*/g, " ");
