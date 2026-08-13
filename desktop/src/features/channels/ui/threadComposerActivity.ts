export function composeThreadActivityPubkeys(
  channelWorkingPubkeys: readonly string[],
  threadTypingPubkeys: readonly string[],
): string[] {
  const seen = new Set<string>();
  return [...channelWorkingPubkeys, ...threadTypingPubkeys].filter((pubkey) => {
    const normalized = pubkey.toLowerCase();
    if (seen.has(normalized)) return false;
    seen.add(normalized);
    return true;
  });
}
