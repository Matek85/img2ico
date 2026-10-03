import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

// A missing brace does not stop the build: the browser reads what follows as nested rules and
// those quietly stop working (it happened once, when two edits of the file were merged).
describe('the stylesheet', () => {
  it('opens and closes the same number of blocks', () => {
    const css = readFileSync(new URL('./app.css', import.meta.url), 'utf8').replace(/\/\*[\s\S]*?\*\//g, '');
    expect(css.split('{').length).toBe(css.split('}').length);
  });
});
