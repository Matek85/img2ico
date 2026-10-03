import { readFileSync, readdirSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { en } from '../i18n/en';

// Every message the engine can raise has a code (`msg!("code", ...)` in the Rust crates), and the page
// has a sentence for it under `msg.<code>` - with the same values - or it would show the English one.

const crates = ['../../../crates/core/src', '../../../crates/wasm/src'];

function source(directory: string): string[] {
  const folder = new URL(directory + '/', import.meta.url);
  return readdirSync(folder)
    .filter((name) => name.endsWith('.rs'))
    .map((name) => readFileSync(new URL(name, folder), 'utf8'));
}

// code -> the values the Rust template names
function codesInTheEngine(): Map<string, Set<string>> {
  const found = new Map<string, Set<string>>();
  const call = /msg!\(\s*"([a-z0-9_.]+)"\s*,\s*"((?:[^"\\]|\\.)*)"/g;
  for (const file of crates.flatMap(source)) {
    for (const match of file.matchAll(call)) {
      const [, code, template] = match;
      if (code.startsWith('test.') || code.startsWith('example.')) continue;
      const names = new Set([...template.matchAll(/\{(\w+)\}/g)].map((name) => name[1]));
      found.set(code, new Set([...(found.get(code) ?? []), ...names]));
    }
  }
  return found;
}

describe('the codes of the engine and the sentences of the page', () => {
  const codes = codesInTheEngine();

  it('finds the codes', () => {
    expect(codes.size).toBeGreaterThan(80);
  });

  it('has a sentence for every code, and no sentence for a code that is gone', () => {
    const catalogue = Object.keys(en)
      .filter((key) => key.startsWith('msg.') && !key.startsWith('msg.option_') && key !== 'msg.other')
      .map((key) => key.slice(4));
    expect([...codes.keys()].filter((code) => !catalogue.includes(code))).toEqual([]);
    expect(catalogue.filter((code) => !codes.has(code))).toEqual([]);
  });

  it('only uses values the engine gives', () => {
    const wrong: string[] = [];
    for (const [code, given] of codes) {
      const sentence = (en as Record<string, string>)[`msg.${code}`] ?? '';
      for (const [, name] of sentence.matchAll(/\{(\w+)\}/g)) {
        if (!given.has(name)) wrong.push(`${code}: {${name}}`);
      }
    }
    expect(wrong).toEqual([]);
  });
});
