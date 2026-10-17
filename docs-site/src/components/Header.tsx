const links = [
  { href: "/getting-started/", title: "Learn" },
  { href: "/showcase/", title: "Showcase" },
  { href: "/reference/", title: "Reference" },
];
function Theme() {
  return (
    <label class="theme-control">
      <span class="visually-hidden">Select theme</span>
      <select data-theme-select aria-label="Select theme">
        <option value="auto">System</option>
        <option value="light">Light</option>
        <option value="dark">Dark</option>
      </select>
    </label>
  );
}
export function Header({ wordmark }: { wordmark: string }) {
  return (
    <div class="recite-header">
      <a class="brand" href="/" aria-label="Recite home">
        <span aria-hidden="true" dangerouslySetInnerHTML={{ __html: wordmark }} />
      </a>
      <nav class="desktop-nav" aria-label="Main navigation">
        {links.map(({ href, title }) => (
          <a href={href}>{title}</a>
        ))}
      </nav>
      <button type="button" data-search-trigger hidden>
        Search
      </button>
      <div class="desktop-theme">
        <Theme />
      </div>
      <button class="menu-toggle" type="button" popovertarget="global-menu" aria-label="Site menu">
        Menu
      </button>
      <div id="global-menu" popover="auto">
        <nav aria-label="Main navigation">
          {links.map(({ href, title }) => (
            <a href={href}>{title}</a>
          ))}
        </nav>
        <Theme />
      </div>
    </div>
  );
}
