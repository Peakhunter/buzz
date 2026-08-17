import assert from "node:assert/strict";
import { test } from "node:test";

import { JSDOM } from "jsdom";
import React, { act } from "react";
import { createRoot } from "react-dom/client";

import { QuickBotBar } from "./QuickBotBar.tsx";
import { beginRelayOriginFetch, resetMediaCaches } from "@/shared/lib/mediaUrl";
import { TooltipProvider } from "@/shared/ui/tooltip";

const HASH = "a".repeat(64);

test("QuickBotBar rerenders historical media across origin resolution and reset", async () => {
  const previousWindow = globalThis.window;
  const previousDocument = globalThis.document;
  const previousActEnvironment = globalThis.IS_REACT_ACT_ENVIRONMENT;
  const dom = new JSDOM(
    "<!doctype html><html><body><div id='root'></div></body></html>",
  );
  globalThis.window = dom.window;
  globalThis.document = dom.window.document;
  globalThis.IS_REACT_ACT_ENVIRONMENT = true;
  resetMediaCaches();

  const legacyUrl = `http://buzz.peakhunter.com:3000/media/${HASH}.png`;
  const root = createRoot(document.getElementById("root"));
  try {
    await act(async () =>
      root.render(
        React.createElement(
          TooltipProvider,
          null,
          React.createElement(QuickBotBar, {
            personas: [
              {
                persona: {
                  id: "historical-agent",
                  displayName: "Historical Agent",
                  avatarUrl: legacyUrl,
                },
                instanceName: "Historical Agent",
              },
            ],
            pending: false,
            onAdd() {},
          }),
        ),
      ),
    );
    const image = document.querySelector('img[alt="Historical Agent"]');
    assert.equal(image?.getAttribute("src"), legacyUrl);

    await act(async () => {
      beginRelayOriginFetch()("https://buzz.peakhunter.com:8443");
    });
    assert.equal(
      image?.getAttribute("src"),
      `buzz-media://localhost/media/${HASH}.png`,
    );

    await act(async () => resetMediaCaches());
    assert.equal(image?.getAttribute("src"), legacyUrl);
  } finally {
    await act(async () => root.unmount());
    resetMediaCaches();
    dom.window.close();
    globalThis.window = previousWindow;
    globalThis.document = previousDocument;
    globalThis.IS_REACT_ACT_ENVIRONMENT = previousActEnvironment;
  }
});
