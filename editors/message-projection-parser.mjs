/**
 * Parse the deliberately small Fluent subset that host projections can
 * represent. A projected message is one line with plain text and simple
 * variable placeables. Fluent continuation lines, selectors, terms,
 * attributes, and other expressions fail closed rather than being truncated.
 */
export function parseRepresentableMessages(source, ids, host = "editor") {
  const wanted = new Set(ids);

  const messages = new Map();
  let currentId;
  for (const [index, line] of source.split(/\r?\n/u).entries()) {
    const message = /^(?<id>[a-z0-9-]+) = (?<value>[^\r\n]*)$/u.exec(line);
    if (message) {
      currentId = message.groups.id;
      if (!wanted.has(currentId)) continue;
      if (messages.has(currentId)) {
        throw new Error(`duplicate canonical Fluent message ${currentId} at line ${index + 1}`);
      }
      const value = message.groups.value;
      assertRepresentableValue(currentId, value, index + 1);
      messages.set(currentId, value);
      continue;
    }
    if (currentId && wanted.has(currentId) && line.trim() !== "") {
      if (/^\s/u.test(line)) {
        throw new Error(
          `canonical Fluent message ${currentId} uses a continuation at line ${index + 1}; ` +
          `${host} projections support only single-line templates`
        );
      }
      if (/^\./u.test(line.trim())) {
        throw new Error(`canonical Fluent message ${currentId} uses an unsupported attribute at line ${index + 1}`);
      }
    }
    if (line.trim() !== "") currentId = undefined;
  }
  for (const id of wanted) {
    if (!messages.has(id)) throw new Error(`canonical Fluent message is missing ${id}`);
  }
  return messages;
}

function assertRepresentableValue(id, value, line) {
  if (!/^([^{}]|\{\$[a-zA-Z][a-zA-Z0-9_-]*\})*$/u.test(value)) {
    throw new Error(`canonical Fluent message ${id} uses an unsupported expression at line ${line}`);
  }
}
