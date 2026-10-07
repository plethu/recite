(() => {
  const control = document.querySelector("#menu-control");
  const menu = document.querySelector(".book-menu");
  if (!control || !menu) return;
  control.addEventListener("change", () => {
    if (control.checked) {
      const first = menu.querySelector("#book-search-input") ?? menu.querySelector("a");
      // The checkbox's default action must reveal the CSS menu before focus.
      requestAnimationFrame(() => {
        if (control.checked) first?.focus();
      });
    }
  });
  menu.addEventListener("keydown", (event) => {
    if (event.key !== "Escape") return;
    event.preventDefault();
    control.checked = false;
    control.focus();
  });
})();
