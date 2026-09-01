const MESSAGE_CALL = /\bclientMessage\s*\(\s*[^,\n]+,\s*(["'])([a-z0-9-]+)\1/g;
const MESSAGE_CALL_SITE = /\bclientMessage\s*\(/g;

/**
 * Check the extension's visible message boundary against the generated
 * projection. Source owns the use-sites; Fluent owns the text.
 */
export function assertSourceMessageOwnership(sourceFiles, ownedIds, projectedMessages) {
  const expected = new Set(ownedIds);
  const used = new Map();
  let callCount = 0;

  for (const [file, source] of sourceFiles) {
    callCount += [...source.matchAll(MESSAGE_CALL_SITE)].length;
    for (const match of source.matchAll(MESSAGE_CALL)) {
      const id = match[2];
      const sites = used.get(id) ?? [];
      sites.push(file);
      used.set(id, sites);
    }
  }

  assert(used.size === expected.size && [...expected].every((id) => used.has(id)),
    `extension source message use must cover exactly the owned IDs; missing ${missing(expected, used)}`);
  assert(callCount === [...used.values()].reduce((count, files) => count + files.length, 0),
    "every clientMessage call must use a literal canonical message ID");
  for (const [id, files] of used) {
    assert(Object.hasOwn(projectedMessages, id),
      `extension source references a message without a generated projection: ${id} (${files.join(", ")})`);
  }
}

function missing(expected, used) {
  return [...expected].filter((id) => !used.has(id)).join(", ") || "none";
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}
