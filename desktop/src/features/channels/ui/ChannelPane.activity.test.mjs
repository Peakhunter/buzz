import assert from "node:assert/strict";
import test from "node:test";

import { composeThreadActivityPubkeys } from "./threadComposerActivity.ts";

test("observer-only channel activity reaches the open thread composer", () => {
  const observerAgent = "ABCDEF";

  const channelComposerPubkeys = [observerAgent];
  const matchingThreadTypingPubkeys = [];
  const threadComposerPubkeys = composeThreadActivityPubkeys(
    channelComposerPubkeys,
    matchingThreadTypingPubkeys,
  );

  assert.deepEqual(threadComposerPubkeys, [observerAgent]);
  assert.deepEqual(threadComposerPubkeys, channelComposerPubkeys);
});

test("thread activity unions matching typing without case-insensitive duplicates", () => {
  assert.deepEqual(
    composeThreadActivityPubkeys(["ABCDEF"], ["abcdef", "123456"]),
    ["ABCDEF", "123456"],
  );
});
