import * as React from "react";

import {
  getCachedMediaProxyPort,
  getMediaRewriteSnapshot,
  subscribeMediaProxyPort,
} from "./mediaUrl";

/**
 * The resolved localhost media-proxy port, re-rendering whenever the port or
 * relay-origin authorization changes. Components that call `rewriteRelayUrl`
 * during render use this to move from fail-closed, to an authorized custom
 * protocol URL, to the loopback proxy as each dependency resolves.
 */
export function useMediaProxyPort(): number | null {
  React.useSyncExternalStore(
    subscribeMediaProxyPort,
    getMediaRewriteSnapshot,
    () => "0::",
  );
  return getCachedMediaProxyPort();
}
