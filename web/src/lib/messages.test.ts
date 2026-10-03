import { describe, expect, it } from 'vitest';
import type { EngineMessage } from '../engine/protocol';
import { EngineError, describeMessage, explain, friendly } from './messages';

const message = (code: string, params: EngineMessage['params'], text = 'English'): EngineMessage => ({ code, params, text });

describe('the engine messages as sentences for the page', () => {
  it('writes a message from its code and values', () => {
    const nothing = describeMessage(message('chroma.nothing_removed', { prefix: '', color: '#ffffff' }));
    expect(nothing).toContain('#ffffff');
    expect(nothing).not.toContain('--');
    expect(nothing).not.toContain('img2ico');

    const little = describeMessage(message('chroma.almost_nothing_removed', { percent: '0.4', color: '#00ff00' }));
    expect(little).toContain('0.4%');
    expect(little).toContain('#00ff00');

    const big = describeMessage(message('source.too_many_pixels', { width: '20000', height: '20000', megapixels: '400.0', limit: '40.0' }));
    expect(big).toContain('20000 × 20000');
    expect(big).not.toContain('--');
  });

  it('turns a value that is a message itself into text first', () => {
    const sentence = describeMessage(
      message('source.unreadable', {
        name: 'big.png',
        e: message('source.too_many_pixels', { width: '9000', height: '9000', megapixels: '81.0', limit: '40.0' }),
      }),
    );
    expect(sentence).toContain('big.png');
    expect(sentence).toContain('9000 × 9000');
  });

  it('shows the English sentence for a message without a code, with the option names in words', () => {
    expect(
      describeMessage(message('other', {}, 'Warning: something about --tolerance and --foo-bar happened')),
    ).toBe('Something about the tolerance and foo bar happened');
  });

  it('shows the English sentence for a code the catalogue does not know', () => {
    expect(describeMessage(message('not.yet', {}, 'Something new happened.'))).toBe('Something new happened.');
  });

  it('reads an error of the engine, with or without a message in it', () => {
    const made = new EngineError(
      JSON.stringify(message('zip.zip64', {}, 'ZIP64 files (over 4 GB or 65535 files) are not supported.')),
    );
    expect(made.message).toContain('ZIP64');
    expect(explain(made)).toContain('65,535');

    const plainError = new EngineError('No picture is open.');
    expect(plainError.info).toBeUndefined();
    expect(explain(plainError)).toBe('No picture is open.');
    expect(explain(new Error('Warning: --trim found nothing to keep.'))).toBe('Cutting away the margin found nothing to keep.');
    expect(explain('plain text')).toBe('Plain text');
    expect(friendly('Warning: --padding is high')).toBe('The margin is high');
  });
});
