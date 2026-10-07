import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { type Download, type Page, expect, test } from '@playwright/test';

// The main paths of the page, in a real browser: converting, the formats, looking into an icon file, the queue,
// the language, the shortcuts and the help pages. The texts are the English ones.

const LOGO = fileURLToPath(new URL('../../.github/testdata/logo.png', import.meta.url));
// The same picture as an upload with a name (several files at once cannot be mixed with a path).
const LOGO_FILE = { name: 'logo.png', mimeType: 'image/png', buffer: readFileSync(LOGO) };

const SVG_WITH_TEXT = Buffer.from(
  '<svg xmlns="http://www.w3.org/2000/svg" width="256" height="256"><rect width="256" height="256" fill="#3b82f6"/><circle cx="128" cy="100" r="50" fill="#fde047"/><text x="128" y="220" font-size="48" text-anchor="middle" fill="white">Hi</text></svg>',
);

/** Puts a file on the start page and waits for the editor to have made the icon. */
async function openInEditor(page: Page, file: string | { name: string; mimeType: string; buffer: Buffer }) {
  await page.goto('/');
  await page.setInputFiles('input[type=file]', file);
  await expect(page.locator('.workspace')).toBeVisible();
  await expect(page.locator('.download button.primary')).toBeEnabled();
}

async function downloaded(download: Download): Promise<Buffer> {
  return readFileSync((await download.path())!);
}

const startDownload = (page: Page) => Promise.all([page.waitForEvent('download'), page.locator('.download button.primary').click()]);

