<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { buildZip, closePicture, convert, openPicture, pngZip } from '../engine/client';
  import type { Converted, Opened } from '../engine/protocol';
  import { t } from '../i18n';
  import Compare from './Compare.svelte';
  import PixelInspector from './PixelInspector.svelte';
  import CropTool from './CropTool.svelte';
  import {
    ASPECTS,
    type AspectChoice,
    type Rect,
    aspectValue,
    clampRect,
    fitAspect,
    fullRect,
    isFull,
  } from '../lib/crop';
  import { type IconEntry, iconEntries } from '../lib/ico';
  import { type Preset, STYLE_PRESETS, USE_PRESETS, isActive, withPreset } from '../lib/presets';
  import { type BatchItem, outputName, stemOf } from '../lib/batch';
  import { ICNS_TYPE, ICO_TYPE, ZIP_TYPE, saveBytes } from '../lib/download';
  import { loadSettings, saveSettings } from '../lib/storage';
  import {
    DEFAULT_SIZES,
    SIZE_CHOICES,
    type Settings,
    defaultSettings,
    downloadName,
    toEngineOptions,
  } from '../lib/settings';

  let {
    file,
    onback,
    batch,
  }: {
    file: File;
    onback: () => void;
    /** Several pictures to turn into icons with the same settings; `file` is the one shown. */
    batch?: { items: BatchItem[]; archive: string; notes: string[] };
  } = $props();

  type Backdrop = 'checker' | 'light' | 'dark' | 'gray';
  const BACKDROPS: Backdrop[] = ['checker', 'light', 'dark', 'gray'];

  // What was chosen last time is the starting point (see storage.ts).
  let settings = $state<Settings>(loadSettings());
  let opened = $state<Opened>();
  let openFailure = $state('');
  let originalUrl = $state('');

  let backdrop = $state<Backdrop>('checker');
  let view = $state<'icon' | 'compare' | 'pixels'>('icon');
  let converted = $state<Converted>();
  let tiles = $state<{ size: number; url: string }[]>([]);
  let working = $state(false);
  let convertFailure = $state('');

  // What the last conversion was made from, so a stale answer (a slider moved
  // again while the engine was busy) is thrown away instead of shown.
  let latest = 0;

  // The picture shown, kept so it can be opened again after a batch has used the engine.
  let previewBytes = new Uint8Array();
  // A batch run is going on: the engine is busy with it, so the preview waits.
  let running = $state(false);
  let progress = $state<{ done: number; total: number; name: string } | null>(null);
  let results = $state<{ name: string; ok: boolean; message: string }[]>([]);

  // The crop frame. It is `settings.crop` only while crop is on and the frame
  // is smaller than the picture; the frame itself is kept while it is off.
  let cropOn = $state(false);
  let cropAspect = $state<AspectChoice>('free');
  let frame = $state<Rect>({ x: 0, y: 0, width: 1, height: 1 });
  let picture = $derived(opened ? { width: opened.width, height: opened.height } : undefined);

  $effect(() => {
    settings.crop = picture && cropOn && !isFull(frame, picture) ? { ...frame } : null;
  });

  onMount(async () => {
    originalUrl = URL.createObjectURL(file);
    try {
      const bytes = new Uint8Array(await file.arrayBuffer());
      previewBytes = bytes.slice();
      opened = await openPicture(bytes, file.name);
      frame = fullRect(opened);
    } catch (error) {
      openFailure = error instanceof Error ? error.message : String(error);
    }
  });

  onDestroy(() => {
    if (originalUrl) URL.revokeObjectURL(originalUrl);
    revoke(tiles);
    closePicture().catch(() => {});
  });

  function revoke(list: { url: string }[]) {
    for (const tile of list) URL.revokeObjectURL(tile.url);
  }

  function show(entries: IconEntry[]) {
    revoke(tiles);
    tiles = entries.map((entry) => ({
      size: entry.size,
      url: URL.createObjectURL(new Blob([entry.png as BlobPart], { type: 'image/png' })),
    }));
  }

  // Make the icon again whenever a setting changes - after a short pause, so
  // dragging a slider does not start a conversion for every pixel it moves.
  $effect(() => {
    if (!opened || running) return;
    const options = toEngineOptions(settings);
    if (settings.sizes.length === 0) {
      converted = undefined;
      show([]);
      convertFailure = '';
      return;
    }
    const mine = ++latest;
    working = true;
    const timer = setTimeout(async () => {
      try {
        const result = await convert(options);
        if (mine !== latest) return;
        converted = result;
        show(iconEntries(result.bytes));
        convertFailure = '';
      } catch (error) {
        if (mine !== latest) return;
        convertFailure = error instanceof Error ? error.message : String(error);
      } finally {
        if (mine === latest) working = false;
      }
    }, 120);
    return () => clearTimeout(timer);
  });

  // Remember the settings for the next visit.
  $effect(() => {
    saveSettings($state.snapshot(settings));
  });

  function usePreset(preset: Preset) {
    // Keep the crop frame: it belongs to this picture, not to a preset.
    const crop = settings.crop;
    settings = { ...withPreset($state.snapshot(settings), preset), crop };
  }

  function resetSettings() {
    settings = { ...defaultSettings(), crop: settings.crop };
  }

  function chooseAspect(choice: AspectChoice) {
    cropAspect = choice;
    const ratio = aspectValue(choice);
    if (ratio !== null && picture) frame = fitAspect(frame, ratio, picture);
  }

  function setFrame(field: keyof Rect, value: number) {
    if (!picture || !Number.isFinite(value)) return;
    frame = clampRect({ ...frame, [field]: value }, picture);
  }

  async function convertAll() {
    if (!batch || running) return;
    running = true;
    results = [];
    const format = settings.format;
    const options = toEngineOptions({ ...$state.snapshot(settings), crop: null }, format);
    const made: { name: string; bytes: Uint8Array }[] = [];
    try {
      for (const [at, item] of batch.items.entries()) {
        progress = { done: at, total: batch.items.length, name: item.name };
        try {
          await openPicture(await item.load(), item.name);
          const result = await convert(options);
          made.push({
            name: outputName(made.map((m) => m.name), item.name, format),
            bytes: result.bytes,
          });
          results.push({
            name: item.name,
            ok: true,
            message: result.warnings.length > 0 ? t('batch.warnings', { count: result.warnings.length }) : '',
          });
        } catch (error) {
          results.push({ name: item.name, ok: false, message: error instanceof Error ? error.message : String(error) });
        }
      }
      progress = { done: batch.items.length, total: batch.items.length, name: '' };
      if (made.length > 0) {
        saveBytes(await buildZip(made), `${batch.archive}_icons.zip`, ZIP_TYPE);
      }
    } finally {
      // The engine goes back to the picture that is shown.
      try {
        await openPicture(previewBytes.slice(), file.name);
      } catch {
        // The preview picture opened before, so this does not fail in practice.
      }
      running = false;
    }
  }

  async function downloadPngZip() {
    if (!converted) return;
    saveBytes(await pngZip(converted.bytes, stemOf(file.name)), `${stemOf(file.name)}_png.zip`, ZIP_TYPE);
  }

  async function download() {
    let bytes: Uint8Array | undefined;
    if (settings.format === 'ico') {
      bytes = converted?.bytes;
    } else {
      bytes = (await convert(toEngineOptions(settings, 'icns'))).bytes;
    }
    if (!bytes) return;
    saveBytes(bytes, downloadName(file.name, settings.format), settings.format === 'ico' ? ICO_TYPE : ICNS_TYPE);
  }

  // The largest image is shown big; the others are shown at their real size.
  let largest = $derived(tiles.length > 0 ? tiles[tiles.length - 1] : undefined);
  let smaller = $derived(tiles.slice(0, -1));
