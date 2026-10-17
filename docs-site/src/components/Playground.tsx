import english from "../content/i18n/en.json";
import { messageNames } from "../client/messages.ts";
import type { Site } from "./Layout.tsx";
const t = (name: keyof typeof english) => english[name];
const messages = Object.fromEntries(messageNames.map((name) => [name, t(`playground.${name}`)]));
export function Playground({ site }: { site: Site }) {
  return (
    <div
      class="playground"
      data-playground
      data-worker-url={site.assets.worker}
      data-module-url={site.assets.module}
      data-binary-url={site.assets.binary}
      data-pagefind-ignore
    >
      <p>{t("playground.description")}</p>
      <div class="playground-columns">
        <div>
          <label for="recite-source">{t("playground.source")}</label>
          <textarea id="recite-source" spellcheck={false} autocapitalize="off" rows={16}>
            {site.source}
          </textarea>
          <div class="playground-actions">
            <button type="button" data-run hidden>
              {t("playground.run")}
            </button>
            <button type="button" data-stop hidden disabled>
              {t("playground.stop")}
            </button>
            <button type="button" data-reset hidden>
              {t("playground.reset")}
            </button>
            <a href="/examples/junction.recite" download="junction.recite">
              {t("playground.download")}
            </a>
          </div>
          <div class="file-pick">
            <input id="recite-file" type="file" accept=".recite,text/plain" disabled />
            <label for="recite-file">{t("playground.open")}</label>
          </div>
          <noscript>
            <p>{t("playground.noJs")}</p>
          </noscript>
        </div>
        <div class="scene-script playground-output">
          <h3>{t("playground.output")}</h3>
          <p data-status role="status" tabindex={-1}>
            {t("playground.ready")}
          </p>
          <ol
            data-transcript
            tabindex={0}
            aria-live="polite"
            aria-relevant="additions"
            aria-label={t("playground.transcript")}
          ></ol>
          <div class="playground-actions" data-controls></div>
        </div>
      </div>
      <script
        type="application/json"
        data-messages
        dangerouslySetInnerHTML={{ __html: JSON.stringify(messages).replaceAll("<", "\\u003c") }}
      />
    </div>
  );
}
