// The few functions of Node's file system that a test reads the Rust sources with
// (the page itself runs in the browser and has no use for Node's types).
declare module 'node:fs' {
  export function readFileSync(path: URL, encoding: 'utf8'): string;
  export function readdirSync(path: URL): string[];
  export function mkdirSync(path: URL, options: { recursive: boolean }): void;
  export function writeFileSync(path: URL, data: string): void;
}
