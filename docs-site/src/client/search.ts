interface PagefindUIConstructor {
  new (options: { element: string; showSubResults: boolean; showImages: boolean }): unknown;
}
declare global {
  interface Window {
    PagefindUI?: PagefindUIConstructor;
  }
}
export function setupSearch() {
  const trigger = document.querySelector<HTMLButtonElement>("[data-search-trigger]");
  const dialog = document.querySelector<HTMLDialogElement>("#search-dialog");
  const status = dialog?.querySelector<HTMLElement>("[data-search-status]");
  if (!trigger || !dialog || !status) return;
  let loading: Promise<void> | undefined;
  let ready = false;
  function resource(tag: "script" | "link", url: string) {
    return new Promise<void>((resolve, reject) => {
      const element = document.createElement(tag);
      if (element instanceof HTMLScriptElement) element.src = url;
      else {
        element.rel = "stylesheet";
        element.href = url;
      }
      element.onload = () => resolve();
      element.onerror = () => {
        element.remove();
        reject(new Error(`Could not load ${url}`));
      };
      document.head.append(element);
    });
  }
  trigger.hidden = false;
  trigger.addEventListener("click", async () => {
    dialog.showModal();
    if (!ready) {
      status.textContent = "Loading search…";
      loading ??= Promise.all([
        resource("link", "/pagefind/pagefind-ui.css"),
        resource("script", "/pagefind/pagefind-ui.js"),
      ])
        .then(() => {
          if (!window.PagefindUI) throw new Error("Search did not initialize.");
          new window.PagefindUI({
            element: "#search-results",
            showSubResults: false,
            showImages: false,
          });
          const input = dialog.querySelector<HTMLInputElement>("input");
          if (!input) throw new Error("Search input did not initialize.");
          input.id = "documentation-search";
          ready = true;
        })
        .catch((error: unknown) => {
          loading = undefined;
          throw error;
        });
      try {
        await loading;
        status.textContent = "";
      } catch {
        status.textContent = "Search could not load. Close and reopen to retry.";
        return;
      }
    }
    if (dialog.open) dialog.querySelector<HTMLInputElement>("input")?.focus();
  });
  dialog.addEventListener("close", () => trigger.focus());
}
