import { text } from "../content/messages.ts";
import { modes as links } from "./modes.ts";

const themes = [
  ["auto", text("ui.system")],
  ["light", text("ui.light")],
  ["dark", text("ui.dark")],
] as const;

function current(url: string, href: string, sections: string[]) {
  if (url === href) return "page";
  return sections.some((section) => url.startsWith(section)) ? "true" : undefined;
}

// A one-of choice is a segment track, as in Writer. Each instance needs its own radio name.
function ThemeSelector({ name }: { name: string }) {
  return (
    <fieldset class="theme-control segments" data-theme-select>
      <legend class="visually-hidden">{text("ui.theme")}</legend>
      {themes.map(([value, label]) => (
        <label>
          <input type="radio" name={name} value={value} checked={value === "auto"} />
          <span>{label}</span>
        </label>
      ))}
    </fieldset>
  );
}

// Writer's pickers open from a field-like control; this is a button styled as that field.
function SearchField() {
  return (
    <button class="search-field" type="button" data-search-trigger hidden>
      <svg viewBox="0 0 16 16" aria-hidden="true" focusable="false">
        <circle cx="6.75" cy="6.75" r="4.75" />
        <path d="m10.25 10.25 4 4" />
      </svg>
      <span>{text("search.label")}</span>
    </button>
  );
}

export function Header({ wordmark, url }: { wordmark: string; url: string }) {
  return (
    <div class="recite-header">
      <a class="brand" href="/" aria-label="Recite home">
        <span aria-hidden="true" dangerouslySetInnerHTML={{ __html: wordmark }} />
      </a>
      <SearchField />
      <PrimaryNavigation className="desktop-nav" url={url} />
      <div class="desktop-theme">
        <ThemeSelector name="theme" />
      </div>
      <button class="menu-toggle" type="button" popovertarget="global-menu" aria-label="Site menu">
        Menu
      </button>
      <div id="global-menu" class="slip" popover="auto">
        <PrimaryNavigation url={url} />
        <ThemeSelector name="theme-menu" />
      </div>
    </div>
  );
}

function PrimaryNavigation({ className, url }: { className?: string; url: string }) {
  return (
    <nav class={className} aria-label="Main navigation">
      {links.map(({ href, title, sections }) => (
        <a href={href} aria-current={current(url, href, sections)}>
          {title}
        </a>
      ))}
    </nav>
  );
}
