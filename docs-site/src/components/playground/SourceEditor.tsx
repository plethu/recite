import { text } from "../../content/messages.ts";

export function SourceEditor({ source }: { source: string }) {
  return (
    <div>
      <label for="recite-source">{text("playground.source")}</label>
      <textarea id="recite-source" spellcheck={false} autocapitalize="off" rows={16}>
        {source}
      </textarea>
      <RunControls />
      <div class="file-pick">
        <input id="recite-file" type="file" accept=".recite,text/plain" disabled />
        <label for="recite-file">{text("playground.open")}</label>
      </div>
      <noscript>
        <p>{text("playground.noJs")}</p>
      </noscript>
    </div>
  );
}

function RunControls() {
  return (
    <div class="playground-actions">
      <button type="button" data-run hidden>
        {text("playground.run")}
      </button>
      <button type="button" data-stop hidden disabled>
        {text("playground.stop")}
      </button>
      <button type="button" data-reset hidden>
        {text("playground.reset")}
      </button>
      <a href="/examples/junction.recite" download="junction.recite">
        {text("playground.download")}
      </a>
    </div>
  );
}
