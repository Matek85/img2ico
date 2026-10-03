// The states a person has been through, as a timeline with a place in it: Undo and Redo step along it, and any
// state can be gone to at once. Only the bookkeeping lives here; when a change is a change (a slider dragged
// along is one) is decided by the editor.

export class History<T, N = undefined> {
  // The states are kept as JSON text, so nothing that is later changed in place can change what was kept.
  private states: string[] = [];
  private notes: N[] = [];
  private at = -1;
  /** How many of the oldest states were let go to stay within the limit. */
  private gone = 0;

  /** `limit` is the most states kept (the first one included); the oldest are let go. */
  constructor(private readonly limit = 50) {}

  get started(): boolean {
    return this.at >= 0;
  }

  /** The first state of the timeline. */
  start(state: T, note?: N): void {
    this.states = [JSON.stringify(state)];
    this.notes = [note as N];
    this.at = 0;
    this.gone = 0;
  }

  /** The state the person is at. */
  get current(): T {
    return JSON.parse(this.states[this.at]) as T;
  }

  /** A change was made: this is the new state, with a note on it. Whatever could have been redone is gone. */
  record(state: T, note?: N): void {
    this.states.length = this.at + 1;
    this.notes.length = this.at + 1;
    this.states.push(JSON.stringify(state));
    this.notes.push(note as N);
    this.at = this.states.length - 1;
    while (this.states.length > this.limit) {
      this.states.shift();
      this.notes.shift();
      this.at -= 1;
      this.gone += 1;
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

  /** What was noted with each state, the oldest first. */
  get noted(): N[] {
    return [...this.notes];
  }

  get dropped(): number {
    return this.gone;
  }

  get index(): number {
    return this.at;
  }
}

// The timelines of the icons in the queue: leaving an icon keeps its changes, coming back finds them again.
const kept = new Map<number, History<unknown, unknown>>();

export const histories = {
  keep<T, N>(id: number, history: History<T, N>): void {
    if (history.started) kept.set(id, history as unknown as History<unknown, unknown>);
  },
  take<T, N = undefined>(id: number): History<T, N> | undefined {
    return kept.get(id) as unknown as History<T, N> | undefined;
  },
  drop(id: number): void {
    kept.delete(id);
  },
  clear(): void {
    kept.clear();
  },
};
