import { text } from "../content/messages.ts";
import { searchMessageNames } from "../client/messages.ts";

const messages = Object.fromEntries(
  searchMessageNames.map((name) => [name, text(`search.${name}`)]),
);
const serializedMessages = JSON.stringify(messages).replaceAll("<", "\\u003c");

// A Writer picker: a field well beside a close icon, one 32px row per result, the active row's
// excerpt in a detail area, and a hint line. search.ts renders the rows with Pagefind's API.
export function SearchDialog() {
  return (
    <dialog id="search-dialog" aria-label={text("search.dialog")} data-pagefind-ignore>
      <div class="picker-head">
        <div class="picker-field">
          <svg viewBox="0 0 16 16" aria-hidden="true" focusable="false">
            <circle cx="6.75" cy="6.75" r="4.75" />
            <path d="m10.25 10.25 4 4" />
          </svg>
          <label class="visually-hidden" for="documentation-search">
            {text("search.label")}
          </label>
          <input
            id="documentation-search"
            type="text"
            autocomplete="off"
            spellcheck={false}
            placeholder={text("search.placeholder")}
          />
        </div>
        <form method="dialog">
          <button class="picker-close" aria-label={text("search.close")}>
            <svg viewBox="0 0 16 16" aria-hidden="true" focusable="false">
              <path d="m4 4 8 8m0-8-8 8" />
            </svg>
          </button>
        </form>
      </div>
      <ul class="picker-results" data-search-results aria-label={text("search.results")}></ul>
      <p class="picker-detail" id="search-detail" data-search-detail hidden></p>
      <p class="picker-foot">
        <span data-search-status role="status"></span>
        <span>{text("search.hint")}</span>
      </p>
      <script
        type="application/json"
        data-search-messages
        dangerouslySetInnerHTML={{ __html: serializedMessages }}
      />
    </dialog>
  );
}
