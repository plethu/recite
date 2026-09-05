const MAX_RECORD_BYTES = 4 * 1024 * 1024;
const PROTOCOL_VERSION = 1;
const LOSSLESS_INTEGER_MARKER = "\u0000recite-lossless-integer:";

/**
 * JSON has no integer type and JavaScript rounds values beyond 2^53 - 1.
 * Keep such wire integers as a small, JSON-safe value object.  The adapter
 * validates their range at each typed field before exposing them to a host.
 */
export class LosslessInteger {
  constructor(raw) {
    if (!/^-?(?:0|[1-9][0-9]*)$/u.test(raw)) throw new TypeError("invalid lossless integer");
    this.raw = raw;
    Object.freeze(this);
  }

  toString() { return this.raw; }
  toJSON() { return this.raw; }
}

export function isLosslessInteger(value) {
  return value instanceof LosslessInteger;
}

export function isIntegerValue(value) {
  return Number.isSafeInteger(value) || isLosslessInteger(value);
}

export function integerInRange(value, minimum, maximum) {
  const raw = isLosslessInteger(value) ? value.raw : Number.isSafeInteger(value) ? String(value) : undefined;
  if (raw === undefined || !/^-?(?:0|[1-9][0-9]*)$/u.test(raw)) return false;
  const candidate = BigInt(raw);
  return candidate >= BigInt(minimum) && candidate <= BigInt(maximum);
}

export function compareIntegers(left, right) {
  if (!isIntegerValue(left) || !isIntegerValue(right)) return undefined;
  const a = BigInt(isLosslessInteger(left) ? left.raw : left);
  const b = BigInt(isLosslessInteger(right) ? right.raw : right);
  return a < b ? -1 : a > b ? 1 : 0;
}

export class CommandProtocolError extends Error {
  constructor(code, detail) {
    super(detail ? `${code}: ${detail}` : code);
    this.name = "ReciteCommandProtocolError";
    this.code = code;
  }
}

/** A bounded newline-delimited JSON decoder shared by finite and watch hosts. */
export class NdjsonRecordParser {
  constructor({ maxBytes = MAX_RECORD_BYTES } = {}) {
    this.maxBytes = maxBytes;
    this.buffer = Buffer.alloc(0);
    this.bytes = 0;
    this.finished = false;
  }

  push(chunk) {
    if (this.finished) throw protocol("records_after_end");
    const bytes = Buffer.from(chunk);
    this.buffer = Buffer.concat([this.buffer, bytes]);
    this.bytes += bytes.byteLength;
    if (this.buffer.byteLength > this.maxBytes) {
      throw protocol("record_too_large");
    }
    const records = [];
    let newline;
    while ((newline = this.buffer.indexOf(0x0a)) >= 0) {
      const line = decodeUtf8(this.buffer.subarray(0, newline));
      this.buffer = this.buffer.slice(newline + 1);
      if (line.endsWith("\r")) throw protocol("carriage_return_record");
      records.push(parseRecord(line));
    }
    return records;
  }

  finish() {
    if (this.finished) return [];
    this.finished = true;
    if (this.buffer.byteLength !== 0) throw protocol("truncated_record");
    return [];
  }
}

export function protocol(code, detail) {
  return new CommandProtocolError(code, detail);
}

function parseRecord(line) {
  if (line.length === 0) throw protocol("empty_record");
  if (Buffer.byteLength(line, "utf8") > MAX_RECORD_BYTES) throw protocol("record_too_large");
  let record;
  try {
    record = parseLosslessJson(line);
  } catch (error) {
    throw protocol("invalid_json", error.message);
  }
  if (!record || typeof record !== "object" || Array.isArray(record)) {
    throw protocol("record_not_object");
  }
  return record;
}

/**
 * Parse JSON while preserving integer tokens outside JavaScript's exact
 * Number range.  Only integer tokens are rewritten; JSON syntax, strings,
 * fractions, and exponents remain the native JSON parser's responsibility.
 */
export function parseLosslessJson(line) {
  const marker = losslessMarker(line);
  let transformed = "";
  let index = 0;
  while (index < line.length) {
    const character = line[index];
    if (character === '"') {
      const start = index++;
      let escaped = false;
      while (index < line.length) {
        const current = line[index++];
        if (escaped) {
          escaped = false;
        } else if (current === "\\") {
          escaped = true;
        } else if (current === '"') {
          break;
        }
      }
      transformed += line.slice(start, index);
      continue;
    }
    if (character === "-" || /[0-9]/u.test(character)) {
      const match = line.slice(index).match(/^-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?/u);
      if (match) {
        const token = match[0];
        if (!/[.eE]/u.test(token) && !rawIntegerInRange(token, "-9007199254740991", "9007199254740991")) {
          transformed += JSON.stringify(`${marker}${token}`);
        } else {
          transformed += token;
        }
        index += token.length;
        continue;
      }
    }
    transformed += character;
    index += 1;
  }
  return JSON.parse(transformed, (_key, value) => {
    if (typeof value === "string" && value.startsWith(marker)) {
      const raw = value.slice(marker.length);
      if (/^-?(?:0|[1-9][0-9]*)$/u.test(raw)) return new LosslessInteger(raw);
    }
    return value;
  });
}

function rawIntegerInRange(value, minimum, maximum) {
  if (typeof value !== "string" || !/^-?(?:0|[1-9][0-9]*)$/u.test(value)) return false;
  const candidate = BigInt(value);
  return candidate >= BigInt(minimum) && candidate <= BigInt(maximum);
}

function losslessMarker(line) {
  const strings = [];
  try {
    collectStrings(JSON.parse(line), strings);
  } catch {
    // The transformed parse below will report the actual JSON syntax error.
  }
  let marker = LOSSLESS_INTEGER_MARKER;
  while (strings.some((value) => value.startsWith(marker))) marker += "x";
  return marker;
}

function collectStrings(value, strings) {
  if (typeof value === "string") {
    strings.push(value);
  } else if (Array.isArray(value)) {
    value.forEach((item) => collectStrings(item, strings));
  } else if (value && typeof value === "object") {
    Object.values(value).forEach((item) => collectStrings(item, strings));
  }
}

function decodeUtf8(bytes) {
  try {
    return new TextDecoder("utf-8", { fatal: true }).decode(bytes);
  } catch (error) {
    throw protocol("invalid_utf8", error.message);
  }
}

export function validateEnvelope(record, command, invocationId, sequence) {
  if (record.version !== PROTOCOL_VERSION || record.command !== command ||
      record.sequence !== sequence || typeof record.event !== "string") {
    throw protocol("invalid_envelope");
  }
  const hasInvocationId = Object.hasOwn(record, "invocation_id");
  if (invocationId === undefined) {
    if (hasInvocationId) throw protocol("unexpected_invocation_id");
  } else if (!hasInvocationId || record.invocation_id !== invocationId) {
    throw protocol("invocation_mismatch");
  }
}

/** Check the exact envelope shape, omitting optional invocation metadata when
 * the caller did not provide an invocation ID. */
export function exactEnvelopeKeys(value, expected, invocationId) {
  const keys = invocationId === undefined
    ? expected.filter((key) => key !== "invocation_id")
    : expected;
  return value && typeof value === "object" && JSON.stringify(Object.keys(value).sort()) ===
    JSON.stringify(keys.slice().sort());
}
