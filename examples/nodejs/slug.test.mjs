import { test } from "node:test";
import assert from "node:assert";
import { slug } from "./slug.mjs";

test("spaces become dashes", () => {
  assert.equal(slug("a b"), "a-b");
});

test("lowercased", () => {
  assert.equal(slug("AB CD"), "ab-cd");
});