test.describe('converting', () => {
  test('a picture becomes a Windows .ico with several sizes', async ({ page }) => {
    await openInEditor(page, LOGO);
    await expect(page.locator('.download button.primary')).toHaveText(/Download logo\.ico/);
    const [download] = await startDownload(page);
    expect(download.suggestedFilename()).toBe('logo.ico');
    const bytes = await downloaded(download);
    // An .ico starts with 0, 0, 1, 0 and the number of images.
    expect([...bytes.subarray(0, 4)]).toEqual([0, 0, 1, 0]);
    expect(bytes.readUInt16LE(4)).toBeGreaterThanOrEqual(5);
  });

  test('the macOS .icns format', async ({ page }) => {
    await openInEditor(page, LOGO);
    await page.locator('label', { hasText: 'macOS .icns' }).click();
    await expect(page.locator('.download button.primary')).toHaveText(/Download logo\.icns/);
    await expect(page.locator('.download button.primary')).toBeEnabled();
    const [download] = await startDownload(page);
    expect(download.suggestedFilename()).toBe('logo.icns');
    expect((await downloaded(download)).subarray(0, 4).toString('latin1')).toBe('icns');
  });

  test('the picture can be saved as an ordinary image: PNG in its own size, JPG, and the icon sizes', async ({ page }) => {
    await openInEditor(page, LOGO);
    await page.locator('.formats label', { hasText: 'Image' }).click();
    // The picture in its own size, as a PNG.
    await expect(page.locator('.download button.primary')).toHaveText(/Download logo\.png/);
    await expect(page.locator('.download button.primary')).toBeEnabled();
    const [png] = await startDownload(page);
    expect(png.suggestedFilename()).toBe('logo.png');
    const pngBytes = await downloaded(png);
    expect([...pngBytes.subarray(0, 4)]).toEqual([0x89, 0x50, 0x4e, 0x47]);
    // The logo is 256 x 256; a PNG says its size in the header.
    expect([pngBytes.readUInt32BE(16), pngBytes.readUInt32BE(20)]).toEqual([256, 256]);

    // A JPG has no transparency: the color it is laid on can be chosen.
    await page.locator('.image-type select').selectOption('jpg');
    await expect(page.locator('.image-background')).toBeVisible();
    await expect(page.locator('.download button.primary')).toHaveText(/Download logo\.jpg/);
    const [jpg] = await startDownload(page);
    expect(jpg.suggestedFilename()).toBe('logo.jpg');
    expect([...(await downloaded(jpg)).subarray(0, 3)]).toEqual([0xff, 0xd8, 0xff]);

    // One image for each icon size: several come as a ZIP, named after the sizes.
    await page.locator('.image-size label', { hasText: 'Icon sizes' }).click();
    await expect(page.locator('.download button.primary')).toHaveText(/Download logo_jpg\.zip/);
    const [zip] = await startDownload(page);
    const bytes = await downloaded(zip);
    expect(bytes.subarray(0, 2).toString('latin1')).toBe('PK');
    const text = bytes.toString('latin1');
    for (const name of ['logo_16x16.jpg', 'logo_256x256.jpg']) expect(text, name).toContain(name);
  });

  test('the comparison shows the saved picture, not the icon, when "Image" is saved in its own size', async ({ page }) => {
    await openInEditor(page, LOGO);
    await page.keyboard.press('2');
    await page.locator('.formats label', { hasText: 'Image' }).click();
    await page.locator('.image-size label', { hasText: 'Icon sizes' }).click();
    await expect(page.locator('.download button.primary')).toBeEnabled();
    const after = page.locator('.compare .after');
    await expect(after).toBeVisible();
    // With the icon sizes the "after" is the largest icon image ...
    const iconImage = await after.getAttribute('src');
    // ... with the original size it is the picture as it is saved, another picture.
    await page.locator('.image-size label', { hasText: 'Original size' }).click();
    await expect(after).not.toHaveAttribute('src', iconImage!);
    await expect(after).toHaveAttribute('src', /^blob:/);
    // And back.
    await page.locator('.image-size label', { hasText: 'Icon sizes' }).click();
    await expect(after).toHaveAttribute('src', iconImage!);
  });

  test('an image cannot be put in the queue', async ({ page }) => {
    await openInEditor(page, LOGO);
    await page.locator('.formats label', { hasText: 'Image' }).click();
    await expect(page.getByRole('button', { name: 'Add to queue' })).toBeDisabled();
    await page.locator('.formats label', { hasText: 'Windows .ico' }).click();
    await expect(page.getByRole('button', { name: 'Add to queue' })).toBeEnabled();
  });

  test('the website package is a ZIP with the favicon files', async ({ page }) => {
    await openInEditor(page, LOGO);
    await page.locator('label', { hasText: 'Website ZIP' }).click();
    await page.locator('dialog[open] button.primary').click();
    await expect(page.locator('.download button.primary')).toHaveText(/Download logo_favicon\.zip/);
    await expect(page.locator('.download button.primary')).toBeEnabled();
    const [download] = await startDownload(page);
    const bytes = await downloaded(download);
    expect(bytes.subarray(0, 2).toString('latin1')).toBe('PK');
    // The file names stand uncompressed in the ZIP's directory.
    const text = bytes.toString('latin1');
    for (const name of ['favicon.ico', 'apple-touch-icon.png', 'site.webmanifest']) expect(text, name).toContain(name);
  });

  test('an SVG with text opens, converts and warns that the text is left out', async ({ page }) => {
    await openInEditor(page, { name: 'text.svg', mimeType: 'image/svg+xml', buffer: SVG_WITH_TEXT });
    await expect(page.locator('.failure')).toHaveCount(0);
    await expect(page.getByText('This SVG has text, which is left out')).toBeVisible();
    const [download] = await startDownload(page);
    expect((await downloaded(download)).readUInt16LE(2)).toBe(1);
  });

  test('an SVG without text gets no such warning', async ({ page }) => {
    const plain = SVG_WITH_TEXT.toString().replace(/<text.*<\/text>/, '');
    await openInEditor(page, { name: 'plain.svg', mimeType: 'image/svg+xml', buffer: Buffer.from(plain) });
    await expect(page.getByText('left out')).toHaveCount(0);
  });

  test('a file that is no picture says so instead of failing silently', async ({ page }) => {
    await page.goto('/');
    await page.setInputFiles('input[type=file]', { name: 'notes.png', mimeType: 'image/png', buffer: Buffer.from('this is not a picture') });
    await expect(page.locator('.failure')).toBeVisible();
  });
});

