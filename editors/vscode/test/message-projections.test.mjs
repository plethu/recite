import test from "node:test";
import assert from "node:assert/strict";
import {
  cp,
  mkdtemp,
  mkdir,
  readFile,
  readdir,
  rename,
  rm,
  symlink,
  writeFile
} from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import os from "node:os";
import {
  generateMessageProjections,
  projectMessages,
  verifyMessageProjections
} from "../scripts/message-projections.mjs";

const packageRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

test("message projections reject multiline and selector Fluent before generation", async () => {
  const sourcePath = path.resolve(packageRoot, "../../crates/recite-ui/resources/en-US.ftl");
  const source = await readFile(sourcePath, "utf8");
  const original = "lsp-client-start-failed = Recite language server could not be started: {$detail}.";
  assert.throws(
    () => projectMessages(source.replace(original, `${original}\n  continuation`)),
    /continuation/
  );
  assert.throws(
    () => projectMessages(source.replace(original, "lsp-client-start-failed = { $kind -> [one] one *[other] other }")),
    /unsupported expression/
  );
});

test("message verification rejects a mutation without rewriting its projection", async () => {
  const { root, fixturePackage, fixtureSource } = await createProjectionFixture();
  try {
    const projection = path.join(fixtureSource, "messages.generated.js");
    const mutated = `${await readFile(projection, "utf8")}\n// mutation fixture\n`;
    await writeFile(projection, mutated, "utf8");
    await assert.rejects(
      verifyMessageProjections(fixturePackage),
      /message projection is stale/
    );
    assert.equal(await readFile(projection, "utf8"), mutated);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("message projection updates refuse symlinked destinations before writing", async () => {
  const { root, fixturePackage, fixtureSource } = await createProjectionFixture();
  const packageProjection = path.join(fixturePackage, "package.nls.json");
  const runtimeProjection = path.join(fixtureSource, "messages.generated.js");
  const outside = path.join(root, "outside.json");
  const originalRuntime = await readFile(runtimeProjection, "utf8");
  try {
    await writeFile(outside, "outside\n", "utf8");
    await rm(packageProjection);
    await symlink(outside, packageProjection);

    await assert.rejects(
      generateMessageProjections(fixturePackage),
      /refusing symlink/
    );
    assert.equal(await readFile(outside, "utf8"), "outside\n");
    assert.equal(await readFile(runtimeProjection, "utf8"), originalRuntime);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("message projection updates refuse symlinked projection parents before writing", async () => {
  const { root, fixturePackage, fixtureSource } = await createProjectionFixture();
  const outsideSource = path.join(root, "outside-src");
  const outsideProjection = path.join(outsideSource, "messages.generated.js");
  const packageProjection = path.join(fixturePackage, "package.nls.json");
  const originalPackage = await readFile(packageProjection, "utf8");
  try {
    await mkdir(outsideSource);
    await cp(path.join(fixtureSource, "messages.generated.js"), outsideProjection);
    const outsideOriginal = await readFile(outsideProjection, "utf8");
    await rm(fixtureSource, { recursive: true });
    await symlink(outsideSource, fixtureSource);

    await assert.rejects(
      generateMessageProjections(fixturePackage),
      /refusing symlink/
    );
    assert.equal(await readFile(outsideProjection, "utf8"), outsideOriginal);
    assert.equal(await readFile(packageProjection, "utf8"), originalPackage);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("message projection updates restore missing projections", async () => {
  const { root, fixturePackage, fixtureSource } = await createProjectionFixture();
  const runtimeProjection = path.join(fixtureSource, "messages.generated.js");
  try {
    await rm(runtimeProjection);
    await generateMessageProjections(fixturePackage);
    await assert.doesNotReject(verifyMessageProjections(fixturePackage));
    assert.deepEqual(
      (await readdir(fixturePackage)).filter((entry) => entry.startsWith(".recite-message-projections-")),
      []
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("missing projections remain absent when a later install fails", async () => {
  const { root, fixturePackage, fixtureSource } = await createProjectionFixture();
  const runtimeProjection = path.join(fixtureSource, "messages.generated.js");
  const packageProjection = path.join(fixturePackage, "package.nls.json");
  const originalPackage = await readFile(packageProjection, "utf8");
  let renameCount = 0;
  try {
    await rm(runtimeProjection);
    await assert.rejects(
      generateMessageProjections(fixturePackage, {
        fileSystem: {
          rename: async (from, to) => {
            renameCount += 1;
            if (renameCount === 2) throw new Error("induced projection install failure");
            return rename(from, to);
          }
        }
      }),
      /induced projection install failure/
    );
    assert.equal(renameCount, 2, "the newly installed missing projection should be removed on rollback");
    assert.equal((await readdir(fixtureSource)).includes("messages.generated.js"), false);
    assert.equal(await readFile(packageProjection, "utf8"), originalPackage);
    assert.deepEqual(
      (await readdir(fixturePackage)).filter((entry) => entry.startsWith(".recite-message-projections-")),
      []
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("message projection updates install verified projections as a pair", async () => {
  const { root, fixturePackage } = await createProjectionFixture();
  try {
    await generateMessageProjections(fixturePackage);
    await assert.doesNotReject(verifyMessageProjections(fixturePackage));
    assert.deepEqual(
      (await readdir(fixturePackage)).filter((entry) => entry.startsWith(".recite-message-projections-")),
      []
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("message projection updates roll back a partial commit and clean staging", async () => {
  const { root, fixturePackage, fixtureSource } = await createProjectionFixture();
  const packageProjection = path.join(fixturePackage, "package.nls.json");
  const runtimeProjection = path.join(fixtureSource, "messages.generated.js");
  const originals = await Promise.all([
    readFile(runtimeProjection, "utf8"),
    readFile(packageProjection, "utf8")
  ]);
  let renameCount = 0;
  try {
    await assert.rejects(
      generateMessageProjections(fixturePackage, {
        fileSystem: {
          rename: async (from, to) => {
            renameCount += 1;
            if (renameCount === 2) throw new Error("induced projection install failure");
            return rename(from, to);
          }
        }
      }),
      /induced projection install failure/
    );
    assert.equal(renameCount, 3, "rollback should restore the first projection after the failed install");
    assert.deepEqual(await Promise.all([
      readFile(runtimeProjection, "utf8"),
      readFile(packageProjection, "utf8")
    ]), originals);
    assert.deepEqual(
      (await readdir(fixturePackage)).filter((entry) => entry.startsWith(".recite-message-projections-")),
      []
    );
    assert.deepEqual(
      (await readdir(path.join(fixturePackage, "src"))).filter((entry) => entry.includes(".backup-")),
      []
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

async function createProjectionFixture() {
  const root = await mkdtemp(path.join(os.tmpdir(), "recite-vscode-projections-"));
  const fixturePackage = path.join(root, "editors", "vscode");
  const fixtureSource = path.join(fixturePackage, "src");
  const fixtureFluent = path.join(root, "crates", "recite-ui", "resources");
  await mkdir(fixtureSource, { recursive: true });
  await mkdir(fixtureFluent, { recursive: true });
  await cp(
    path.join(packageRoot, "src", "messages.generated.js"),
    path.join(fixtureSource, "messages.generated.js")
  );
  await cp(path.join(packageRoot, "package.nls.json"), path.join(fixturePackage, "package.nls.json"));
  await cp(
    path.resolve(packageRoot, "../../crates/recite-ui/resources/en-US.ftl"),
    path.join(fixtureFluent, "en-US.ftl")
  );
  return { root, fixturePackage, fixtureSource };
}
