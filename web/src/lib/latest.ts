// A job that is asked for again and again while the last one still runs (a setting
// clicked through quickly) only needs its newest request: the ones in between are
// dropped, so a slow job never builds up a queue of work nobody will look at.

/** What a dropped request is rejected with. */
export class Superseded extends Error {
  constructor() {
    super('A newer request replaced this one.');
    this.name = 'Superseded';
  }
}

/**
 * `run` as a function that runs one request at a time. A request that arrives while
 * another runs waits for it; if a third arrives meanwhile, the waiting one is rejected
 * with `Superseded` and only the newest is run.
 */
export function latestOnly<A, R>(run: (argument: A) => Promise<R>): (argument: A) => Promise<R> {
  let busy = false;
  let waiting: { argument: A; resolve: (value: R) => void; reject: (error: unknown) => void } | null = null;

  async function pump() {
    busy = true;
    while (waiting) {
      const job = waiting;
      waiting = null;
      try {
        job.resolve(await run(job.argument));
      } catch (error) {
        job.reject(error);
      }
    }
    busy = false;
  }

  return (argument) =>
    new Promise<R>((resolve, reject) => {
      waiting?.reject(new Superseded());
      waiting = { argument, resolve, reject };
      if (!busy) void pump();
    });
}
