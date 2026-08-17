import assert from "node:assert/strict";
import { test } from "node:test";

import { JSDOM } from "jsdom";
import React, { act } from "react";
import { createRoot } from "react-dom/client";

import {
  beginRelayOriginFetch,
  resetMediaCaches,
  rewriteRelayUrl,
} from "./mediaUrl.ts";
import { useMediaProxyPort } from "./useMediaProxyPort.ts";

const HASH = "a".repeat(64);

test("useMediaProxyPort rerenders rewritten media when relay authorization resolves", async () => {
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
  function Harness() {
    useMediaProxyPort();
    return React.createElement("img", {
      alt: "historical media",
      src: rewriteRelayUrl(legacyUrl),
    });
  }

  const root = createRoot(document.getElementById("root"));
  try {
    await act(async () => root.render(React.createElement(Harness)));
    assert.equal(document.querySelector("img").getAttribute("src"), legacyUrl);

    await act(async () => {
      beginRelayOriginFetch()("https://buzz.peakhunter.com:8443");
    });

    assert.equal(
      document.querySelector("img").getAttribute("src"),
      `buzz-media://localhost/media/${HASH}.png`,
    );
  } finally {
    await act(async () => root.unmount());
    resetMediaCaches();
    dom.window.close();
    globalThis.window = previousWindow;
    globalThis.document = previousDocument;
    globalThis.IS_REACT_ACT_ENVIRONMENT = previousActEnvironment;
  }
});
