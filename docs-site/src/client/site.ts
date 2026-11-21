import { setupSearch } from "./search.ts";
const media = matchMedia("(prefers-color-scheme: dark)");
const options = document.querySelectorAll<HTMLInputElement>("[data-theme-select] input");
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
  for (const option of options) option.checked = option.value === selection;
}
for (const option of options)
  option.addEventListener("change", () => {
    if (!option.checked) return;
    selection = option.value;
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
