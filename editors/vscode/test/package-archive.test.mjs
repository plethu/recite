import test from "node:test";
import assert from "node:assert/strict";
import { rm } from "node:fs/promises";
import { readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { unzipSync } from "fflate";

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
    assertZipArchive(archiveData);
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

function assertZipArchive(data) {
  const end = data.lastIndexOf(Buffer.from([0x50, 0x4b, 0x05, 0x06]));
  assert.ok(end >= 0, "VSIX has no ZIP end-of-directory record");
  const count = data.readUInt16LE(end + 10);
  const centralOffset = data.readUInt32LE(end + 16);
  const entries = unzipSync(data);
  const names = [];
  let cursor = centralOffset;
  let previousBodyEnd = 0;

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
    const name = data.subarray(cursor + 46, cursor + 46 + filenameLength).toString("utf8");
    const localOffset = data.readUInt32LE(cursor + 42);
    assert.equal(data.readUInt32LE(localOffset), 0x04034b50,
      `local entry ${index} is malformed`);
    assert.equal(data.readUInt16LE(localOffset + 10), DOS_EPOCH_TIME,
      `local entry ${index} has a non-epoch DOS time`);
    assert.equal(data.readUInt16LE(localOffset + 12), DOS_EPOCH_DATE,
      `local entry ${index} has an illegal DOS date`);
    const localNameLength = data.readUInt16LE(localOffset + 26);
    const localExtraLength = data.readUInt16LE(localOffset + 28);
    assert.equal(data.subarray(localOffset + 30, localOffset + 30 + localNameLength).toString("utf8"), name,
      `local entry ${index} name differs from the central directory`);
    const compressedSize = data.readUInt32LE(cursor + 20);
    const uncompressedSize = data.readUInt32LE(cursor + 24);
    const bodyStart = localOffset + 30 + localNameLength + localExtraLength;
    const bodyEnd = bodyStart + compressedSize;
    assert.ok(localOffset >= previousBodyEnd, `local entry ${index} offset overlaps a previous entry`);
    assert.ok(bodyEnd <= centralOffset, `local entry ${index} overlaps the central directory`);
    assert.equal(data.readUInt32LE(localOffset + 14), data.readUInt32LE(cursor + 16),
      `local entry ${index} CRC differs from the central directory`);
    assert.equal(data.readUInt32LE(localOffset + 18), compressedSize,
      `local entry ${index} compressed size differs from the central directory`);
    assert.equal(data.readUInt32LE(localOffset + 22), uncompressedSize,
      `local entry ${index} uncompressed size differs from the central directory`);
    const content = Buffer.from(entries[name] ?? []);
    assert.equal(content.length, uncompressedSize, `ZIP consumer returned the wrong size for ${name}`);
    assert.equal(crc32(content), data.readUInt32LE(cursor + 16), `ZIP consumer CRC failed for ${name}`);
    names.push(name);
    previousBodyEnd = bodyEnd;
    cursor += 46 + filenameLength + extraLength + commentLength;
  }

  assert.equal(count > 0, true, "VSIX archive contains no entries");
  assert.equal(cursor, end, "central directory does not end at the ZIP footer");
  assert.deepEqual(Object.keys(entries).sort(), names.slice().sort(),
    "independent ZIP consumer returned a different file list");
  for (const expected of [
    "[Content_Types].xml",
    "extension.vsixmanifest",
    "extension/package.json",
    "extension/syntaxes/recite.tmLanguage.json",
    "extension/dist/extension.cjs",
    "extension/dist/extension.js"
  ]) {
    assert.ok(entries[expected], `VSIX is missing ${expected}`);
  }
}

function crc32(data) {
  let crc = 0xffffffff;
  for (const byte of data) {
    crc ^= byte;
    for (let bit = 0; bit < 8; bit++) crc = (crc >>> 1) ^ (crc & 1 ? 0xedb88320 : 0);
  }
  return (crc ^ 0xffffffff) >>> 0;
}
