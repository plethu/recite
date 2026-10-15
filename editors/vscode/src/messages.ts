import messages from "./messages.generated.js";
import type { HostApi } from "./types.ts";

export function clientMessage(
  api: HostApi,
  id: keyof typeof messages,
  ...arguments_: (string | number | boolean)[]
): string {
  const template = messages[id];
  if (!template) throw new Error(`missing Recite UI message projection: ${id}`);
  if (arguments_.length === 0) return api.l10n?.t ? api.l10n.t(template) : template;
  return api.l10n?.t
    ? api.l10n.t(template, ...arguments_)
    : arguments_.reduce<string>(
        (value, argument, index) => value.replace(`{${index}}`, () => String(argument)),
        template,
      );
}
