import { cp, mkdir, mkdtemp, readFile, rm, utimes, writeFile } from "node:fs/promises";
import { existsSync, readdirSync, statSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const packageRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const manifest = JSON.parse(await readFile(path.join(packageRoot, "package.json"), "utf8"));
const output = path.join(packageRoot, `${manifest.name}-${manifest.version}.vsix`);
const stage = await mkdtemp(path.join(os.tmpdir(), "recite-vscode-package-"));
const extension = path.join(stage, "extension");

try {
  if (!existsSync(path.join(packageRoot, "dist", "extension.js"))) {
    runNode(path.join(packageRoot, "scripts", "build.mjs"));
  }
  await mkdir(extension, { recursive: true });
  for (const file of ["package.json", "package.nls.json", "language-configuration.json", "README.md"]) {
    await cp(path.join(packageRoot, file), path.join(extension, file));
  }
  await cp(path.join(packageRoot, "dist"), path.join(extension, "dist"), { recursive: true });
  await cp(path.resolve(packageRoot, "..", "..", "LICENSE"), path.join(extension, "LICENSE"));
  await writeText(path.join(stage, "[Content_Types].xml"), contentTypes());
  await writeText(path.join(stage, "extension.vsixmanifest"), vsixManifest(manifest));
  await setStableMtimes(stage);

  await writeZip(output, stage);
  if (process.argv.includes("--check")) await checkArchive(output, stage);
  console.log(`${process.argv.includes("--check") ? "checked" : "created"} ${output}`);
} finally {
  await rm(stage, { recursive: true, force: true });
}

function files(directory) {
  return readdirSync(directory).flatMap((entry) => {
    const full = path.join(directory, entry);
    const relative = path.relative(directory, full);
    return statSync(full).isDirectory()
      ? files(full).map((child) => path.join(relative, child))
      : [relative];
  }).sort();
}

async function writeText(file, text) {
  await writeFile(file, text, "utf8");
}

async function setStableMtimes(directory) {
  const fixed = new Date("1980-01-01T00:00:00.000Z");
  for (const relative of files(directory)) {
    await utimes(path.join(directory, relative), fixed, fixed);
  }
}

function contentTypes() {
  return `<?xml version="1.0" encoding="utf-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="json" ContentType="application/json" />
  <Default Extension="js" ContentType="application/javascript" />
  <Default Extension="md" ContentType="text/markdown" />
  <Default Extension="xml" ContentType="text/xml" />
  <Default Extension="vsixmanifest" ContentType="text/xml" />
</Types>
`;
}

function vsixManifest(packageManifest) {
  const displayName = xml(packageManifest.displayName);
  const description = xml(packageManifest.description);
  return `<?xml version="1.0" encoding="utf-8"?>
<PackageManifest Version="1.0.0" xmlns="http://schemas.microsoft.com/developer/vsx-schema/2011">
  <Metadata>
    <Identity Language="en-US" Id="${xml(packageManifest.name)}" Version="${xml(packageManifest.version)}" Publisher="${xml(packageManifest.publisher)}" />
    <DisplayName>${displayName}</DisplayName>
    <Description xml:space="preserve">${description}</Description>
    <MoreInfo>${xml(packageManifest.homepage)}</MoreInfo>
    <GalleryFlags>Public</GalleryFlags>
  </Metadata>
  <Installation InstalledByMsi="false">
    <InstallationTarget Id="Microsoft.VisualStudio.Code" Version="[1.89,2.0)" />
  </Installation>
  <Dependencies>
    <Dependency Id="Microsoft.VisualStudio.Code" DisplayName="Visual Studio Code" Version="[1.89,2.0)" />
  </Dependencies>
  <Assets>
    <Asset Type="Microsoft.VisualStudio.Code.Manifest" Path="extension/package.json" />
  </Assets>
</PackageManifest>
`;
}

function xml(value) {
  return String(value).replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;").replaceAll("'", "&apos;");
}

function runNode(script) {
  const result = spawnSync(process.execPath, [script], { cwd: packageRoot, stdio: "inherit" });
  if (result.status !== 0) process.exit(result.status ?? 1);
}

async function writeZip(archive, stageRoot) {
  const { deflateRawSync } = await import("node:zlib");
  const chunks = [];
  const central = [];
  let offset = 0;
  for (const name of files(stageRoot)) {
    const data = await readFile(path.join(stageRoot, name));
    const compressed = deflateRawSync(data, { level: 9 });
    const filename = Buffer.from(name, "utf8");
    const crc = crc32(data);
    const local = Buffer.alloc(30 + filename.length);
    local.writeUInt32LE(0x04034b50, 0);
    local.writeUInt16LE(20, 4);
    local.writeUInt16LE(0, 6);
    local.writeUInt16LE(8, 8);
    local.writeUInt16LE(0, 10);
    local.writeUInt16LE(0, 12);
    local.writeUInt32LE(crc, 14);
    local.writeUInt32LE(compressed.length, 18);
    local.writeUInt32LE(data.length, 22);
    local.writeUInt16LE(filename.length, 26);
    local.writeUInt16LE(0, 28);
    filename.copy(local, 30);
    chunks.push(local, compressed);
    const directory = Buffer.alloc(46 + filename.length);
    directory.writeUInt32LE(0x02014b50, 0);
    directory.writeUInt16LE(20, 4);
    directory.writeUInt16LE(20, 6);
    directory.writeUInt16LE(0, 8);
    directory.writeUInt16LE(8, 10);
    directory.writeUInt16LE(0, 12);
    directory.writeUInt16LE(0, 14);
    directory.writeUInt32LE(crc, 16);
    directory.writeUInt32LE(compressed.length, 20);
    directory.writeUInt32LE(data.length, 24);
    directory.writeUInt16LE(filename.length, 28);
    directory.writeUInt16LE(0, 30);
    directory.writeUInt16LE(0, 32);
    directory.writeUInt16LE(0, 34);
    directory.writeUInt16LE(0, 36);
    directory.writeUInt32LE(0, 38);
    directory.writeUInt32LE(offset, 42);
    filename.copy(directory, 46);
    central.push(directory);
    offset += local.length + compressed.length;
  }
  const centralData = Buffer.concat(central);
  const end = Buffer.alloc(22);
  end.writeUInt32LE(0x06054b50, 0);
  end.writeUInt16LE(0, 4);
  end.writeUInt16LE(0, 6);
  end.writeUInt16LE(central.length, 8);
  end.writeUInt16LE(central.length, 10);
  end.writeUInt32LE(centralData.length, 12);
  end.writeUInt32LE(offset, 16);
  end.writeUInt16LE(0, 20);
  await writeFile(archive, Buffer.concat([...chunks, centralData, end]));
}

function crc32(data) {
  let crc = 0xffffffff;
  for (const byte of data) {
    crc ^= byte;
    for (let bit = 0; bit < 8; bit++) crc = (crc >>> 1) ^ (crc & 1 ? 0xedb88320 : 0);
  }
  return (crc ^ 0xffffffff) >>> 0;
}

async function checkArchive(archive, stageRoot) {
  const archiveData = await readFile(archive);
  const end = archiveData.lastIndexOf(Buffer.from([0x50, 0x4b, 0x05, 0x06]));
  if (end < 0) throw new Error("VSIX has no ZIP end-of-directory record");
  const count = archiveData.readUInt16LE(end + 10);
  const centralOffset = archiveData.readUInt32LE(end + 16);
  const entries = new Set();
  let cursor = centralOffset;
  for (let index = 0; index < count; index++) {
    if (archiveData.readUInt32LE(cursor) !== 0x02014b50) throw new Error("VSIX central directory is malformed");
    const filenameLength = archiveData.readUInt16LE(cursor + 28);
    const extraLength = archiveData.readUInt16LE(cursor + 30);
    const commentLength = archiveData.readUInt16LE(cursor + 32);
    entries.add(archiveData.subarray(cursor + 46, cursor + 46 + filenameLength).toString("utf8"));
    cursor += 46 + filenameLength + extraLength + commentLength;
  }
  for (const expected of ["[Content_Types].xml", "extension.vsixmanifest", "extension/package.json", "extension/dist/extension.js"]) {
    if (!entries.has(expected)) throw new Error(`VSIX is missing ${expected}`);
  }
  if (entries.size !== files(stageRoot).length) throw new Error("VSIX file list is not deterministic");
}
