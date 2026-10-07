import type { Command, Response } from "./protocol.js";
import { renderOutput } from "./render.js";

const root = document.querySelector<HTMLElement>("[data-playground]");
if (root) setup(root);

function setup(root: HTMLElement) {
  const source = root.querySelector<HTMLTextAreaElement>("textarea");
  const run = root.querySelector<HTMLButtonElement>("[data-run]");
  const stop = root.querySelector<HTMLButtonElement>("[data-stop]");
  const reset = root.querySelector<HTMLButtonElement>("[data-reset]");
  const file = root.querySelector<HTMLInputElement>("input[type=file]");
  const status = root.querySelector<HTMLElement>("[data-status]");
  const transcript = root.querySelector<HTMLOListElement>("[data-transcript]");
  const controls = root.querySelector<HTMLElement>("[data-controls]");
  const config = root.querySelector<HTMLScriptElement>("[data-messages]");
  if (
    !source || !run || !stop || !reset || !file || !status || !transcript || !controls || !config
  ) {
    throw new Error("Incomplete playground template.");
  }
  const messages: Record<string, string> = JSON.parse(config.textContent ?? "{}");
  const original = source.value;
  const { workerUrl, moduleUrl, binaryUrl } = root.dataset;
  if (!workerUrl || !moduleUrl || !binaryUrl) throw new Error("Missing playground resources.");
  let worker: Worker | undefined;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let generation = 0;
  let busy = false;

  const invalidate = () => {
    generation += 1;
    worker?.terminate();
    worker = undefined;
    clearTimeout(timer);
    busy = false;
    run.disabled = false;
    stop.disabled = true;
    controls.replaceChildren();
  };
  const fail = (message: string) => {
    invalidate();
    status.textContent = message;
  };
  const send = (command: Command) => {
    if (busy || !worker) return;
    busy = true;
    run.disabled = true;
    stop.disabled = false;
    for (const button of controls.querySelectorAll("button")) button.disabled = true;
    status.textContent = command.kind === "run" ? messages.loadingCompiler : messages.loading;
    const current = worker;
    const id = ++generation;
    const deadline = (milliseconds: number, message: string) => {
      clearTimeout(timer);
      timer = setTimeout(() => {
        if (worker === current && generation === id) fail(message);
      }, milliseconds);
    };
    deadline(
      command.kind === "run" ? 60_000 : 15_000,
      command.kind === "run" ? messages.downloadTimeout : messages.timeout,
    );
    current.onmessage = ({ data }: MessageEvent<Response>) => {
      if (worker !== current || data.id !== generation) return;
      if (data.kind === "executing") {
        status.textContent = messages.loading;
        deadline(15_000, messages.timeout);
        return;
      }
      clearTimeout(timer);
      busy = false;
      run.disabled = false;
      stop.disabled = true;
      if (data.kind !== "output") {
        fail(data.kind === "trap" ? messages.resourceLimit : data.message);
        return;
      }
      controls.replaceChildren();
      renderOutput(data.output, { transcript, controls, status, messages, send });
    };
    current.onerror = () => {
      if (worker === current && generation === id) fail(messages.failed);
    };
    current.postMessage({ id, command });
  };
  run.hidden = false;
  stop.hidden = false;
  reset.hidden = false;
  file.disabled = false;
  run.addEventListener("click", () => {
    invalidate();
    transcript.replaceChildren();
    if (source.value.length > 65_536 || new TextEncoder().encode(source.value).length > 65_536) {
      fail(messages.tooLarge);
      return;
    }
    try {
      worker = new Worker(workerUrl, { type: "module" });
      send({ kind: "run", source: source.value, moduleUrl, binaryUrl });
    } catch {
      fail(messages.failed);
    }
  });
  source.addEventListener("input", () => {
    invalidate();
    status.textContent = messages.changed;
  });
  stop.addEventListener("click", () => {
    invalidate();
    status.textContent = messages.stopped;
  });
  reset.addEventListener("click", () => {
    invalidate();
    source.value = original;
    transcript.replaceChildren();
    status.textContent = messages.ready;
  });
  file.addEventListener("change", async () => {
    const selected = file.files?.[0];
    if (!selected) return;
    invalidate();
    const expected = generation;
    if (selected.size > 65_536) {
      fail(messages.tooLarge);
      return;
    }
    try {
      const text = await selected.text();
      if (generation !== expected) return;
      source.value = text;
      transcript.replaceChildren();
      status.textContent = messages.changed;
    } catch {
      if (generation === expected) fail(messages.failed);
    }
  });
  window.addEventListener("pagehide", invalidate);
}