</script>

{#if openFailure}
  <p class="failure" role="alert">{t('state.failed', { reason: openFailure })}</p>
  <button type="button" onclick={onback}>{t('state.back')}</button>
{:else if !opened}
  <p class="working" role="status">{t('state.opening', { name: file.name })}</p>
{:else}
  <div class="bar">
    <button type="button" class="quiet" onclick={onback}>← {t('state.back')}</button>
    <span class="file">{file.name}</span>
  </div>

  <div class="editor">
    <section class="preview" aria-labelledby="preview-title">
      <h2 id="preview-title">{t('editor.preview')}</h2>

      <div class="backdrops" role="radiogroup" aria-label={t('editor.background')}>
        {#each BACKDROPS as choice (choice)}
          <label class:chosen={backdrop === choice}>
            <input type="radio" name="backdrop" value={choice} bind:group={backdrop} />
            {t(`editor.bg_${choice}`)}
          </label>
        {/each}
      </div>

      <div class="chips views" role="radiogroup" aria-label={t('editor.view')}>
        {#each ['icon', 'compare', 'pixels'] as choice (choice)}
          <label class:chosen={view === choice}>
            <input type="radio" name="view" value={choice} bind:group={view} />
            {t(`editor.view_${choice}`)}
          </label>
        {/each}
      </div>

      <div class="stage {backdrop}" aria-live="polite">
        {#if settings.sizes.length === 0}
          <p class="note">{t('editor.no_sizes')}</p>
        {:else if converted && tiles.length > 0 && view === 'pixels'}
          <PixelInspector bytes={converted.bytes} sizes={tiles.map((tile) => tile.size)} />
        {:else if largest && view === 'compare' && picture}
          <Compare before={originalUrl} after={largest.url} {picture} crop={settings.crop} />
        {:else if largest}
          <figure class="big">
            <img src={largest.url} alt="" width={largest.size} height={largest.size} />
            <figcaption>{largest.size}</figcaption>
          </figure>
        {/if}
      </div>

      {#if smaller.length > 0}
        <ul class="tiles {backdrop}">
          {#each smaller as tile (tile.size)}
            <li>
              <img src={tile.url} alt="" width={tile.size} height={tile.size} />
              <span>{tile.size}</span>
            </li>
          {/each}
        </ul>
      {/if}
      {#if tiles.length > 0}<p class="hint">{t('editor.preview_note')}</p>{/if}

      {#if working}<p class="hint" role="status">{t('editor.working')}</p>{/if}
      {#if convertFailure}
        <p class="failure" role="alert">{t('editor.convert_failed', { reason: convertFailure })}</p>
      {/if}
      {#if converted && converted.warnings.length > 0}
        <h3>{t('editor.warnings')}</h3>
        <ul class="findings warn">
          {#each converted.warnings as warning}
            <li>{warning.replace(/^Warning:\s*/, '')}</li>
          {/each}
        </ul>
      {/if}
    </section>

    <section class="controls" aria-label={t('controls.look')}>
      <figure class="original">
        <img src={originalUrl} alt={t('editor.source')} />
        <figcaption>
          {opened.vector
            ? t('editor.source_vector', { width: opened.width, height: opened.height })
            : t('editor.source_size', { width: opened.width, height: opened.height })}
        </figcaption>
      </figure>

      <fieldset class="presets">
        <legend>{t('presets.title')}</legend>
        {#each [{ label: 'presets.use', list: USE_PRESETS }, { label: 'presets.style', list: STYLE_PRESETS }] as group (group.label)}
          <div class="preset-group">
            <span class="group-label">{t(group.label)}</span>
            <div class="chips" role="group" aria-label={t(group.label)}>
              {#each group.list as preset (preset.id)}
                <button
                  type="button"
                  class="chip"
                  class:chosen={isActive(settings, preset)}
                  aria-pressed={isActive(settings, preset)}
                  title={t(`preset.${preset.id}_hint`)}
                  onclick={() => usePreset(preset)}>{t(`preset.${preset.id}`)}</button
                >
              {/each}
            </div>
          </div>
        {/each}
        <div class="preset-foot">
          <button type="button" class="quiet" onclick={resetSettings}>{t('presets.reset')}</button>
          <span class="hint">{t('presets.remembered')}</span>
        </div>
      </fieldset>

      {#if !batch && !opened.vector && picture}
        <fieldset class:empty={!cropOn}>
          <legend>
            <label><input type="checkbox" bind:checked={cropOn} /> {t('crop.use')}</label>
          </legend>
          {#if cropOn}
            <div class="chips" role="radiogroup" aria-label={t('crop.aspect')}>
              {#each ASPECTS as choice (choice)}
                <label class:chosen={cropAspect === choice}>
                  <input
                    type="radio"
                    name="crop-aspect"
                    value={choice}
                    checked={cropAspect === choice}
                    onchange={() => chooseAspect(choice)}
                  />
                  {choice === 'free' ? t('crop.aspect_free') : choice}
                </label>
              {/each}
            </div>
            <CropTool src={originalUrl} size={picture} bind:rect={frame} aspect={aspectValue(cropAspect)} />
            <p class="hint">{t('crop.hint')}</p>
            <div class="numbers">
              {#each [['x', 'crop.x'], ['y', 'crop.y'], ['width', 'crop.width'], ['height', 'crop.height']] as [field, label] (field)}
                <label>
                  <span>{t(label)}</span>
                  <input
                    type="number"
                    min="0"
                    max={field === 'x' || field === 'width' ? picture.width : picture.height}
                    value={frame[field as keyof Rect]}
                    onchange={(e) => setFrame(field as keyof Rect, e.currentTarget.valueAsNumber)}
                  />
                </label>
              {/each}
            </div>
            <button type="button" class="quiet" onclick={() => (frame = fullRect(picture))} disabled={isFull(frame, picture)}>
              {t('crop.reset')}
            </button>
          {/if}
        </fieldset>
      {/if}

      <fieldset>
        <legend>{t('controls.sizes')}</legend>
        <div class="checks">
          {#each SIZE_CHOICES as size (size)}
            <label><input type="checkbox" value={size} bind:group={settings.sizes} /> {size}</label>
          {/each}
        </div>
        <p class="hint">{t('controls.sizes_hint')}</p>
        <button
          type="button"
          class="quiet"
          onclick={() => (settings.sizes = [...DEFAULT_SIZES])}
          disabled={[...settings.sizes].sort((a, b) => a - b).join() === DEFAULT_SIZES.join()}
          >{t('controls.sizes_reset')}</button
        >
      </fieldset>

      <fieldset>
        <legend>{t('controls.look')}</legend>

        <label class="slider">
          <span>{t('controls.padding')}</span>
          <input type="range" min="0" max="40" bind:value={settings.padding} />
          <output>{settings.padding}%</output>
        </label>

        <label class="slider">
          <span>{t('controls.radius')}</span>
          <input type="range" min="0" max="50" bind:value={settings.cornerRadius} />
          <output>{settings.cornerRadius}%</output>
        </label>

        <label class="select">
          <span>{t('controls.fit')}</span>
          <select bind:value={settings.fit}>
            <option value="contain">{t('controls.fit_contain')}</option>
            <option value="cover">{t('controls.fit_cover')}</option>
          </select>
        </label>

        <label><input type="checkbox" bind:checked={settings.grayscale} /> {t('controls.grayscale')}</label>
        <label><input type="checkbox" bind:checked={settings.trim} /> {t('controls.trim')}</label>
      </fieldset>

      <fieldset class:empty={!settings.removeBackground}>
        <legend>
          <label><input type="checkbox" bind:checked={settings.removeBackground} /> {t('controls.background')}</label>
        </legend>
        {#if settings.removeBackground}
          <label><input type="checkbox" bind:checked={settings.backgroundAuto} /> {t('controls.bg_auto')}</label>
          {#if !settings.backgroundAuto}
            <label class="select">
              <span>{t('controls.bg_color')}</span>
              <input type="color" bind:value={settings.backgroundColor} />
            </label>
          {/if}
          <label class="slider">
            <span>{t('controls.tolerance')}</span>
            <input type="range" min="0" max="100" bind:value={settings.tolerance} />
            <output>{settings.tolerance}</output>
          </label>
          <p class="hint">{t('controls.tolerance_hint')}</p>
          <label class="slider">
            <span>{t('controls.feather')}</span>
            <input type="range" min="0" max="100" bind:value={settings.feather} />
            <output>{settings.feather}%</output>
          </label>
        {/if}
      </fieldset>
    </section>
  </div>

  <section class="download">
    <fieldset class="formats">
      <legend>{t('download.format')}</legend>
      <label><input type="radio" name="format" value="ico" bind:group={settings.format} /> {t('download.ico')}</label>
      <label><input type="radio" name="format" value="icns" bind:group={settings.format} /> {t('download.icns')}</label>
    </fieldset>
    {#if batch}
      <button type="button" class="primary" onclick={convertAll} disabled={running || settings.sizes.length === 0}>
        {t('batch.convert', { count: batch.items.length })}
      </button>
      {#if settings.format === 'icns'}<p class="hint">{t('download.icns_note')}</p>{/if}
      <p class="hint">{t('batch.preview')}</p>
      {#each batch.notes as note}<p class="hint">{note}</p>{/each}
      {#if progress}
        <progress max={progress.total} value={progress.done} aria-label={t('batch.progress')}></progress>
        <p class="hint" role="status">
          {progress.name
            ? t('batch.working', { done: progress.done + 1, total: progress.total, name: progress.name })
            : t('batch.finished', { count: results.filter((r) => r.ok).length, failed: results.filter((r) => !r.ok).length })}
        </p>
      {/if}
      {#if results.length > 0}
        <ul class="results">
          {#each results as result}
            <li class:bad={!result.ok}>
              {result.ok ? '✓' : '✗'} {result.name}{#if result.message}{' - '}{result.message}{/if}
            </li>
          {/each}
        </ul>
      {/if}
    {:else}
      <button type="button" class="primary" onclick={download} disabled={!converted || working}>
        {t('download.button', { name: downloadName(file.name, settings.format) })}
      </button>
      <button type="button" onclick={downloadPngZip} disabled={!converted || working}>
        {t('download.png_zip')}
      </button>
      {#if settings.format === 'icns'}<p class="hint">{t('download.icns_note')}</p>{/if}
    {/if}
  </section>
{/if}
