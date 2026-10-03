// The states a person has been through, as a timeline with a place in it: Undo and Redo step along it, and any
// state can be gone to at once. Only the bookkeeping lives here; when a change is a change (a slider dragged
// along is one) is decided by the editor.

export class History<T> {
  // The states are kept as JSON text, so nothing that is later changed in place can change what was kept.
  private states: string[] = [];
  private at = -1;

  constructor(private readonly limit = 100) {}

  get started(): boolean {
    return this.at >= 0;
  }

  /** The first state of the timeline. */
  start(state: T): void {
    this.states = [JSON.stringify(state)];
    this.at = 0;
  }

  /** The state the person is at. */
  get current(): T {
    return JSON.parse(this.states[this.at]) as T;
  }

  /** A change was made: this is the new state. Whatever could have been redone is gone. */
  record(state: T): void {
    this.states.length = this.at + 1;
    this.states.push(JSON.stringify(state));
    this.at = this.states.length - 1;
    if (this.states.length > this.limit + 1) {
      this.states.shift();
      this.at -= 1;
    }
  }

  /** Steps back: the state before, or `null` if there is none. */
  undo(): T | null {
    return this.at > 0 ? this.goTo(this.at - 1) : null;
  }

  /** Steps forward again: the state that was undone, or `null` if there is none. */
  redo(): T | null {
    return this.at < this.states.length - 1 ? this.goTo(this.at + 1) : null;
  }

  /** Goes to the state at `index` of `all` (any number of steps at once); `null` if there is no such state. */
  goTo(index: number): T | null {
    if (!Number.isInteger(index) || index < 0 || index >= this.states.length) return null;
    this.at = index;
    return this.current;
  }

  get canUndo(): boolean {
    return this.at > 0;
  }

  get canRedo(): boolean {
    return this.at >= 0 && this.at < this.states.length - 1;
  }

  /** Every state, the oldest first, and the place the person is at. */
  get all(): T[] {
    return this.states.map((text) => JSON.parse(text) as T);
  }

  get index(): number {
    return this.at;
  }
}

// The timelines of the icons in the queue: leaving an icon keeps its changes, coming back finds them again.
const kept = new Map<number, History<never>>();

export const histories = {
  keep<T>(id: number, history: History<T>): void {
    if (history.started) kept.set(id, history as unknown as History<never>);
  },
  take<T>(id: number): History<T> | undefined {
    return kept.get(id) as unknown as History<T> | undefined;
  },
  drop(id: number): void {
    kept.delete(id);
  },
  clear(): void {
    kept.clear();
  },
};
