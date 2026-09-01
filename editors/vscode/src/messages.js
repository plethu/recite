import messages from "./messages.generated.js";

export function clientMessage(api, id, detail) {
  const template = messages[id];
  if (!template) throw new Error(`missing Recite UI message projection: ${id}`);
  if (detail === undefined) return api.l10n?.t ? api.l10n.t(template) : template;
  return api.l10n?.t
    ? api.l10n.t(template, String(detail))
    : template.replace("{0}", () => String(detail));
}
