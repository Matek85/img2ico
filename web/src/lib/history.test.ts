import { describe, expect, it } from 'vitest';
import { History, histories } from './history';

function through(...states: number[]) {
  const history = new History<number>();
  history.start(states[0]);
  for (const state of states.slice(1)) history.record(state);
  return history;
}

describe('History', () => {
  it('goes back and forward through the states', () => {
    const history = through(1, 2, 3);
    expect(history.canUndo).toBe(true);
    expect(history.canRedo).toBe(false);
    expect(history.undo()).toBe(2);
    expect(history.undo()).toBe(1);
    expect(history.undo()).toBeNull();
    expect(history.canRedo).toBe(true);
    expect(history.redo()).toBe(2);
    expect(history.redo()).toBe(3);
    expect(history.redo()).toBeNull();
    expect(history.canRedo).toBe(false);
  });

  it('forgets what could be redone when something new is done', () => {
    const history = through(1, 2, 3);
    history.undo();
    history.record(9);
    expect(history.canRedo).toBe(false);
    expect(history.all).toEqual([1, 2, 9]);
    expect(history.current).toBe(9);
  });

  it('goes to any state at once', () => {
    const history = through(1, 2, 3, 4);
    expect(history.goTo(0)).toBe(1);
    expect(history.index).toBe(0);
    expect(history.goTo(2)).toBe(3);
    expect(history.canRedo).toBe(true);
    expect(history.goTo(7)).toBeNull();
    expect(history.goTo(-1)).toBeNull();
    expect(history.index).toBe(2);
  });

  it('keeps copies: changing a state afterwards does not change the history', () => {
    const history = new History<{ list: number[] }>();
    const state = { list: [1] };
    history.start(state);
    state.list.push(2);
    expect(history.current).toEqual({ list: [1] });
  });

  it('keeps only the most recent states', () => {
    const history = new History<number>(3);
    history.start(0);
    for (let n = 1; n <= 5; n += 1) history.record(n);
    expect(history.all).toEqual([2, 3, 4, 5]);
    expect(history.index).toBe(3);
  });

  it('is kept for an icon of the queue until it is dropped', () => {
    const history = through(1, 2);
    histories.keep(7, history);
    expect(histories.take<number>(7)?.current).toBe(2);
    histories.drop(7);
    expect(histories.take(7)).toBeUndefined();
  });
});
