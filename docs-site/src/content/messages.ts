import english from "./i18n/en.json";

export function text(name: keyof typeof english) {
  return english[name];
}
