import { text } from "../../content/messages.ts";

export function ExecutionOutput() {
  return (
    <div class="scene-script playground-output">
      <h3>{text("playground.output")}</h3>
      <p data-status role="status" tabindex={-1}>
        {text("playground.ready")}
      </p>
      <ol
        data-transcript
        tabindex={0}
        aria-live="polite"
        aria-relevant="additions"
        aria-label={text("playground.transcript")}
      ></ol>
      <div class="playground-actions" data-controls></div>
    </div>
  );
}
