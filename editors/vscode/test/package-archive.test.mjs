import test from "node:test";
import assert from "node:assert/strict";
import { rm } from "node:fs/promises";
import { readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const packageRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const packageScript = path.join(packageRoot, "scripts", "package.mjs");
const archive = path.join(packageRoot, "recite-vscode-0.1.0.vsix");
const DOS_EPOCH_TIME = 0;
const DOS_EPOCH_DATE = 0x21;

test("VSIX archives are deterministic, readable, and use a legal DOS epoch", async () => {
  try {
    const first = runPackage();
    const second = runPackage();
    assert.deepEqual(second, first, "repeated packaging changed the VSIX bytes");

    const archiveData = readFileSync(archive);
    assertZipTimestamps(archiveData);

    const unzip = spawnSync("unzip", ["-t", archive], { encoding: "utf8" });
    assert.equal(unzip.status, 0,
      `independent ZIP validation failed:\n${unzip.stdout}\n${unzip.stderr}`);
  } finally {
    await rm(archive, { force: true });
  }
});

function runPackage() {
  const result = spawnSync(process.execPath, [packageScript], {
    cwd: packageRoot,
    encoding: "utf8",
  });
  assert.equal(result.status, 0,
    `VSIX packaging failed:\n${result.stdout}\n${result.stderr}`);
  return readArchive();
}

function readArchive() {
  return readFileSync(archive);
}

function assertZipTimestamps(data) {
  const end = data.lastIndexOf(Buffer.from([0x50, 0x4b, 0x05, 0x06]));
  assert.ok(end >= 0, "VSIX has no ZIP end-of-directory record");
  const count = data.readUInt16LE(end + 10);
  const centralOffset = data.readUInt32LE(end + 16);
  let cursor = centralOffset;

  for (let index = 0; index < count; index++) {
    assert.equal(data.readUInt32LE(cursor), 0x02014b50,
      `central directory entry ${index} is malformed`);
    assert.equal(data.readUInt16LE(cursor + 12), DOS_EPOCH_TIME,
      `central directory entry ${index} has a non-epoch DOS time`);
    assert.equal(data.readUInt16LE(cursor + 14), DOS_EPOCH_DATE,
      `central directory entry ${index} has an illegal DOS date`);

    const filenameLength = data.readUInt16LE(cursor + 28);
    const extraLength = data.readUInt16LE(cursor + 30);
    const commentLength = data.readUInt16LE(cursor + 32);
    const localOffset = data.readUInt32LE(cursor + 42);
    assert.equal(data.readUInt32LE(localOffset), 0x04034b50,
      `local entry ${index} is malformed`);
    assert.equal(data.readUInt16LE(localOffset + 10), DOS_EPOCH_TIME,
      `local entry ${index} has a non-epoch DOS time`);
    assert.equal(data.readUInt16LE(localOffset + 12), DOS_EPOCH_DATE,
      `local entry ${index} has an illegal DOS date`);
    cursor += 46 + filenameLength + extraLength + commentLength;
  }

  assert.equal(count > 0, true, "VSIX archive contains no entries");
  assert.equal(cursor, end, "central directory does not end at the ZIP footer");
}
