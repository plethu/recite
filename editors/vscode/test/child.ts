import { EventEmitter } from "node:events";

class Input extends EventEmitter<{ error: [Error]; }> {
  writable = true;
  writes: unknown[] = [];

  write(value: string) {
    this.writes.push(JSON.parse(value) as unknown);
    return true;
  }
  end() {}
  destroy() {
    this.writable = false;
  }
}

export class FakeChild
  extends EventEmitter<{ error: [Error]; close: [number | null, string | null]; }>
{
  stdout = new EventEmitter<{ data: [Buffer | string]; }>();
  stderr = new EventEmitter<{ data: [Buffer | string]; }>();
  stdin = new Input();
  killed = false;

  kill(signal?: NodeJS.Signals) {
    this.killed = true;
    if (signal === "SIGKILL") queueMicrotask(() => this.close(1));
    return true;
  }
  close(code: number) {
    this.emit("close", code, null);
  }
}
