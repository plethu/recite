import { text } from "../content/messages.ts";
import { messageNames } from "../client/messages.ts";
import { SourceEditor } from "./playground/SourceEditor.tsx";
import { ExecutionOutput } from "./playground/ExecutionOutput.tsx";

export interface PlaygroundProps {
  source: string;
  workerUrl: string;
  moduleUrl: string;
  binaryUrl: string;
}
const messages = Object.fromEntries(messageNames.map((name) => [name, text(`playground.${name}`)]));
const serializedMessages = JSON.stringify(messages).replaceAll("<", "\\u003c");

export function Playground({ source, workerUrl, moduleUrl, binaryUrl }: PlaygroundProps) {
  return (
    <div
      class="playground"
      data-playground
      data-worker-url={workerUrl}
      data-module-url={moduleUrl}
      data-binary-url={binaryUrl}
      data-pagefind-ignore
    >
      <p>{text("playground.description")}</p>
      <div class="playground-columns">
        <SourceEditor source={source} />
        <ExecutionOutput />
      </div>
      <script
        type="application/json"
        data-messages
        dangerouslySetInnerHTML={{ __html: serializedMessages }}
      />
    </div>
  );
}
