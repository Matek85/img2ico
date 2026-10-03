// Undo and redo: the states a person has been through. Only the bookkeeping lives here; when a change is a change
// (a slider dragged along is one) is decided by the editor.

export class History<T> {
  // The states are kept as JSON text, so nothing that is later changed in place can change what was kept.
  private past: string[] = [];
  private future: string[] = [];

  constructor(private readonly limit = 100) {}

  /** A change was made: `before` is the state it started from. Whatever could have been redone is gone. */
  push(before: T): void {
    this.past.push(JSON.stringify(before));
    if (this.past.length > this.limit) this.past.shift();
    this.future = [];
  }

  /** Steps back from `current`: the state before, or `null` if there is none. */
  undo(current: T): T | null {
    const previous = this.past.pop();
    if (previous === undefined) return null;
    this.future.push(JSON.stringify(current));
    return JSON.parse(previous) as T;
  }

  /** Steps forward again from `current`: the state that was undone, or `null` if there is none. */
  redo(current: T): T | null {
    const next = this.future.pop();
    if (next === undefined) return null;
    this.past.push(JSON.stringify(current));
    return JSON.parse(next) as T;
  }

  get canUndo(): boolean {
    return this.past.length > 0;
  }

  get canRedo(): boolean {
    return this.future.length > 0;
  }
}
