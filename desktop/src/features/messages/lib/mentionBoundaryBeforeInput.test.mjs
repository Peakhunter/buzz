import assert from "node:assert/strict";
import test from "node:test";

import { Schema } from "@tiptap/pm/model";
import { EditorState, TextSelection } from "@tiptap/pm/state";

import { handleMentionBoundaryBeforeInput } from "./mentionBoundaryBeforeInput.ts";

const schema = new Schema({
  nodes: {
    doc: { content: "block+" },
    paragraph: { group: "block", content: "inline*" },
    text: { group: "inline" },
  },
});

function createView(text, options = {}) {
  const doc = schema.node("doc", null, [
    schema.node("paragraph", null, text ? [schema.text(text)] : []),
  ]);
  const from = options.from ?? 1 + text.length;
  const to = options.to ?? from;
  let state = EditorState.create({
    doc,
    selection: TextSelection.create(doc, from, to),
  });
  let dispatchCount = 0;

  return {
    get state() {
      return state;
    },
    composing: options.composing ?? false,
    dispatch(transaction) {
      dispatchCount += 1;
      state = state.apply(transaction);
    },
    get dispatchCount() {
      return dispatchCount;
    },
  };
}

function createBeforeInput(overrides = {}) {
  let prevented = false;
  return {
    inputType: "insertText",
    data: "t",
    isComposing: false,
    preventDefault() {
      prevented = true;
    },
    get prevented() {
      return prevented;
    },
    ...overrides,
  };
}

test("inserts the first character after a highlighted agent mention through ProseMirror", () => {
  const view = createView("@Reinhold ");
  const event = createBeforeInput();

  assert.equal(
    handleMentionBoundaryBeforeInput(view, event, ["Reinhold"]),
    true,
  );
  assert.equal(event.prevented, true);
  assert.equal(view.dispatchCount, 1);
  assert.equal(view.state.doc.textContent, "@Reinhold t");
  assert.equal(
    view.state.doc.textContent.codePointAt("@Reinhold".length),
    0x20,
  );
  assert.equal(view.state.selection.from, 1 + "@Reinhold t".length);
});

test("leaves ordinary typing outside an agent-mention boundary to the browser", () => {
  const view = createView("ordinary ");
  const event = createBeforeInput();

  assert.equal(
    handleMentionBoundaryBeforeInput(view, event, ["Reinhold"]),
    false,
  );
  assert.equal(event.prevented, false);
  assert.equal(view.dispatchCount, 0);
});

test("requires a collapsed selection after a U+0020 separator", () => {
  const selectedView = createView("@Reinhold ", { from: 1, to: 2 });
  const selectedEvent = createBeforeInput();
  assert.equal(
    handleMentionBoundaryBeforeInput(selectedView, selectedEvent, ["Reinhold"]),
    false,
  );

  const noSpaceView = createView("@Reinhold");
  const noSpaceEvent = createBeforeInput();
  assert.equal(
    handleMentionBoundaryBeforeInput(noSpaceView, noSpaceEvent, ["Reinhold"]),
    false,
  );
});

test("does not intercept an unhighlighted mention", () => {
  const view = createView("@Reinhold ");
  const event = createBeforeInput();

  assert.equal(handleMentionBoundaryBeforeInput(view, event, []), false);
  assert.equal(event.prevented, false);
  assert.equal(view.dispatchCount, 0);
});

test("does not intercept paste or replacement input", () => {
  for (const inputType of ["insertFromPaste", "insertReplacementText"]) {
    const view = createView("@Reinhold ");
    const event = createBeforeInput({ inputType });
    assert.equal(
      handleMentionBoundaryBeforeInput(view, event, ["Reinhold"]),
      false,
    );
    assert.equal(event.prevented, false);
    assert.equal(view.dispatchCount, 0);
  }
});

test("does not intercept composition or IME input", () => {
  const composingEventView = createView("@Reinhold ");
  const composingEvent = createBeforeInput({ isComposing: true });
  assert.equal(
    handleMentionBoundaryBeforeInput(composingEventView, composingEvent, [
      "Reinhold",
    ]),
    false,
  );

  const composingView = createView("@Reinhold ", { composing: true });
  const commitEvent = createBeforeInput();
  assert.equal(
    handleMentionBoundaryBeforeInput(composingView, commitEvent, ["Reinhold"]),
    false,
  );
});
