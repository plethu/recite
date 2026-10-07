(() => {
  const media = matchMedia("(prefers-color-scheme: dark)");
  let preference = "auto";
  try {
    preference = localStorage.getItem("recite-theme") ?? "auto";
  } catch {}
  if (!["auto", "light", "dark"].includes(preference)) preference = "auto";
  const apply = () => {
    document.documentElement.dataset.theme = preference === "auto"
      ? (media.matches ? "dark" : "light")
      : preference;
  };
  apply();
  media.addEventListener("change", apply);
  document.addEventListener("DOMContentLoaded", () => {
    const selector = document.querySelector("[data-theme-select]");
    if (!selector) return;
    selector.closest(".theme-control").hidden = false;
    selector.value = preference;
    selector.addEventListener("change", () => {
      preference = selector.value;
      try {
        localStorage.setItem("recite-theme", preference);
      } catch {}
      apply();
    });
  });
})();
