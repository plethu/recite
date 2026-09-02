import { encodeMessage } from "./lsp-protocol.js";

/** Keep framed stdio writes and one-shot child teardown at the transport edge. */
export function writeFramedMessage(stdin, message, onFailure) {
  if (!stdin?.writable) return false;
  try {
    stdin.write(encodeMessage(message), (error) => {
      if (error) onFailure(error);
    });
    return true;
  } catch (error) {
    onFailure(error);
    return false;
  }
}

export function closeTransport(child, stdin, exited) {
  stdin?.destroy?.();
  if (!exited && child && !child.killed) {
    try { child.kill(); } catch { /* already gone */ }
  }
}
