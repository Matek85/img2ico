// The messages between the page and the engine worker.

export type Request =
  | { id: number; op: 'version' }
  | { id: number; op: 'validate'; bytes: Uint8Array };

export type Response =
  | { id: number; ok: true; value: string }
  | { id: number; ok: false; error: string };
