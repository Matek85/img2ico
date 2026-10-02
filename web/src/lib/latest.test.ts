import { describe, expect, it } from 'vitest';
import { Superseded, latestOnly } from './latest';

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((r) => (resolve = r));
  return { promise, resolve };
}

describe('only the newest request runs', () => {
  it('runs a single request at once', async () => {
    const run = latestOnly(async (n: number) => n * 2);
    expect(await run(21)).toBe(42);
  });

  it('drops the requests in between while one is running', async () => {
    const started: number[] = [];
    const gates = new Map<number, ReturnType<typeof deferred<number>>>();
    const run = latestOnly((n: number) => {
      started.push(n);
      const gate = deferred<number>();
      gates.set(n, gate);
      return gate.promise;
    });

    const first = run(1);
    const second = run(2);
    const third = run(3);
    const fourth = run(4);
    await expect(second).rejects.toBeInstanceOf(Superseded);
    await expect(third).rejects.toBeInstanceOf(Superseded);

    gates.get(1)!.resolve(10);
    expect(await first).toBe(10);
    // let the newest one start, then finish it
    await Promise.resolve();
    await Promise.resolve();
    gates.get(4)!.resolve(40);
    expect(await fourth).toBe(40);
    expect(started).toEqual([1, 4]);
  });

  it('passes an error on to its request and carries on', async () => {
    let fail = true;
    const run = latestOnly(async (n: number) => {
      if (fail) throw new Error('broken');
      return n;
    });
    await expect(run(1)).rejects.toThrow('broken');
    fail = false;
    expect(await run(2)).toBe(2);
  });
});
