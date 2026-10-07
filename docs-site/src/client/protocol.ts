// Site-private projection of the Rust host. Never interpret Recite source here.
export interface Line {
  id: string;
  text: string;
  speaker: string | null;
}
export interface Choice {
  id: string;
  text: string;
  available: boolean;
  reason: string | null;
}
export interface Effect {
  id: string;
  function: string;
  mode: "immediate" | "blocking" | "deferred";
  args: { type: string; value: string | number | boolean; }[];
}
export interface Diagnostic {
  code: string;
  compatibility_message: string | null;
  span: { start: { line: number; column: number; }; };
}
export type Output =
  | { kind: "line"; line: Line; }
  | { kind: "prompt"; line: Line | null; choices: Choice[]; }
  | { kind: "effect"; effect: Effect; }
  | { kind: "end"; deferred_effects: Effect[]; }
  | { kind: "diagnostics"; diagnostics: Diagnostic[]; };
export type Command =
  | { kind: "run"; source: string; moduleUrl: string; binaryUrl: string; }
  | { kind: "advance"; }
  | { kind: "select"; id: string; }
  | { kind: "acknowledge"; };
export type Request = { id: number; command: Command; };
export type Response =
  & { id: number; }
  & (
    | { kind: "executing"; }
    | { kind: "output"; output: Output; }
    | { kind: "error"; message: string; }
    | { kind: "trap"; }
  );
