import test from "node:test";
import assert from "node:assert/strict";
import { optionalSavePath, requiredBlock, requiredFixturePath, requiredOpenPath } from "../src/command-inputs.js";

test("runtime picker cancellation is a quiet no-op", async () => {
  const ui = {
    chooseAssetPath: async () => undefined,
    chooseBlock: async () => undefined,
    chooseFixturePath: async () => undefined,
    commandInputInvalid: () => new Error("invalid")
  };
  assert.equal(await requiredOpenPath(undefined, ui), undefined);
  assert.equal(await requiredBlock(undefined, ui), undefined);
  assert.equal(await requiredFixturePath(undefined, ui), undefined);
});

test("programmatic runtime inputs still require absolute paths and nonempty block names", async () => {
  const ui = { commandInputInvalid: () => new Error("invalid") };
  await assert.rejects(requiredOpenPath("relative.asset", ui), /invalid/);
  await assert.rejects(requiredFixturePath("relative.fixture", ui), /invalid/);
  await assert.rejects(requiredBlock("", ui), /invalid/);
});

test("extract picker cancellation is distinct from explicit stdout", async () => {
  const ui = {
    chooseExtractOutputPath: async () => undefined,
    commandInputInvalid: () => new Error("invalid")
  };
  assert.equal(await optionalSavePath(undefined, ui), undefined);
  assert.equal(await optionalSavePath(null, ui), null);
});
