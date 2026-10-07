import type { Messages } from "./messages.js";
import type { Command, Effect, Line, Output } from "./protocol.js";

interface View {
  transcript: HTMLOListElement;
  controls: HTMLElement;
  status: HTMLElement;
  messages: Messages;
  send: (command: Command) => void;
}

export function renderOutput(output: Output, view: View) {
  const { messages, status } = view;
  const append = (text: string) => {
    const item = document.createElement("li");
    item.textContent = text;
    view.transcript.append(item);
    // A looping dialogue can emit indefinitely; this is a view, not a save log.
    if (view.transcript.children.length > 100) view.transcript.firstElementChild?.remove();
    view.transcript.scrollTop = view.transcript.scrollHeight;
  };
  const line = (value: Line) =>
    append(value.speaker ? `${value.speaker}: ${value.text}` : value.text);
  const effect = (value: Effect) =>
    append(
      `${messages.effect} ${value.mode} ${value.function}(${
        value.args.map(arg => arg.type === "string" ? JSON.stringify(arg.value) : String(arg.value))
          .join(", ")
      })`,
    );
  const button = (label: string, command: Command, disabled = false) => {
    const control = document.createElement("button");
    control.type = "button";
    control.textContent = label;
    control.disabled = disabled;
    control.addEventListener("click", () => view.send(command));
    view.controls.append(control);
  };
  switch (output.kind) {
    case "line":
      line(output.line);
      button(messages.next, { kind: "advance" });
      status.textContent = messages.playing;
      break;
    case "prompt":
      if (output.line) line(output.line);
      for (const choice of output.choices) {
        button(choice.reason ? `${choice.text} (${choice.reason})` : choice.text, {
          kind: "select",
          id: choice.id,
        }, !choice.available);
      }
      status.textContent = messages.choose;
      break;
    case "effect":
      effect(output.effect);
      button(output.effect.mode === "blocking" ? messages.acknowledge : messages.next, {
        kind: output.effect.mode === "blocking" ? "acknowledge" : "advance",
      });
      status.textContent = messages.effect;
      break;
    case "end":
      output.deferred_effects.forEach(effect);
      status.textContent = messages.ended;
      break;
    case "diagnostics":
      for (const diagnostic of output.diagnostics) {
        append(
          `${diagnostic.code} · ${diagnostic.span.start.line}:${diagnostic.span.start.column} · ${
            diagnostic.compatibility_message ?? diagnostic.code
          }`,
        );
      }
      status.textContent = messages.invalid;
      break;
  }
  const next = view.controls.querySelector<HTMLButtonElement>("button:not(:disabled)");
  if (next) next.focus({ preventScroll: true });
  else status.focus({ preventScroll: true });
}
