const VISIBLE_OUTPUT_METHODS = new Set(["append", "appendLine", "createOutputChannel"]);

/**
 * Check the extension's visible message boundary against the generated
 * projection. The small lexer keeps comments and string contents out of the
 * source inspection, while still retaining string tokens for call arguments.
 */
export function assertSourceMessageOwnership(sourceFiles, ownedIds, projectedMessages) {
  const expected = new Set(ownedIds);
  const used = new Map();
  let callCount = 0;

  for (const [file, source] of sourceFiles) {
    const tokens = lex(source);
    for (let index = 0; index < tokens.length; index += 1) {
      const token = tokens[index];
      if (token.type === "identifier" && token.value === "clientMessage" &&
          tokens[index + 1]?.value === "(") {
        const call = parseCall(tokens, index);
        assert(call, `clientMessage must be called with source-owned arguments (${file})`);
        callCount += 1;
        const id = call.args[1]?.type === "string" ? call.args[1].value : undefined;
        assert(id, `clientMessage must use a literal canonical message ID (${file})`);
        const sites = used.get(id) ?? [];
        sites.push(file);
        used.set(id, sites);
      }
      if (token.type === "identifier" && VISIBLE_OUTPUT_METHODS.has(token.value) &&
          tokens[index - 1]?.value === ".") {
        const call = parseCall(tokens, index);
        if (call?.args[0]?.type === "string" || call?.args[0]?.type === "template") {
          throw new Error(`visible host output ${token.value} must use clientMessage (${file})`);
        }
      }
    }
  }

  assert(callCount === [...used.values()].reduce((count, files) => count + files.length, 0),
    "every clientMessage call must use a literal canonical message ID");
  assert(used.size === expected.size && [...expected].every((id) => used.has(id)),
    `extension source message use must cover exactly the owned IDs; missing ${missing(expected, used)}`);
  for (const [id, files] of used) {
    assert(Object.hasOwn(projectedMessages, id),
      `extension source references a message without a generated projection: ${id} (${files.join(", ")})`);
  }
}

function lex(source) {
  const tokens = [];
  for (let index = 0; index < source.length;) {
    const character = source[index];
    if (/\s/.test(character)) {
      index += 1;
    } else if (character === "/" && source[index + 1] === "/") {
      index = source.indexOf("\n", index + 2);
      if (index < 0) break;
    } else if (character === "/" && source[index + 1] === "*") {
      const end = source.indexOf("*/", index + 2);
      index = end < 0 ? source.length : end + 2;
    } else if (character === "'" || character === '"') {
      const result = readQuoted(source, index, character);
      tokens.push({ type: "string", value: result.value });
      index = result.end;
    } else if (character === "`") {
      const result = readTemplate(source, index);
      tokens.push({ type: "template", value: result.value });
      index = result.end;
    } else if (/[A-Za-z_$]/.test(character)) {
      const start = index;
      index += 1;
      while (index < source.length && /[A-Za-z0-9_$]/.test(source[index])) index += 1;
      tokens.push({ type: "identifier", value: source.slice(start, index) });
    } else {
      tokens.push({ type: "punctuation", value: character });
      index += 1;
    }
  }
  return tokens;
}

function readQuoted(source, start, quote) {
  let value = "";
  for (let index = start + 1; index < source.length; index += 1) {
    if (source[index] === "\\") {
      value += source[index + 1] ?? "";
      index += 1;
    } else if (source[index] === quote) {
      return { value, end: index + 1 };
    } else {
      value += source[index];
    }
  }
  return { value, end: source.length };
}

function readTemplate(source, start) {
  let value = "";
  for (let index = start + 1; index < source.length; index += 1) {
    if (source[index] === "\\") {
      value += source[index] + (source[index + 1] ?? "");
      index += 1;
    } else if (source[index] === "`") {
      return { value, end: index + 1 };
    } else {
      value += source[index];
    }
  }
  return { value, end: source.length };
}

function parseCall(tokens, index) {
  if (tokens[index + 1]?.value !== "(") return undefined;
  const args = [];
  let current = [];
  let depth = 0;
  for (let cursor = index + 1; cursor < tokens.length; cursor += 1) {
    const value = tokens[cursor].value;
    if (value === "(") {
      depth += 1;
      if (depth > 1) current.push(tokens[cursor]);
    } else if (value === ")") {
      depth -= 1;
      if (depth === 0) {
        args.push(current[0]);
        return { args };
      }
      current.push(tokens[cursor]);
    } else if (value === "," && depth === 1) {
      args.push(current[0]);
      current = [];
    } else {
      current.push(tokens[cursor]);
    }
  }
  return undefined;
}

function missing(expected, used) {
  return [...expected].filter((id) => !used.has(id)).join(", ") || "none";
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}
