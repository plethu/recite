import type { SearchMessages } from "./messages.ts";

interface PagefindData {
  url: string;
  excerpt: string;
  meta: { title?: string; section?: string };
}
interface Pagefind {
  init(): Promise<void>;
  debouncedSearch(
    term: string,
    options?: object,
    timeout?: number,
  ): Promise<{ results: { data(): Promise<PagefindData> }[] } | null>;
}

// Rows render at most this many results; the count still reports every match.
const shownResults = 30;

export function setupSearch() {
  const trigger = document.querySelector<HTMLButtonElement>("[data-search-trigger]");
  const dialog = document.querySelector<HTMLDialogElement>("#search-dialog");
  const input = dialog?.querySelector<HTMLInputElement>("#documentation-search");
  const list = dialog?.querySelector<HTMLUListElement>("[data-search-results]");
  const detail = dialog?.querySelector<HTMLElement>("[data-search-detail]");
  const status = dialog?.querySelector<HTMLElement>("[data-search-status]");
  const source = dialog?.querySelector("[data-search-messages]")?.textContent;
  if (!trigger || !dialog || !input || !list || !detail || !status || !source) return;
  const messages = JSON.parse(source) as SearchMessages;
  const excerpts = new Map<HTMLAnchorElement, string>();
  let loading: Promise<Pagefind> | undefined;
  let pagefind: Pagefind | undefined;
  let attempts = 0;

  // Pagefind's own module; a retry gets a fresh URL so a failed fetch is not reused.
  function load() {
    const url = `/pagefind/pagefind.js${attempts > 0 ? `?retry=${attempts}` : ""}`;
    attempts += 1;
    return (import(url) as Promise<Pagefind>).then(async (module) => {
      await module.init();
      return module;
    });
  }

  const rows = () => [...list.querySelectorAll<HTMLAnchorElement>("a")];

  // The detail area shows the excerpt of the row that Enter would open.
  const show = (row: HTMLAnchorElement | undefined) => {
    const excerpt = row ? excerpts.get(row) : undefined;
    detail.hidden = !excerpt;
    // Excerpts come from this site's own index; Pagefind escapes the text and adds only <mark>.
    detail.innerHTML = excerpt ?? "";
  };

  const render = async (term: string) => {
    if (!pagefind) return;
    const search = await pagefind.debouncedSearch(term, {}, 120);
    if (!search) return;
    const data = await Promise.all(search.results.slice(0, shownResults).map((r) => r.data()));
    if (input.value !== term) return;
    list.replaceChildren();
    excerpts.clear();
    data.forEach((result, index) => {
      const item = document.createElement("li");
      const link = document.createElement("a");
      link.href = result.url;
      link.textContent = result.meta.title ?? result.url;
      const section = document.createElement("span");
      section.id = `search-section-${index}`;
      section.textContent = result.meta.section ?? "Recite";
      link.setAttribute("aria-describedby", section.id);
      excerpts.set(link, result.excerpt);
      item.append(link, section);
      list.append(item);
    });
    const count = search.results.length;
    status.textContent = !term.trim()
      ? ""
      : count === 0
        ? messages.empty
        : count === 1
          ? messages.one
          : messages.many.replace("{count}", String(count));
    show(rows()[0]);
  };

  input.addEventListener("input", () => void render(input.value));
  input.addEventListener("keydown", (event) => {
    const first = rows()[0];
    if (!first) return;
    if (event.key === "ArrowDown") {
      event.preventDefault();
      first.focus();
    } else if (event.key === "Enter") {
      event.preventDefault();
      first.click();
    }
  });
  list.addEventListener("keydown", (event) => {
    const all = rows();
    const index = all.indexOf(document.activeElement as HTMLAnchorElement);
    if (index < 0) return;
    if (event.key === "ArrowDown" && index < all.length - 1) {
      event.preventDefault();
      all[index + 1]!.focus();
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      (index === 0 ? input : all[index - 1]!).focus();
    }
  });
  list.addEventListener("focusin", (event) => {
    if (event.target instanceof HTMLAnchorElement) show(event.target);
  });
  input.addEventListener("focus", () => show(rows()[0]));

  trigger.hidden = false;
  trigger.addEventListener("click", async () => {
    dialog.showModal();
    if (!pagefind) {
      status.textContent = messages.loading;
      loading ??= load().catch((error: unknown) => {
        loading = undefined;
        throw error;
      });
      try {
        pagefind = await loading;
        status.textContent = "";
      } catch {
        status.textContent = messages.failed;
        return;
      }
    }
    if (dialog.open) {
      input.focus();
      input.select();
      if (input.value) void render(input.value);
    }
  });
  dialog.addEventListener("close", () => trigger.focus());
}
