import type { ManagedAgent } from "@/shared/api/types";

const STARTER_PERSONA_IDS = new Set([
  "builtin:fizz",
  "builtin:honey",
  "builtin:bumble",
]);

export type StarterCleanupEntry = ManagedAgent & {
  cleanupBlockedReason: string | null;
};

function isFullHexPubkey(value: string): boolean {
  return /^[0-9a-f]{64}$/.test(value);
}

function cleanupBlockedReason(agent: ManagedAgent): string | null {
  if (!isFullHexPubkey(agent.pubkey)) {
    return "This identity does not have a valid full public key.";
  }
  if (agent.status !== "stopped") {
    return "Stop this identity before deleting it.";
  }
  return null;
}

/**
 * Returns only instantiated local Welcome Team identities. Definitions never
 * enter this list because listManagedAgents returns instances, not personas.
 */
export function getStarterCleanupEntries(
  agents: readonly ManagedAgent[],
): StarterCleanupEntry[] {
  return agents
    .filter(
      (agent) =>
        agent.backend.type === "local" &&
        agent.personaId !== null &&
        STARTER_PERSONA_IDS.has(agent.personaId),
    )
    .map((agent) => ({
      ...agent,
      cleanupBlockedReason: cleanupBlockedReason(agent),
    }))
    .sort(
      (left, right) =>
        left.name.localeCompare(right.name) ||
        left.createdAt.localeCompare(right.createdAt) ||
        left.pubkey.localeCompare(right.pubkey),
    );
}

/** Exact-pubkey lookup. Never falls back to name, persona, or relay URL. */
export function findStarterCleanupAgent(
  agents: readonly ManagedAgent[],
  pubkey: string,
): StarterCleanupEntry | null {
  return (
    getStarterCleanupEntries(agents).find(
      (candidate) => candidate.pubkey === pubkey,
    ) ?? null
  );
}

export function canConfirmExactManagedAgentDeletion(
  typedPubkey: string,
  targetPubkey: string,
): boolean {
  return isFullHexPubkey(targetPubkey) && typedPubkey.trim() === targetPubkey;
}

type RelayMembershipSource = {
  pubkey: string;
  channelIds: readonly string[];
};

type ChannelMembershipSource = {
  id: string;
  channelType: "stream" | "forum" | "dm";
  memberPubkeys: readonly string[];
};

/**
 * Union both active-relay membership sources, but return only mutable
 * stream/forum memberships. DM participation is immutable signed history and
 * is hidden by identity archival rather than channel-member removal.
 */
export function collectExactCleanupChannelIds(
  targetPubkey: string,
  relayAgents: readonly RelayMembershipSource[],
  channels: readonly ChannelMembershipSource[],
): string[] {
  const normalizedTarget = targetPubkey.trim().toLowerCase();
  const ids = new Set<string>();
  const dmIds = new Set(
    channels
      .filter((channel) => channel.channelType === "dm")
      .map((channel) => channel.id),
  );

  for (const relayAgent of relayAgents) {
    if (relayAgent.pubkey.trim().toLowerCase() !== normalizedTarget) continue;
    for (const channelId of relayAgent.channelIds) {
      if (channelId) ids.add(channelId);
    }
  }
  for (const channel of channels) {
    if (
      channel.memberPubkeys.some(
        (pubkey) => pubkey.trim().toLowerCase() === normalizedTarget,
      )
    ) {
      ids.add(channel.id);
    }
  }

  return [...ids].filter((channelId) => !dmIds.has(channelId)).sort();
}

type ExactStarterAgentDeleteInput = {
  targetPubkey: string;
  removeMembership: (channelId: string, pubkey: string) => Promise<unknown>;
  deleteManagedAgent: (pubkey: string) => Promise<unknown>;
  listManagedAgents: () => Promise<readonly ManagedAgent[]>;
  listMembershipChannelIds: (pubkey: string) => Promise<readonly string[]>;
};

/**
 * Executes the destructive operation in fail-closed order: fresh membership
 * discovery, owner-signed removals, proof that no mutable memberships remain,
 * a fresh eligibility read, then the exact backend transaction and exact
 * one-pubkey postcondition. The backend independently revalidates eligibility.
 */
export async function deleteExactStarterAgent({
  targetPubkey,
  removeMembership,
  deleteManagedAgent,
  listManagedAgents,
  listMembershipChannelIds,
}: ExactStarterAgentDeleteInput): Promise<void> {
  const initial = await listManagedAgents();
  const initialTarget = findStarterCleanupAgent(initial, targetPubkey);
  if (!initialTarget) {
    throw new Error(`Starter identity ${targetPubkey} was not found.`);
  }
  if (initialTarget.cleanupBlockedReason) {
    throw new Error(initialTarget.cleanupBlockedReason);
  }

  const channelIds = await listMembershipChannelIds(targetPubkey);
  for (const channelId of channelIds) {
    await removeMembership(channelId, targetPubkey);
  }
  const remainingChannelIds = await listMembershipChannelIds(targetPubkey);
  if (remainingChannelIds.length > 0) {
    throw new Error(
      `Membership verification failed for ${targetPubkey}: ${remainingChannelIds.length} membership(s) remain. Local deletion was not attempted.`,
    );
  }

  const beforeDelete = await listManagedAgents();
  const revalidatedTarget = findStarterCleanupAgent(beforeDelete, targetPubkey);
  if (!revalidatedTarget) {
    throw new Error(`Starter identity ${targetPubkey} was not found.`);
  }
  if (revalidatedTarget.cleanupBlockedReason) {
    throw new Error(revalidatedTarget.cleanupBlockedReason);
  }

  await deleteManagedAgent(targetPubkey);

  const after = await listManagedAgents();
  if (!verifyExactManagedAgentRemoval(beforeDelete, after, targetPubkey)) {
    throw new Error(
      "Exact cleanup verification failed: the managed identity set changed unexpectedly. Stop cleanup now.",
    );
  }
}

/**
 * Fail-closed postcondition: the after-set must equal the before-set with only
 * the requested exact pubkey removed.
 */
export function verifyExactManagedAgentRemoval(
  before: readonly Pick<ManagedAgent, "pubkey">[],
  after: readonly Pick<ManagedAgent, "pubkey">[],
  targetPubkey: string,
): boolean {
  const beforePubkeys = before.map((agent) => agent.pubkey).sort();
  const afterPubkeys = after.map((agent) => agent.pubkey).sort();
  const expected = beforePubkeys.filter((pubkey) => pubkey !== targetPubkey);

  return (
    beforePubkeys.includes(targetPubkey) &&
    !afterPubkeys.includes(targetPubkey) &&
    expected.length === afterPubkeys.length &&
    expected.every((pubkey, index) => pubkey === afterPubkeys[index])
  );
}
