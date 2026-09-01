const HEADER_SEPARATOR = Buffer.from("\r\n\r\n", "ascii");
const MAX_MESSAGE_BYTES = 16 * 1024 * 1024;

export class LspProtocolError extends Error {
  constructor(message) {
    super(message);
    this.name = "LspProtocolError";
  }
}

export function encodeMessage(message) {
  const body = Buffer.from(JSON.stringify(message), "utf8");
  return Buffer.concat([
    Buffer.from(`Content-Length: ${body.byteLength}\r\n\r\n`, "ascii"),
    body
  ]);
}

/**
 * Incremental parser for the LSP's Content-Length framed JSON messages.
 * Keeping this separate from process management makes malformed transport
 * input testable without a VS Code host.
 */
export class LspFrameParser {
  constructor(onMessage) {
    this.onMessage = onMessage;
    this.buffer = Buffer.alloc(0);
    this.contentLength = undefined;
  }

  push(chunk) {
    if (!Buffer.isBuffer(chunk)) {
      throw new TypeError("LSP input must be a Buffer");
    }
    this.buffer = Buffer.concat([this.buffer, chunk]);
    if (this.buffer.byteLength > MAX_MESSAGE_BYTES + 4096) {
      throw new LspProtocolError("LSP message exceeds the 16 MiB limit");
    }

    while (true) {
      if (this.contentLength === undefined) {
        const separator = this.buffer.indexOf(HEADER_SEPARATOR);
        if (separator < 0) return;
        const headers = this.buffer.subarray(0, separator).toString("ascii");
        this.contentLength = parseContentLength(headers);
        this.buffer = this.buffer.subarray(separator + HEADER_SEPARATOR.length);
      }
      if (this.buffer.byteLength < this.contentLength) return;

      const body = this.buffer.subarray(0, this.contentLength).toString("utf8");
      this.buffer = this.buffer.subarray(this.contentLength);
      this.contentLength = undefined;
      let message;
      try {
        message = JSON.parse(body);
      } catch (error) {
        throw new LspProtocolError(`invalid LSP JSON: ${error.message}`);
      }
      this.onMessage(message);
    }
  }
}

function parseContentLength(headers) {
  const values = headers
    .split("\r\n")
    .filter((header) => header.toLowerCase().startsWith("content-length:"))
    .map((header) => header.slice(header.indexOf(":") + 1).trim());
  if (values.length !== 1 || !/^\d+$/.test(values[0])) {
    throw new LspProtocolError("LSP message has no unique Content-Length header");
  }
  const length = Number(values[0]);
  if (!Number.isSafeInteger(length) || length > MAX_MESSAGE_BYTES) {
    throw new LspProtocolError("LSP message exceeds the 16 MiB limit");
  }
  return length;
}
