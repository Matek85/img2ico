import { describe, expect, it } from 'vitest';
import { History } from './history';

describe('History', () => {
  it('goes back and forward through the states', () => {
    const history = new History<{ n: number }>();
    expect(history.canUndo).toBe(false);
    expect(history.canRedo).toBe(false);
    history.push({ n: 1 });
    history.push({ n: 2 });
    expect(history.canUndo).toBe(true);
    // Now at 3: back to 2, back to 1, then nothing more.
    expect(history.undo({ n: 3 })).toEqual({ n: 2 });
    expect(history.undo({ n: 2 })).toEqual({ n: 1 });
    expect(history.undo({ n: 1 })).toBeNull();
    expect(history.canRedo).toBe(true);
    expect(history.redo({ n: 1 })).toEqual({ n: 2 });
    expect(history.redo({ n: 2 })).toEqual({ n: 3 });
    expect(history.redo({ n: 3 })).toBeNull();
    expect(history.canRedo).toBe(false);
  });

  it('forgets what could be redone when something new is done', () => {
    const history = new History<number>();
    history.push(1);
    history.push(2);
    history.undo(3);
    expect(history.canRedo).toBe(true);
    history.push(2);
    expect(history.canRedo).toBe(false);
    expect(history.undo(4)).toBe(2);
  });

  it('keeps copies: changing a state afterwards does not change the history', () => {
    const history = new History<{ list: number[] }>();
    const state = { list: [1] };
    history.push(state);
    state.list.push(2);
    expect(history.undo(state)).toEqual({ list: [1] });
  });

  it('keeps only the most recent states', () => {
    const history = new History<number>(3);
    for (let n = 1; n <= 5; n += 1) history.push(n);
    const seen: number[] = [];
    let current = 6;
    for (let step = history.undo(current); step !== null; step = history.undo(current)) {
      seen.push(step);
      current = step;
    }
    expect(seen).toEqual([5, 4, 3]);
  });
});