test.describe('icon files and the queue', () => {
  /** An .ico made by the page itself, to feed it back in. */
  async function makeIco(page: Page): Promise<Buffer> {
    await openInEditor(page, LOGO);
    const [download] = await startDownload(page);
    return downloaded(download);
  }

  test('an .ico dropped on the start page is looked into and found valid', async ({ page }) => {
    const ico = await makeIco(page);
    await page.goto('/');
    await page.setInputFiles('input[type=file]', { name: 'made.ico', mimeType: 'image/vnd.microsoft.icon', buffer: ico });
    await expect(page.getByText('This icon file is valid.')).toBeVisible();
  });

  test('in the queue a click on an .ico edits it, and the magnifier looks into it over the editor', async ({ page }) => {
    const ico = await makeIco(page);
    await page.goto('/');
    await page.setInputFiles('input[type=file]', [LOGO_FILE, { name: 'made.ico', mimeType: 'image/vnd.microsoft.icon', buffer: ico }]);
    await expect(page.locator('.workspace')).toBeVisible();
    const rows = page.locator('.queue-list li');
    await expect(rows).toHaveCount(2);
    await expect(page.locator('.bar .file')).toHaveText('logo.png');

    // The magnifier: the inspector over the editor, which stays behind it.
    await page.getByRole('button', { name: /Look into made\.ico/ }).click();
    await expect(page.locator('dialog.inspect-dialog')).toBeVisible();
    await expect(page.locator('dialog.inspect-dialog').getByText('This icon file is valid.')).toBeVisible();
    await page.keyboard.press('Escape');
    await expect(page.locator('dialog.inspect-dialog')).toHaveCount(0);
    await expect(page.locator('.bar .file')).toHaveText('logo.png');

    // The row itself: the editor, with the icon's largest image as a picture.
    await page.getByRole('button', { name: /Edit made\.ico as a picture/ }).first().click();
    await expect(page.locator('.bar .file')).toHaveText('made_edited.png');
  });

  test('several pictures at once make a queue that can be downloaded as one ZIP', async ({ page }) => {
    await page.goto('/');
    await page.setInputFiles('input[type=file]', [LOGO_FILE, { name: 'text.svg', mimeType: 'image/svg+xml', buffer: SVG_WITH_TEXT }]);
    await expect(page.locator('.queue-list li')).toHaveCount(2);
    const [download] = await Promise.all([page.waitForEvent('download'), page.getByRole('button', { name: 'Download all as ZIP' }).click()]);
    expect((await downloaded(download)).subarray(0, 2).toString('latin1')).toBe('PK');
  });
});

test.describe('languages', () => {
  test('a language chosen in the editor keeps the work', async ({ page }) => {
    await page.goto('/es/');
    await expect(page.locator('html')).toHaveAttribute('lang', 'es');
    await expect(page.locator('.ai-note')).toBeVisible();
    await page.setInputFiles('input[type=file]', LOGO);
    await expect(page.locator('.workspace')).toBeVisible();
    await page.evaluate(() => ((window as unknown as { marker: number }).marker = 1));
    await page.locator('.menu.lang summary').click();
    await page.locator('.menu.lang a[data-lang="de"]').click();
    await expect(page.locator('html')).toHaveAttribute('lang', 'de');
    await expect(page).toHaveURL(/\/de\/$/);
    // No new page was loaded, and the editor and its picture are still there.
    expect(await page.evaluate(() => (window as unknown as { marker: number }).marker)).toBe(1);
    await expect(page.locator('.workspace')).toBeVisible();
    await expect(page.locator('.bar .file')).toHaveText('logo.png');
    await expect(page.locator('.download button.primary')).toHaveText(/logo\.ico herunterladen/);
  });

  test('the footer line is written again in the new language', async ({ page }) => {
    await page.goto('/es/');
    await expect(page.locator('#engine')).toHaveText(/^Versión del \d{4}-\d{2}-\d{2} · \S+$/);
    await page.locator('.menu.lang summary').click();
    await page.locator('.menu.lang a[data-lang="de"]').click();
    await expect(page.locator('html')).toHaveAttribute('lang', 'de');
    await expect(page.locator('#engine')).toHaveText(/^Stand \d{4}-\d{2}-\d{2} · \S+$/);
  });

  test.describe('detection', () => {
    test.use({ locale: 'de-DE' });
    test('a browser in German is sent to the German page', async ({ page }) => {
      await page.goto('/');
      await expect(page).toHaveURL(/\/de\/$/);
    });
  });

  test('the German page carries no AI note, the French one does', async ({ page }) => {
    await page.goto('/de/');
    await expect(page.locator('.ai-note')).toHaveCount(0);
    await page.goto('/fr/');
    await expect(page.locator('.ai-note')).toBeVisible();
  });
});

