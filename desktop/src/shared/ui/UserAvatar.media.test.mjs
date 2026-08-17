import assert from "node:assert/strict";
import { test } from "node:test";

import { JSDOM } from "jsdom";
import React, { act } from "react";
import { createRoot } from "react-dom/client";

import { beginRelayOriginFetch, resetMediaCaches } from "@/shared/lib/mediaUrl";
import { UserAvatar } from "./UserAvatar.tsx";

const HASH = "a".repeat(64);

test("UserAvatar rerenders historical media when relay authorization resolves", async () => {
  const previousWindow = globalThis.window;
  const previousDocument = globalThis.document;
  const previousImage = globalThis.Image;
  const previousActEnvironment = globalThis.IS_REACT_ACT_ENVIRONMENT;
  const dom = new JSDOM(
    "<!doctype html><html><body><div id='root'></div></body></html>",
  );
  const requestedSources = [];

  class LoadedImage {
    onload = null;
    onerror = null;

    set src(value) {
      requestedSources.push(value);
      queueMicrotask(() => this.onload?.());
    }
  }

  globalThis.window = dom.window;
  globalThis.document = dom.window.document;
  globalThis.Image = LoadedImage;
  dom.window.Image = LoadedImage;
  globalThis.IS_REACT_ACT_ENVIRONMENT = true;
  resetMediaCaches();

  const legacyUrl = `http://buzz.peakhunter.com:3000/media/${HASH}.png`;
  const root = createRoot(document.getElementById("root"));
  try {
    await act(async () => {
      root.render(
        React.createElement(UserAvatar, {
          avatarUrl: legacyUrl,
          displayName: "Historical User",
          fallbackDelayMs: 0,
          testId: "historical-avatar",
        }),
      );
      await Promise.resolve();
    });
    assert.equal(requestedSources.at(-1), legacyUrl);

    await act(async () => {
      beginRelayOriginFetch()("https://buzz.peakhunter.com:8443");
      await Promise.resolve();
    });

    assert.equal(
      requestedSources.at(-1),
      `buzz-media://localhost/media/${HASH}.png`,
    );
  } finally {
    await act(async () => root.unmount());
    resetMediaCaches();
    dom.window.close();
    globalThis.window = previousWindow;
    globalThis.document = previousDocument;
    globalThis.Image = previousImage;
    globalThis.IS_REACT_ACT_ENVIRONMENT = previousActEnvironment;
  }
});
