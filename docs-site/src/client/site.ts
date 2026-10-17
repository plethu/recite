import { setupSearch } from "./search.ts";
const media = matchMedia("(prefers-color-scheme: dark)");
const selects = document.querySelectorAll<HTMLSelectElement>("[data-theme-select]");
let selection = "auto";
try {
  selection = localStorage.getItem("theme") ?? "auto";
} catch {
  /* Storage is optional. */
}
if (!["light", "dark"].includes(selection)) selection = "auto";
function applyTheme() {
  document.documentElement.dataset.theme =
    selection === "auto" ? (media.matches ? "dark" : "light") : selection;
  for (const select of selects) select.value = selection;
}
for (const select of selects)
  select.addEventListener("change", () => {
    selection = select.value;
    try {
      localStorage.setItem("theme", selection);
    } catch {
      /* Storage is optional. */
    }
    applyTheme();
  });
media.addEventListener("change", applyTheme);
applyTheme();
setupSearch();
