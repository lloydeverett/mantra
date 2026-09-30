// Run with: just test
import { test } from "node:test";
import assert from "node:assert/strict";
import { parseTags, toggleTag } from "./tags.js";
import { parseCommitment } from "./commitment.js";

const tags = (s) => {
  const { tags, text } = parseTags(s);
  return [text, tags];
};

test("no tags", () => {
  assert.deepEqual(tags("Wash the dishes"), ["Wash the dishes", []]);
  assert.deepEqual(parseTags("Wash the dishes").matches, []);
});

test("tags come out of the text, in TAGS order, once each", () => {
  assert.deepEqual(tags("Email Bob @work"), ["Email Bob", ["work"]]);
  assert.deepEqual(tags("@personal Call Mum"), ["Call Mum", ["personal"]]);
  assert.deepEqual(tags("Plan @personal the @work week"), ["Plan the week", ["work", "personal"]]);
  assert.deepEqual(tags("Email @work Bob @work"), ["Email Bob", ["work"]]);
  assert.deepEqual(tags("@work"), ["", ["work"]]);
});

test("case-insensitive", () => {
  assert.deepEqual(tags("Email Bob @Work"), ["Email Bob", ["work"]]);
  assert.deepEqual(tags("Email Bob @WORK"), ["Email Bob", ["work"]]);
});

test("not a tag: unknown names, longer words, mid-word @", () => {
  for (const s of ["Email Bob @home", "Go to @workshop", "Email bob@work.com", "Email Bob @", "Email Bob work"])
    assert.deepEqual(tags(s), [s, []], s);
});

test("punctuation next to the tag", () => {
  assert.deepEqual(tags("Email Bob @work, then lunch"), ["Email Bob, then lunch", ["work"]]);
  assert.deepEqual(tags("Email Bob (@work)"), ["Email Bob ()", ["work"]]);
  assert.deepEqual(tags("Email Bob @work."), ["Email Bob.", ["work"]]);
});

test("matches give each tag's position, and the span that removes it with a space", () => {
  assert.deepEqual(parseTags("Email @work Bob @WORK").matches, [
    { tag: "work", match: [6, 11], cut: [5, 11] },
    { tag: "work", match: [16, 21], cut: [15, 21] },
  ]);
  // At the start there is no space before, so the space after goes.
  assert.deepEqual(parseTags("@personal Call").matches, [{ tag: "personal", match: [0, 9], cut: [0, 10] }]);
});

test("tags and a duration together", () => {
  const { commitment, duration } = parseCommitment("Email Bob @work 20m");
  assert.equal(duration, 20);
  assert.deepEqual(tags(commitment), ["Email Bob", ["work"]]);
});

// Apply toggleTag's splices, as the prompt does to the entry.
const toggle = (s, tag, caret) => toggleTag(s, tag, caret).reduce((t, [start, end, insert]) => t.slice(0, start) + insert + t.slice(end), s);

test("toggling on appends the tag, spaced from the text", () => {
  assert.equal(toggle("Email Bob", "work"), "Email Bob @work");
  assert.equal(toggle("Email Bob @personal", "work"), "Email Bob @personal @work");
  assert.equal(toggle("Email Bob ", "work", 5), "Email Bob @work");
});

test("toggling on at the caret leaves a space to type into", () => {
  assert.equal(toggle("", "work", 0), " @work");
  assert.equal(toggle("Email Bob ", "work", 10), "Email Bob  @work");
  assert.equal(toggle("Email Bob", "work", 9), "Email Bob @work");
});

test("toggling off removes every copy of the tag and its space", () => {
  assert.equal(toggle("Email Bob @work", "work"), "Email Bob");
  assert.equal(toggle("@work Email Bob", "work"), "Email Bob");
  assert.equal(toggle("Email @Work Bob @work 20m", "work"), "Email Bob 20m");
  assert.equal(toggle("@work", "work"), "");
  assert.equal(toggle("Email Bob @work @personal", "personal"), "Email Bob @work");
});

test("toggleTag's splices run from the end, so earlier offsets stay valid", () => {
  assert.deepEqual(toggleTag("a @work b @work", "work"), [[9, 15, ""], [1, 7, ""]]);
  assert.deepEqual(toggleTag("a", "work"), [[1, 1, " @work"]]);
});

test("adjacent tags don't share a space", () => {
  assert.deepEqual(parseTags("@work @work").matches.map((m) => m.cut), [[0, 6], [6, 11]]);
  assert.equal(toggle("@work @work", "work"), "");
});
