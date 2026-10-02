import { describe, expect, it } from 'vitest';
import { explain, friendly } from './messages';

describe('the engine messages as sentences for the page', () => {
  it('rewrites the background removal warnings', () => {
    const nothing = friendly(
      'Warning: nothing was removed - no pixel connected to the image border is close to #ffffff. Check the color and --tolerance (or let img2ico detect it with --chroma-key auto).',
    );
    expect(nothing).toContain('#ffffff');
    expect(nothing).not.toContain('--');
    expect(nothing).not.toContain('img2ico');

    const little = friendly(
      'Warning: only 0.4% of the image matched the background color #00ff00 - almost nothing was removed. Check the color and --tolerance.',
    );
    expect(little).toContain('0.4%');
    expect(little).toContain('#00ff00');
    expect(little).not.toContain('--');

    const much = friendly(
      "Warning: 93.1% of the image matched the background color #101010 - almost everything was removed. The color may be too close to the artwork's, or --tolerance too high.",
    );
    expect(much).toContain('93.1%');
    expect(much).not.toContain('--');
  });

  it('rewrites the detection error, the trim warnings and the thin sliver warning', () => {
    const detect = friendly(
      'Could not detect a single background color: the most common color along the border, #aabbcc, covers only 31% of it (within a tolerance of 10%). The background may be a gradient or a photo. Give the color yourself with --chroma-key #RRGGBB, or raise --tolerance.',
    );
    expect(detect).toContain('#aabbcc');
    expect(detect).toContain('31%');
    expect(detect).toContain('10%');
    expect(detect).not.toContain('--');

    expect(friendly('Warning: --trim found no transparent margin to cut - the image has content up to its edges. For a solid-color background, remove it first (--chroma-key).')).not.toContain('--');
    expect(friendly('Warning: --trim found nothing to keep - the image is completely transparent.')).toContain('completely transparent');
    const sliver = friendly(
      'Warning: at these sizes, only a thin sliver of the actual artwork will be visible: 16, 32 - this can be caused by an elongated source image, a high --padding value, or both. Consider less padding.',
    );
    expect(sliver).toContain('16, 32');
    expect(sliver).not.toContain('--');
  });

  it('rewrites the point and the size limit', () => {
    expect(friendly('Warning: seed point (500,20) is outside the image (400x300) and will be ignored.')).toContain('(500, 20)');
    const big = friendly('the image is 20000x20000 pixels (400.0 megapixels), more than the limit of 100.0 megapixels. Raise the limit with --max-pixels N (0 for no limit) if you trust the file.');
    expect(big).toContain('20000 × 20000');
    expect(big).not.toContain('--');
  });

  it('puts the option names of an unknown sentence into words', () => {
    expect(friendly('Warning: something about --tolerance and --foo-bar happened')).toBe(
      'Something about the tolerance and foo bar happened',
    );
  });

  it('takes the text of an error', () => {
    expect(explain(new Error('Warning: --trim found nothing to keep - the image is completely transparent.'))).toContain('Nothing was kept');
    expect(explain('plain text')).toBe('Plain text');
  });
});