test.describe('shortcuts and the look of the page', () => {
  test('T switches light and dark on the start page, ? lists the keys', async ({ page }) => {
    await page.goto('/');
    const root = page.locator('html');
    await page.keyboard.press('t');
    await expect(root).toHaveAttribute('data-theme', /^(light|dark)$/);
    const first = await root.getAttribute('data-theme');
    await page.keyboard.press('t');
    expect(await root.getAttribute('data-theme')).not.toBe(first);
    await page.keyboard.press('?');
    await expect(page.locator('dialog.keys-dialog')).toBeVisible();
    await expect(page.locator('dialog.keys-dialog')).toContainText('Ctrl+Shift+Z / Ctrl+Y');
  });

  test('in the editor Z and X turn the picture while the frame stays in place', async ({ page }) => {
    await openInEditor(page, LOGO);
    await page.keyboard.press('c');
    const angle = page.locator('input[type=range]');
    await expect(angle).toHaveValue('0');
    await page.keyboard.press('z');
    await expect(angle).toHaveValue('-90');
    await page.keyboard.press('x');
    await expect(angle).toHaveValue('0');
  });

  test('mirroring: H and the buttons mirror the picture, and the icon is made from the mirrored picture', async ({ page }) => {
    await openInEditor(page, LOGO);
    const [plain] = await startDownload(page);
    const before = await downloaded(plain);
    await page.keyboard.press('c');
    const horizontal = page.getByRole('button', { name: 'Mirror the picture left to right' });
    const vertical = page.getByRole('button', { name: 'Mirror the picture top to bottom' });
    await expect(horizontal).toHaveAttribute('aria-pressed', 'false');
    await page.keyboard.press('h');
    await expect(horizontal).toHaveAttribute('aria-pressed', 'true');
    await vertical.click();
    await expect(vertical).toHaveAttribute('aria-pressed', 'true');
    await vertical.click();
    await expect(vertical).toHaveAttribute('aria-pressed', 'false');
    await page.keyboard.press('Escape');
    await expect(page.locator('.download button.primary')).toBeEnabled();
    const [mirrored] = await startDownload(page);
    expect((await downloaded(mirrored)).equals(before)).toBe(false);
    // Reset puts the picture back the way it was.
    await page.getByRole('button', { name: 'Reset to defaults' }).click();
    await page.getByRole('button', { name: /Yes, reset/ }).click();
    await expect(page.locator('.download button.primary')).toBeEnabled();
    const [again] = await startDownload(page);
    expect((await downloaded(again)).equals(before)).toBe(true);
  });

  test('settings as a file: exported for the command line, taken back, and a mistake stops the import', async ({ page }) => {
    await openInEditor(page, LOGO);
    const exportSettings = async () => {
      const [download] = await Promise.all([page.waitForEvent('download'), page.getByRole('button', { name: 'Export settings' }).click()]);
      expect(download.suggestedFilename()).toBe('img2ico.toml');
      return (await downloaded(download)).toString('utf8');
    };
    const first = await exportSettings();
    expect(first).toContain('sizes = "16,32,48,64,128,256"');
    expect(first).toContain('output-format = "ico"');

    // Taking settings from a file: some are the command line's only and are reported, the rest is in use.
    const file = (...lines: string[]) => ({ name: 'img2ico.toml', mimeType: 'text/plain', buffer: Buffer.from(lines.join('\n') + '\n') });
    const fromFile = file('sizes = "16,32"', 'fit = "cover"', 'padding = 12', 'jobs = 4');
    await page.setInputFiles('input[type=file][accept=".toml,text/plain"]', fromFile);
    await expect(page.locator('dialog[open]')).toContainText('Only for the command line, not used here: jobs');
    await page.getByRole('button', { name: 'Close' }).click();
    const second = await exportSettings();
    expect(second).toContain('sizes = "16,32"');
    expect(second).toContain('fit = "cover"');
    expect(second).toContain('padding = 12');
    expect(second).not.toContain('jobs');

    // A name nobody knows: nothing changes, and the dialog says which line.
    const typo = file('padding = 30', 'toleranse = 20');
    await page.setInputFiles('input[type=file][accept=".toml,text/plain"]', typo);
    await expect(page.locator('dialog[open]')).toContainText('Line 2: “toleranse” is not a setting img2ico knows');
    await page.getByRole('button', { name: 'Close' }).click();
    expect(await exportSettings()).toBe(second);
  });

  test('a file saved after mirroring, turning and cropping brings all of it back after a reset', async ({ page }) => {
    await openInEditor(page, LOGO);
    await page.keyboard.press('c');
    await page.keyboard.press('h');
    await page.keyboard.press('z');
    const angle = page.locator('input[type=range]');
    await expect(angle).toHaveValue('-90');
    // A frame smaller than the picture: the crop.
    await page.locator('.numbers.sides input[type=number]').nth(2).fill('100');
    await page.locator('.numbers.sides input[type=number]').nth(2).blur();
    await page.keyboard.press('Escape');
    const [download] = await Promise.all([page.waitForEvent('download'), page.getByRole('button', { name: 'Export settings' }).click()]);
    const text = (await downloaded(download)).toString('utf8');
    expect(text).toContain('flip-horizontal = true');
    expect(text).toContain('rotate = 270');
    expect(text).toMatch(/crop = "\d+,\d+,100,\d+"/);

    await page.getByRole('button', { name: 'Reset to defaults' }).click();
    await page.getByRole('button', { name: /Yes, reset/ }).click();
    const [reset] = await Promise.all([page.waitForEvent('download'), page.getByRole('button', { name: 'Export settings' }).click()]);
    expect((await downloaded(reset)).toString('utf8')).not.toMatch(/flip|rotate|crop =/);

    await page.setInputFiles('input[type=file][accept=".toml,text/plain"]', { name: 'img2ico.toml', mimeType: 'text/plain', buffer: Buffer.from(text) });
    await expect(page.locator('dialog[open]')).toHaveCount(0);
    const [again] = await Promise.all([page.waitForEvent('download'), page.getByRole('button', { name: 'Export settings' }).click()]);
    expect((await downloaded(again)).toString('utf8')).toBe(text);
    // And the tools show it.
    await page.keyboard.press('c');
    await expect(page.getByRole('button', { name: 'Mirror the picture left to right' })).toHaveAttribute('aria-pressed', 'true');
    await expect(angle).toHaveValue('-90');
  });

  test('undo and redo: Ctrl+Z, Ctrl+Shift+Z, Ctrl+Y and the arrows; a run of quick changes is one', async ({ page }) => {
    await openInEditor(page, LOGO);
    await page.keyboard.press('c');
    const horizontal = page.getByRole('button', { name: 'Mirror the picture left to right' });
    const vertical = page.getByRole('button', { name: 'Mirror the picture top to bottom' });
    const undoButton = page.getByRole('button', { name: 'Undo' });
    const redoButton = page.getByRole('button', { name: 'Redo' });
    await expect(undoButton).toBeDisabled();
    await expect(redoButton).toBeDisabled();

    await page.keyboard.press('h');
    await expect(undoButton).toBeEnabled();
    await page.waitForTimeout(700);
    await page.keyboard.press('v');
    await page.waitForTimeout(700);
    await expect(horizontal).toHaveAttribute('aria-pressed', 'true');
    await expect(vertical).toHaveAttribute('aria-pressed', 'true');

    await page.keyboard.press('Control+z');
    await expect(vertical).toHaveAttribute('aria-pressed', 'false');
    await expect(horizontal).toHaveAttribute('aria-pressed', 'true');
    await expect(redoButton).toBeEnabled();
    await undoButton.click();
    await expect(horizontal).toHaveAttribute('aria-pressed', 'false');
    await expect(undoButton).toBeDisabled();
    await page.keyboard.press('Control+y');
    await expect(horizontal).toHaveAttribute('aria-pressed', 'true');
    await page.keyboard.press('Control+Shift+z');
    await expect(vertical).toHaveAttribute('aria-pressed', 'true');
    await expect(redoButton).toBeDisabled();

    // Changes in quick succession are one step back.
    await page.keyboard.press('Control+z');
    await page.keyboard.press('Control+z');
    await expect(undoButton).toBeDisabled();
    await page.keyboard.press('h');
    await page.keyboard.press('v');
    await page.keyboard.press('z');
    await expect(page.locator('input[type=range]')).toHaveValue('-90');
    await page.keyboard.press('Control+z');
    await expect(horizontal).toHaveAttribute('aria-pressed', 'false');
    await expect(vertical).toHaveAttribute('aria-pressed', 'false');
    await expect(page.locator('input[type=range]')).toHaveValue('0');
    // And the icon follows: what is made now is the unchanged picture.
    await expect(page.locator('.download button.primary')).toBeEnabled();
  });

  test('the list of changes goes back several steps at once and shows the picture as it was', async ({ page }) => {
    await openInEditor(page, LOGO);
    const list = page.locator('.change-list');
    // Folded away until asked for.
    await expect(list).toHaveCount(0);
    await page.keyboard.press('c');
    await page.keyboard.press('h');
    await page.waitForTimeout(700);
    await page.keyboard.press('z');
    await page.waitForTimeout(700);
    await page.keyboard.press('v');
    await page.waitForTimeout(700);
    await page.getByRole('button', { name: 'Show the changes' }).click();
    await expect(page.locator('#changes')).toBeFocused();
    const rows = list.locator('button.change');
    await expect(rows).toHaveCount(4);
    // The newest on top, each with its number, date and time; the first is where the picture was opened.
    await expect(rows.nth(0)).toContainText(/^4\s.*\d{1,2}[:.]\d{2}[:.]\d{2}.*Mirror/s);
    await expect(rows.nth(3)).toContainText(/^1\s/);
    await expect(rows.nth(0)).toContainText('Mirror');
    await expect(rows.nth(1)).toContainText('Turn');
    await expect(rows.nth(3)).toContainText('Picture opened');
    await expect(rows.nth(0)).toHaveAttribute('aria-current', 'step');

    // Pointing at a step shows the picture as it was then.
    await rows.nth(2).hover();
    await expect(page.locator('.past img')).toBeVisible();

    // A click goes back two steps at once; the steps after it can be brought back.
    await rows.nth(2).click();
    await expect(page.getByRole('button', { name: 'Mirror the picture left to right' })).toHaveAttribute('aria-pressed', 'true');
    await expect(page.getByRole('button', { name: 'Mirror the picture top to bottom' })).toHaveAttribute('aria-pressed', 'false');
    await expect(page.locator('input[type=range]')).toHaveValue('0');
    await expect(rows.nth(2)).toHaveAttribute('aria-current', 'step');
    await expect(page.getByRole('button', { name: 'Redo' })).toBeEnabled();
    await rows.nth(0).click();
    await expect(page.locator('input[type=range]')).toHaveValue('-90');
  });

  test('every picture of the queue keeps its own changes', async ({ page }) => {
    await page.goto('/');
    await page.setInputFiles('input[type=file]', [LOGO_FILE, { name: 'text.svg', mimeType: 'image/svg+xml', buffer: SVG_WITH_TEXT }]);
    await expect(page.locator('.queue-list li')).toHaveCount(2);
    await expect(page.locator('.bar .file')).toHaveText('logo.png');
    await expect(page.locator('.editing-note')).toBeVisible();
    await page.keyboard.press('c');
    await page.keyboard.press('h');
    await page.waitForTimeout(700);
    await page.keyboard.press('v');
    await page.waitForTimeout(700);
    await page.getByRole('button', { name: 'Show the changes' }).click();
    await expect(page.locator('.change-list button.change')).toHaveCount(3);

    // The other picture has none of them ...
    await page.getByRole('button', { name: /Edit text\.ico/ }).first().click();
    await expect(page.locator('.bar .file')).toHaveText('text.svg');
    await expect(page.locator('.download button.primary')).toBeEnabled();
    // (A vector picture has no crop panel; its button is under the editor.)
    await page.getByRole('button', { name: 'Show the changes' }).click();
    await expect(page.locator('.change-list button.change')).toHaveCount(1);
    // ... and the first has them still.
    await page.getByRole('button', { name: /Edit logo\.ico/ }).first().click();
    await expect(page.locator('.bar .file')).toHaveText('logo.png');
    await page.evaluate(() => (document.activeElement as HTMLElement | null)?.blur());
    await page.keyboard.press('c');
    await page.getByRole('button', { name: 'Show the changes' }).click();
    await expect(page.locator('.change-list button.change')).toHaveCount(3);
    await page.keyboard.press('Control+z');
    await expect(page.locator('.change-list button.change').nth(1)).toHaveAttribute('aria-current', 'step');
    await page.locator('.changes-head').getByRole('button', { name: 'Hide the changes' }).click();
    await expect(page.locator('.change-list')).toHaveCount(0);
  });

  test('"Reset to defaults" can be undone', async ({ page }) => {
    await openInEditor(page, LOGO);
    await page.getByLabel('macOS .icns').or(page.locator('label', { hasText: 'macOS .icns' })).first().click();
    await expect(page.locator('.download button.primary')).toHaveText(/logo\.icns/);
    await page.waitForTimeout(700);
    await page.getByRole('button', { name: 'Reset to defaults' }).click();
    await page.getByRole('button', { name: /Yes, reset/ }).click();
    await expect(page.locator('.download button.primary')).toHaveText(/logo\.ico/);
    await page.keyboard.press('Control+z');
    await expect(page.locator('.download button.primary')).toHaveText(/logo\.icns/);
  });

  test('a crop that does not fit the picture is left out, and a note says so', async ({ page }) => {
    await openInEditor(page, LOGO);
    const file = { name: 'img2ico.toml', mimeType: 'text/plain', buffer: Buffer.from('crop = "10,10,5000,5000"\n') };
    await page.setInputFiles('input[type=file][accept=".toml,text/plain"]', file);
    await expect(page.locator('dialog[open]')).toContainText('does not fit this picture');
  });

  test('the footer says which build the page is', async ({ page }) => {
    await page.goto('/');
    // The date of the build and the commit (a local build says "dev").
    await expect(page.locator('#engine')).toHaveText(/^Build \d{4}-\d{2}-\d{2} · \S+$/);
  });

  test('a help page is styled from the first frame', async ({ page }) => {
    await page.goto('/help/settings/');
    await expect(page.locator('h1')).toBeVisible();
    const background = await page.evaluate(() => getComputedStyle(document.body).backgroundColor);
    expect(background).not.toBe('rgba(0, 0, 0, 0)');
  });
});
