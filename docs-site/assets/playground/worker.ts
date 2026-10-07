import type { Playground } from "./generated/recite_playground.js";
import type { Output, Request, Response } from "./protocol.js";

// Only the Worker messaging API is used; no DOM or game-side callbacks.
const scope = self as unknown as Pick<Worker, "onmessage" | "postMessage">;
let host: Playground | undefined;

scope.onmessage = async ({ data }: MessageEvent<Request>) => {
  const { id, command } = data;
  let response: Response;
  try {
    let encoded: string;
    if (command.kind === "run") {
      const bindings: typeof import("./generated/recite_playground.js") = await import(
        command.moduleUrl
      );
      await bindings.default({ module_or_path: command.binaryUrl });
      host?.free();
      host = new bindings.Playground();
      scope.postMessage({ id, kind: "executing" } satisfies Response);
      encoded = host.run(command.source);
    } else {
      if (!host) throw new Error("Run a scene first.");
      scope.postMessage({ id, kind: "executing" } satisfies Response);
      switch (command.kind) {
        case "advance":
          encoded = host.advance();
          break;
        case "select":
          encoded = host.select(command.id);
          break;
        case "acknowledge":
          encoded = host.acknowledge();
          break;
      }
    }
    // This JSON is produced by the pinned Rust bridge in this same build, not
    // authored source or a remote response. Rust tests cover its projection.
    response = { id, kind: "output", output: JSON.parse(encoded) as Output };
  } catch (error) {
    response = error instanceof WebAssembly.RuntimeError
      ? { id, kind: "trap" }
      : { id, kind: "error", message: String(error) };
  }
  scope.postMessage(response);
};
