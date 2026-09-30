// Run with: just test
import { test } from "node:test";
import assert from "node:assert/strict";
import { parseCommitment, formatDuration, clockText, singleLine } from "./commitment.js";

const parse = (s) => {
  const { commitment, duration } = parseCommitment(s);
  return [commitment, duration];
};

test("no duration", () => {
  assert.deepEqual(parse("Wash the dishes"), ["Wash the dishes", null]);
  assert.deepEqual(parseCommitment("Wash the dishes").match, null);
});

test("empty text", () => {
  assert.deepEqual(parse(""), ["", null]);
  assert.deepEqual(parse("   "), ["", null]);
});

test("minutes", () => {
  assert.deepEqual(parse("Wash the dishes 20m"), ["Wash the dishes", 20]);
  assert.deepEqual(parse("Wash the dishes 20min"), ["Wash the dishes", 20]);
  assert.deepEqual(parse("Wash the dishes 20 m"), ["Wash the dishes", 20]);
  assert.deepEqual(parse("Wash the dishes 20 min"), ["Wash the dishes", 20]);
});

test("hours and hours with minutes", () => {
  assert.deepEqual(parse("Write 1h"), ["Write", 60]);
  assert.deepEqual(parse("Write 2 h"), ["Write", 120]);
  assert.deepEqual(parse("Write 1h30m"), ["Write", 90]);
  assert.deepEqual(parse("Write 1h 30m"), ["Write", 90]);
  assert.deepEqual(parse("Write 1h30min"), ["Write", 90]);
});

test("case-insensitive", () => {
  assert.deepEqual(parse("Write 1H30M"), ["Write", 90]);
  assert.deepEqual(parse("Write 20MIN"), ["Write", 20]);
});

test("anywhere in the text, first match wins", () => {
  assert.deepEqual(parse("20m wash the dishes"), ["wash the dishes", 20]);
  assert.deepEqual(parse("Wash 20m the dishes"), ["Wash the dishes", 20]);
  assert.deepEqual(parse("1h Watch 30m of TV"), ["Watch 30m of TV", 60]);
});

test("match gives the token's position", () => {
  assert.deepEqual(parseCommitment("Wash the dishes 20m").match, [16, 19]);
  assert.deepEqual(parseCommitment("Write 1h 30m now").match, [6, 12]);
});

test("range is 1m to 4h; outside it is plain text", () => {
  assert.deepEqual(parse("Nap 1m"), ["Nap", 1]);
  assert.deepEqual(parse("Nap 4h"), ["Nap", 240]);
  assert.deepEqual(parse("Nap 0m"), ["Nap 0m", null]);
  assert.deepEqual(parse("Nap 4h1m"), ["Nap 4h1m", null]);
  assert.deepEqual(parse("Nap 300m"), ["Nap 300m", null]);
  // An out-of-range token doesn't block a later valid one.
  assert.deepEqual(parse("Lift 0m reps 10m"), ["Lift 0m reps", 10]);
});

test("not a duration: seconds, decimals, other units, inside words", () => {
  for (const s of ["Rest 30s", "Read 1.5h", "Walk 5km", "Write 20mins", "Study 2hr", "Read a20m", "Run 20m2"])
    assert.deepEqual(parse(s), [s, null], s);
});

test("punctuation next to the token", () => {
  assert.deepEqual(parse("Read (20m)"), ["Read ()", 20]);
  assert.deepEqual(parse("Read, 20m."), ["Read, .", 20]);
});

test("formatDuration", () => {
  assert.equal(formatDuration(1), "1m");
  assert.equal(formatDuration(45), "45m");
  assert.equal(formatDuration(60), "1h");
  assert.equal(formatDuration(90), "1h 30m");
  assert.equal(formatDuration(240), "4h");
});

test("clockText rounds up to whole seconds", () => {
  assert.equal(clockText(0), "0:00");
  assert.equal(clockText(1), "0:01");
  assert.equal(clockText(59_000), "0:59");
  assert.equal(clockText(20 * 60_000), "20:00");
  assert.equal(clockText(3_600_000), "1:00:00");
  assert.equal(clockText(2 * 3_600_000 - 1), "2:00:00");
  assert.equal(clockText(3_661_000), "1:01:01");
});

test("singleLine turns line breaks into single spaces", () => {
  assert.equal(singleLine("Wash the dishes"), "Wash the dishes");
  assert.equal(singleLine("Wash\nthe dishes"), "Wash the dishes");
  assert.equal(singleLine("Wash \r\n\r\n  the dishes\n"), "Wash the dishes ");
  assert.equal(singleLine("Wash  the dishes"), "Wash  the dishes"); // other spacing is left alone
});
