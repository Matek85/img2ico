import { describe, expect, it } from 'vitest';
import { parseReport } from './report';

describe('parseReport', () => {
  it('reads what the engine sends', () => {
    const report = parseReport(
      JSON.stringify({ valid: false, bytes: 3, images: [], errors: [{ image: null, message: 'x' }], warnings: [] }),
    );
    expect(report.valid).toBe(false);
    expect(report.errors[0].message).toBe('x');
  });

  it('rejects something that is not a report', () => {
    expect(() => parseReport('{"hello":1}')).toThrow();
  });
});
