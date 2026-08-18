import assert from "node:assert/strict";
import test from "node:test";

import {
  canConfirmExactManagedAgentDeletion,
  collectExactCleanupChannelIds,
  deleteExactStarterAgent,
  findStarterCleanupAgent,
  getStarterCleanupEntries,
  verifyExactManagedAgentRemoval,
} from "./exactStarterAgentCleanup.ts";

const PUB_A = "a".repeat(64);
const PUB_B = "b".repeat(64);
const PUB_C = "c".repeat(64);
const PUB_D = "d".repeat(64);

function agent(overrides = {}) {
  return {
    pubkey: PUB_A,
    name: "Fizz",
    personaId: "builtin:fizz",
    relayUrl: "wss://relay-a.example",
    status: "stopped",
    backend: { type: "local" },
    createdAt: "2026-07-24T00:00:00Z",
    ...overrides,
  };
}

function sequence(...values) {
  let index = 0;
  return async () => values[Math.min(index++, values.length - 1)];
}

test("lists only local built-in starter instances and keeps duplicate names distinct", () => {
  const entries = getStarterCleanupEntries([
    agent({ pubkey: PUB_B, relayUrl: "wss://relay-b.example" }),
    agent({ pubkey: PUB_A }),
    agent({ pubkey: PUB_C, personaId: "custom:fizz" }),
    agent({
      pubkey: PUB_D,
      backend: { type: "provider", id: "x", config: {} },
    }),
  ]);

  assert.deepEqual(
    entries.map((entry) => entry.pubkey),
    [PUB_A, PUB_B],
  );
  assert.equal(entries[0].name, "Fizz");
  assert.equal(entries[1].name, "Fizz");
});

test("exact lookup never substitutes a same-named identity", () => {
  const agents = [
    agent({ pubkey: PUB_A, relayUrl: "wss://old.example" }),
    agent({ pubkey: PUB_B, relayUrl: "wss://current.example" }),
  ];

  assert.equal(
    findStarterCleanupAgent(agents, PUB_A)?.relayUrl,
    "wss://old.example",
  );
  assert.equal(
    findStarterCleanupAgent(agents, PUB_B)?.relayUrl,
    "wss://current.example",
  );
  assert.equal(findStarterCleanupAgent(agents, PUB_C), null);
});

test("confirmation requires the complete exact lowercase pubkey", () => {
  assert.equal(canConfirmExactManagedAgentDeletion(PUB_A, PUB_A), true);
  assert.equal(canConfirmExactManagedAgentDeletion(` ${PUB_A} `, PUB_A), true);
  assert.equal(
    canConfirmExactManagedAgentDeletion(PUB_A.toUpperCase(), PUB_A),
    false,
  );
  assert.equal(
    canConfirmExactManagedAgentDeletion(PUB_A.slice(0, 12), PUB_A),
    false,
  );
  assert.equal(canConfirmExactManagedAgentDeletion("Fizz", PUB_A), false);
  assert.equal(canConfirmExactManagedAgentDeletion(PUB_B, PUB_A), false);
});

test("post-delete verification accepts only the exact one-pubkey delta", () => {
  const before = [agent({ pubkey: PUB_A }), agent({ pubkey: PUB_B })];

  assert.equal(
    verifyExactManagedAgentRemoval(before, [agent({ pubkey: PUB_B })], PUB_A),
    true,
  );
  assert.equal(
    verifyExactManagedAgentRemoval(before, [agent({ pubkey: PUB_A })], PUB_A),
    false,
    "wrong identity removed",
  );
  assert.equal(
    verifyExactManagedAgentRemoval(before, [], PUB_A),
    false,
    "more than one identity removed",
  );
  assert.equal(
    verifyExactManagedAgentRemoval(before, before, PUB_A),
    false,
    "target still present",
  );
});

test("running starter instances remain visible but are ineligible", () => {
  const [entry] = getStarterCleanupEntries([
    agent({ status: "running", pubkey: PUB_A }),
  ]);
  assert.equal(entry.pubkey, PUB_A);
  assert.equal(
    entry.cleanupBlockedReason,
    "Stop this identity before deleting it.",
  );
});

test("membership discovery unions relay profiles and channel member lists", () => {
  assert.deepEqual(
    collectExactCleanupChannelIds(
      PUB_A,
      [
        { pubkey: PUB_A, channelIds: ["relay-only", "both"] },
        { pubkey: PUB_B, channelIds: ["other"] },
      ],
      [
        { id: "channel-only", memberPubkeys: [PUB_A] },
        { id: "both", memberPubkeys: [PUB_A, PUB_B] },
      ],
    ),
    ["both", "channel-only", "relay-only"],
  );
});

test("membership failure prevents local exact-pubkey deletion", async () => {
  const calls = [];
  const before = [agent({ pubkey: PUB_A }), agent({ pubkey: PUB_B })];
  await assert.rejects(
    deleteExactStarterAgent({
      targetPubkey: PUB_A,
      listManagedAgents: async () => before,
      listMembershipChannelIds: async () => ["channel-a", "channel-b"],
      removeMembership: async (channelId, pubkey) => {
        calls.push(["remove", channelId, pubkey]);
        if (channelId === "channel-b") throw new Error("membership rejected");
      },
      deleteManagedAgent: async (pubkey) => calls.push(["delete", pubkey]),
    }),
    /membership rejected/,
  );

  assert.equal(
    calls.some(([operation]) => operation === "delete"),
    false,
  );
  assert.deepEqual(
    calls.filter(([operation]) => operation === "remove"),
    [
      ["remove", "channel-a", PUB_A],
      ["remove", "channel-b", PUB_A],
    ],
  );
});

test("successful cleanup proves memberships absent before exact deletion", async () => {
  const calls = [];
  const before = [agent({ pubkey: PUB_A }), agent({ pubkey: PUB_B })];
  const after = [agent({ pubkey: PUB_B })];

  await deleteExactStarterAgent({
    targetPubkey: PUB_A,
    listManagedAgents: sequence(before, before, after),
    listMembershipChannelIds: sequence(["channel-a"], []),
    removeMembership: async (channelId, pubkey) =>
      calls.push(["remove", channelId, pubkey]),
    deleteManagedAgent: async (pubkey) => calls.push(["delete", pubkey]),
  });

  assert.deepEqual(calls, [
    ["remove", "channel-a", PUB_A],
    ["delete", PUB_A],
  ]);
});

test("identity becoming active during cleanup blocks backend deletion", async () => {
  let deleted = false;
  await assert.rejects(
    deleteExactStarterAgent({
      targetPubkey: PUB_A,
      listManagedAgents: sequence(
        [agent({ pubkey: PUB_A })],
        [agent({ pubkey: PUB_A, status: "running" })],
      ),
      listMembershipChannelIds: async () => [],
      removeMembership: async () => undefined,
      deleteManagedAgent: async () => {
        deleted = true;
      },
    }),
    /Stop this identity/,
  );
  assert.equal(deleted, false);
});

test("unexpected post-delete identity delta is a hard failure", async () => {
  const before = [agent({ pubkey: PUB_A }), agent({ pubkey: PUB_B })];
  await assert.rejects(
    deleteExactStarterAgent({
      targetPubkey: PUB_A,
      listManagedAgents: sequence(before, before, []),
      listMembershipChannelIds: async () => [],
      removeMembership: async () => undefined,
      deleteManagedAgent: async () => undefined,
    }),
    /identity set changed unexpectedly/,
  );
});
