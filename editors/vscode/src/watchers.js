const WATCH_METHOD = "workspace/didChangeWatchedFiles";

export class WatcherRegistry {
  constructor(controller) {
    this.controller = controller;
    this.registrations = new Map();
    this.pendingEvents = new Map();
    this.flushTimer = undefined;
  }

  registerCapabilities(params) {
    for (const registration of params?.registrations ?? []) {
      if (registration.method !== WATCH_METHOD) continue;
      this.unregisterCapabilities({ unregistrations: [{ id: registration.id }] });
      this.install(registration);
    }
  }

  unregisterCapabilities(params) {
    for (const registration of params?.unregisterations ?? params?.unregistrations ?? []) {
      const installed = this.registrations.get(registration.id);
      if (!installed) continue;
      for (const disposable of installed) disposable.dispose();
      this.registrations.delete(registration.id);
    }
  }

  install(registration) {
    const { api } = this.controller;
    const installed = [];
    for (const watcher of registration.registerOptions?.watchers ?? []) {
      const fileWatcher = api.workspace.createFileSystemWatcher(this.pattern(watcher.globPattern));
      const kind = watcher.kind ?? 7;
      if (kind & 1) installed.push(fileWatcher.onDidCreate((uri) => this.queue(1, uri)));
      if (kind & 2) installed.push(fileWatcher.onDidChange((uri) => this.queue(2, uri)));
      if (kind & 4) installed.push(fileWatcher.onDidDelete((uri) => this.queue(3, uri)));
      installed.push(fileWatcher);
    }
    this.registrations.set(registration.id, installed);
  }

  pattern(pattern) {
    const { api } = this.controller;
    if (typeof pattern === "string") return pattern;
    if (pattern?.baseUri && api.RelativePattern) {
      return new api.RelativePattern(api.Uri.parse(pattern.baseUri), pattern.pattern);
    }
    return pattern?.pattern ?? "**/*";
  }

  queue(type, uri) {
    const key = `${type}\0${uri.toString()}`;
    this.pendingEvents.set(key, { type, uri: uri.toString() });
    if (this.flushTimer) return;
    this.flushTimer = setTimeout(() => {
      this.flushTimer = undefined;
      this.flush();
    }, 0);
    this.flushTimer.unref?.();
  }

  flush() {
    if (!this.pendingEvents.size) return;
    const client = this.controller.client;
    if (!client || client.status === "stopped") return;
    const changes = [...this.pendingEvents.values()]
      .sort((left, right) => left.type - right.type || left.uri.localeCompare(right.uri))
      .map(({ type, uri }) => ({ type, uri }));
    this.pendingEvents.clear();
    client.notify(WATCH_METHOD, { changes });
  }

  dispose() {
    if (this.flushTimer) clearTimeout(this.flushTimer);
    this.flushTimer = undefined;
    for (const installed of this.registrations.values()) {
      for (const disposable of installed) disposable.dispose();
    }
    this.registrations.clear();
  }
}
