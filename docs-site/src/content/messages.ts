import english from "./i18n/en.json";

export function text(name: keyof typeof english) {
  return english[name];
}

/** A message with `{name}` placeholders filled from `values`. */
export function format(name: keyof typeof english, values: Record<string, string | number>) {
  return english[name].replace(/\{(\w+)\}/g, (match, key: string) =>
    key in values ? String(values[key]) : match,
  );
}
