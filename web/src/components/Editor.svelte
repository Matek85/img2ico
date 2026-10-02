<script lang="ts">
  import { onDestroy, onMount, tick } from 'svelte';
  import {
    closePicture,
    convert,
    faviconPack,
    faviconSnippet,
    openPicture,
    pngZip,
  } from '../engine/client';
  import type { Converted, Opened } from '../engine/protocol';
  import { t } from '../i18n';
  import Compare from './Compare.svelte';
  import PixelInspector from './PixelInspector.svelte';
  import CropTool from './CropTool.svelte';
  import Icon from './Icon.svelte';
  import Queue from './Queue.svelte';
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
  import { stemOf } from '../lib/batch';
  import { ICNS_TYPE, ICO_TYPE, ZIP_TYPE, saveBytes } from '../lib/download';
  import { type QueueItem, queue } from '../lib/queue.svelte';
  import { loadAutoSave, loadSettings, saveAutoSave, saveSettings } from '../lib/storage';
  import {
    DEFAULT_SIZES,
    FAVICON_SIZES,
    SIZE_CHOICES,
    type Settings,
    defaultSettings,
    downloadName,
    packMeta,
    toEngineOptions,
  } from '../lib/settings';

  let {
    file,
    onback,
    editing,
    onopenitem,
    onnext,
  }: {
    file: File;
    onback: () => void;
    /** Pictures chosen from the queue panel for the next turn; this icon is kept in the queue first. */
    onnext: (files: File[]) => void;
    /** An icon of the queue that is being edited again, with the settings it was made with. */
    editing?: { id: number; settings: Settings };
    /** Opens another icon of the queue (this one is saved first). */
    onopenitem?: (item: QueueItem) => void;
  } = $props();

  type Backdrop = 'checker' | 'light' | 'dark' | 'gray' | 'custom';
  const BACKDROPS: Backdrop[] = ['checker', 'light', 'dark', 'gray', 'custom'];
  const VIEWS = [
    { id: 'icon', icon: 'view' },
    { id: 'compare', icon: 'compare' },
    { id: 'pixels', icon: 'grid' },
  ] as const;

  // What was chosen last time is the starting point (see storage.ts).
  // (The editor is made anew for every icon, so only the first value of `editing` matters.)
  // svelte-ignore state_referenced_locally
  let settings = $state<Settings>(editing ? structuredClone($state.snapshot(editing.settings)) : loadSettings());
  // The crop frame an icon from the queue comes back with; the effect below clears
  // `settings.crop` until the picture is open, so it is noted here first.
  // svelte-ignore state_referenced_locally
  const startCrop: Rect | null = settings.crop ? { ...settings.crop } : null;
  let opened = $state<Opened>();
  let openFailure = $state('');
  let originalUrl = $state('');

  let backdrop = $state<Backdrop>('checker');
  let customColor = $state('#3b82f6');
  // The surface under the mouse is shown at once; a click keeps it.
  let hovered = $state<Backdrop | null>(null);
  let shown = $derived(hovered ?? backdrop);
  let surface = $derived(shown === 'custom' ? `background:${customColor}` : undefined);
  let view = $state<'icon' | 'compare' | 'pixels'>('icon');
  let converted = $state<Converted>();
  let tiles = $state<{ size: number; url: string }[]>([]);
  let working = $state(false);
  let convertFailure = $state('');

  // The website package: the Apple icon shown, the lines for the page's head.
  let appleUrl = $state('');
  let snippet = $state('');
  let packing = $state(false);
  let copied = $state(false);

  // What the last conversion was made from, so a stale answer (a slider moved
  // again while the engine was busy) is thrown away instead of shown.
  let latest = 0;

  // The icon of the queue this picture is (an icon that was opened from it, or added from here).
  // svelte-ignore state_referenced_locally
  let editId = $state<number | undefined>(editing?.id);
  // Changes to that icon go into the queue on their own, unless this is switched off.
  let autoSave = $state(loadAutoSave());
  // The settings as they were when the queue last got this icon (null until the first conversion of a turn).
  let savedSignature = $state<string | null>(null);
  let signature = $derived(JSON.stringify($state.snapshot(settings)));
  let dirty = $derived(editId !== undefined && savedSignature !== null && signature !== savedSignature);
  let saving = $state(false);

  // The crop frame. It is `settings.crop` only while crop is on and the frame
  // is smaller than the picture; the frame itself is kept while it is off.
  let cropOn = $state(false);
  // The detailed controls are folded away until asked for.
  let advanced = $state(false);
  // Picking the color to remove: the pixel view is shown, without the background
  // removal (or the color would already be gone), and a click on a pixel takes it.
  let picking = $state(false);
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
      opened = await openPicture(bytes, file.name);
      // An icon from the queue comes back with its crop frame.
      frame = startCrop ? { ...startCrop } : fullRect(opened);
      cropOn = startCrop !== null;
    } catch (error) {
      openFailure = error instanceof Error ? error.message : String(error);
    }
  });

  onDestroy(() => {
    if (originalUrl) URL.revokeObjectURL(originalUrl);
    if (appleUrl) URL.revokeObjectURL(appleUrl);
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
    if (!opened) return;
    // The website package has a favicon.ico of fixed sizes, and an Apple icon.
    const favicon = settings.format === 'favicon';
    const options = toEngineOptions({
      ...settings,
      ...(favicon ? { sizes: FAVICON_SIZES } : {}),
      removeBackground: settings.removeBackground && !picking,
    });
    const apple = settings.appleBackground;
    if (!favicon && settings.sizes.length === 0) {
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
        let appleImage: Uint8Array | undefined;
        if (favicon) {
          appleImage = (await convert({ ...options, format: 'png', sizes: [180], flatten: apple })).bytes;
        }
        if (mine !== latest) return;
        converted = result;
        show(iconEntries(result.bytes));
        if (appleUrl) URL.revokeObjectURL(appleUrl);
        appleUrl = appleImage ? URL.createObjectURL(new Blob([appleImage as BlobPart], { type: 'image/png' })) : '';
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

  // The lines for the page's head, for the colors as they are now (the package
  // itself has the exact ones).
  $effect(() => {
    if (settings.format !== 'favicon' || !opened) return;
    const hasSvg = opened.vector && file.name.toLowerCase().endsWith('.svg');
    faviconSnippet(hasSvg, settings.themeColor).then((text) => (snippet = text), () => {});
  });

  async function copySnippet() {
    try {
      await navigator.clipboard.writeText(snippet);
      copied = true;
      setTimeout(() => (copied = false), 1500);
    } catch {
      // No clipboard permission: the text can still be selected by hand.
    }
  }

  // Remember the settings for the next visit.
  // (Not while an icon of the queue is edited: that one has its own settings.)
  $effect(() => {
    if (!editing) saveSettings($state.snapshot(settings));
  });

  function usePreset(preset: Preset) {
    // Keep the crop frame: it belongs to this picture, not to a preset.
    const crop = settings.crop;
    settings = { ...withPreset($state.snapshot(settings), preset), crop };
  }

  // Everything back to how the page starts, the crop frame included.
  function resetSettings() {
    settings = defaultSettings();
  }

  let isDefault = $derived(JSON.stringify($state.snapshot(settings)) === JSON.stringify(defaultSettings()));

  function startPicking() {
    picking = true;
    view = 'pixels';
    document.getElementById('preview-title')?.scrollIntoView({ block: 'nearest' });
  }

  // A click on a pixel: its color is the one to remove, and the removal is on.
  function usePicked(color: string) {
    settings.backgroundColor = color;
    settings.backgroundAuto = false;
    settings.removeBackground = true;
    // After picking, show the icon again; from the plain pixel view, stay there.
    if (picking) view = 'icon';
    picking = false;
    showBackgroundSettings();
  }

  // Show where the color went: open the advanced editor, bring the background
  // settings into view and let them flash once.
  let flashBackground = $state(false);
  async function showBackgroundSettings() {
    advanced = true;
    await tick();
    document.getElementById('background-settings')?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
    flashBackground = false;
    await tick();
    flashBackground = true;
    setTimeout(() => (flashBackground = false), 1600);
  }

  // Leaving the pixel view ends the picking.
  $effect(() => {
    if (view !== 'pixels') picking = false;
  });

  function chooseAspect(choice: AspectChoice) {
    cropAspect = choice;
    const ratio = aspectValue(choice);
    if (ratio !== null && picture) frame = fitAspect(frame, ratio, picture);
  }

  function setFrame(field: keyof Rect, value: number) {
    if (!picture || !Number.isFinite(value)) return;
    frame = clampRect({ ...frame, [field]: value }, picture);
  }

  // What the icon is made of, as the queue keeps it; nothing if there is nothing to keep.
  async function madeForQueue() {
    if (!converted || settings.format === 'favicon') return undefined;
    const format = settings.format;
    const bytes = format === 'ico' ? converted.bytes : (await convert(toEngineOptions(settings, 'icns'))).bytes;
    return { file, settings: $state.snapshot(settings) as Settings, format, bytes, preview: converted.bytes };
  }

  // Keep this icon in the queue and go back to choose the next picture.
  async function addToQueue() {
    const made = await madeForQueue();
    if (!made) return;
    editId = queue.add(file.name, made).id;
  }

  // Save the changes to the icon of the queue that is being edited.
  async function saveEditing(announce = true) {
    if (editId === undefined) return;
    const now = signature;
    saving = true;
    try {
      const made = await madeForQueue();
      if (made) {
        queue.update(editId, made, announce);
        savedSignature = now;
      }
    } finally {
      saving = false;
    }
  }

  // With auto-save, each change is in the queue shortly after the preview has been made again.
  $effect(() => {
    if (editId === undefined || !converted || working) return;
    const now = signature;
    if (savedSignature === null) {
      savedSignature = now;
      return;
    }
    if (!autoSave || now === savedSignature) return;
    const timer = setTimeout(() => void saveEditing(false), 300);
    return () => clearTimeout(timer);
  });

  // What reads the queue (a download, say) first asks for the last changes to be put into it:
  // it waits for the preview to be made again, then saves at once instead of after the pause.
  let idleWaiters: (() => void)[] = [];
  $effect(() => {
    if (!working) idleWaiters.splice(0).forEach((resolve) => resolve());
  });
  const untilIdle = () => (working ? new Promise<void>((resolve) => idleWaiters.push(resolve)) : Promise.resolve());
  onMount(() => {
    queue.setFlusher(async () => {
      if (editId === undefined || !autoSave) return;
      await untilIdle();
      if (dirty) await saveEditing(false);
    });
    return () => queue.setFlusher(null);
  });

  // Before leaving the icon being edited: save its changes (auto-save), or ask when they would be lost.
  async function settle(announce: boolean): Promise<boolean> {
    if (editId === undefined) return true;
    if (autoSave) {
      await saveEditing(announce);
      return true;
    }
    return !dirty || confirm(t('queue.leave_unsaved', { name: queue.find(editId)?.fileName ?? file.name }));
  }

  // The next picture comes: this icon is kept in the queue first (added, or updated).
  async function nextPicture(files: File[]) {
    if (editId !== undefined) {
      if (!(await settle(false))) return;
    } else {
      const made = await madeForQueue();
      if (made) queue.add(file.name, made);
    }
    onnext(files);
  }

  // Going back, or to another icon of the queue, keeps the changes of the one being edited.
  async function leave() {
    if (!(await settle(true))) return;
    onback();
  }

  async function jump(item: QueueItem) {
    if (!(await settle(false))) return;
    onopenitem?.(item);
  }

  async function downloadPngZip() {
    if (!converted) return;
    saveBytes(await pngZip(converted.bytes, stemOf(file.name)), `${stemOf(file.name)}_png.zip`, ZIP_TYPE);
  }

  async function download() {
    if (settings.format === 'favicon') {
      packing = true;
      try {
        const pack = await faviconPack(
          toEngineOptions({ ...$state.snapshot(settings), sizes: FAVICON_SIZES }),
          packMeta(settings),
        );
        snippet = pack.snippet;
        saveBytes(pack.zip, downloadName(file.name, 'favicon'), ZIP_TYPE);
      } catch (error) {
        convertFailure = error instanceof Error ? error.message : String(error);
      } finally {
        packing = false;
      }
      return;
    }
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
  <!-- On a wide window the queue (and its note) is a column to the left of the editor. -->
  <div class="workspace" class:with-queue={queue.items.length > 0}>
  <div class="bar">
    <button type="button" class="quiet" onclick={leave}>← {t('state.back')}</button>
    <span class="file">{file.name}</span>
    <button type="button" class="outline reset" onclick={resetSettings} disabled={isDefault} title={t('editor.reset_hint')}>
      <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3 12a9 9 0 1 0 3-6.7L3 8"/><path d="M3 3v5h5"/></svg>
      {t('editor.reset')}
    </button>
  </div>

  {#if editId !== undefined}
    <div class="editing-note" role="status">
      <span>{t('queue.editing_note', { name: queue.find(editId)?.fileName ?? file.name })}</span>
      <label class="autosave">
        <input type="checkbox" bind:checked={autoSave} onchange={() => saveAutoSave(autoSave)} />
        {t('queue.autosave')}
      </label>
      <span class="save-state" class:pending={dirty || saving}>
        {dirty || saving ? (autoSave ? t('queue.saving') : t('queue.unsaved')) : t('queue.saved')}
      </span>
    </div>
  {/if}

  <div class="main">
  <div class="editor">
    <section class="preview" aria-labelledby="preview-title">
      <h2 id="preview-title">{t('editor.preview')}</h2>

      <div class="views" role="radiogroup" aria-label={t('editor.view')}>
        {#each VIEWS as choice (choice.id)}
          <label class:chosen={view === choice.id}>
            <input type="radio" name="view" value={choice.id} bind:group={view} />
            <Icon name={choice.icon} />
            {t(`editor.view_${choice.id}`)}
          </label>
        {/each}
      </div>

      <div class="preview-body">
      <div class="swatches" role="group" aria-label={t('editor.background')} onpointerleave={() => (hovered = null)}>
        {#each BACKDROPS as choice (choice)}
          {@const name = t(`editor.bg_${choice}`)}
          <label
            class:chosen={backdrop === choice}
            title={name}
            onpointerenter={() => (hovered = choice)}
            onfocusin={() => (hovered = choice)}
            onfocusout={() => (hovered = null)}
          >
            <input type="radio" name="backdrop" value={choice} aria-label={name} bind:group={backdrop} />
            <span class="sw {choice}" style={choice === 'custom' ? `background:${customColor}` : undefined}>
              {#if choice === 'custom'}
                <input
                  type="color"
                  class="sw-color"
                  aria-label={t('editor.bg_custom_pick')}
                  bind:value={customColor}
                  oninput={() => (backdrop = 'custom')}
                />
              {/if}
            </span>
          </label>
        {/each}
      </div>

      <div class="preview-main">
      <div class="stage {shown}" style={surface} aria-live="polite">
        {#if settings.sizes.length === 0}
          <p class="note">{t('editor.no_sizes')}</p>
        {:else if converted && tiles.length > 0 && view === 'pixels'}
          <PixelInspector
            bytes={converted.bytes}
            sizes={tiles.map((tile) => tile.size)}
            {picking}
            onuse={usePicked}
            oncancel={() => (picking = false)}
          />
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
        <ul class="tiles {shown}" style={surface}>
          {#each smaller as tile (tile.size)}
            <li>
              <img src={tile.url} alt="" width={tile.size} height={tile.size} />
              <span>{tile.size}</span>
            </li>
          {/each}
        </ul>
      {/if}
      </div>
      </div>
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
          {#if !editing}<span class="hint">{t('presets.remembered')}</span>{/if}
        </div>
      </fieldset>

      <details class="advanced" bind:open={advanced}>
        <summary>
          <span>{t('advanced.title')}</span>
          <small>{t('advanced.hint')}</small>
        </summary>
        <div class="advanced-body">
      {#if !opened.vector && picture}
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

      {#if settings.format !== 'favicon'}
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
      {/if}

      <fieldset id="background-settings" class:empty={!settings.removeBackground} class:flash={flashBackground}>
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
          <div class="pick-row">
            <button type="button" class="outline" onclick={startPicking} disabled={settings.sizes.length === 0}>
              <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m2 22 1-1h3l9-9"/><path d="M3 21v-3l9-9"/><path d="m15 6 3.4-3.4a2.1 2.1 0 1 1 3 3L18 9l.4.4a2.1 2.1 0 1 1-3 3l-3.8-3.8a2.1 2.1 0 1 1 3-3z"/></svg>
              {t('picker.open')}
            </button>
          </div>
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

        </div>
      </details>
    </section>
  </div>

  <section class="download">
    <fieldset class="formats">
      <legend>{t('download.format')}</legend>
      <label><input type="radio" name="format" value="ico" bind:group={settings.format} /> {t('download.ico')}</label>
      <label><input type="radio" name="format" value="icns" bind:group={settings.format} /> {t('download.icns')}</label>
      <label><input type="radio" name="format" value="favicon" bind:group={settings.format} /> {t('download.favicon')}</label>
    </fieldset>
    {#if settings.format === 'favicon'}
      <div class="site">
        <h3>{t('site.title')}</h3>
        <p class="hint">{t('site.contents', { svg: opened.vector ? t('site.svg') : '' })}</p>
        <label class="field">
          <span>{t('site.name')}</span>
          <input type="text" maxlength="60" bind:value={settings.siteName} />
        </label>
        <p class="hint">{t('site.name_hint')}</p>
        <label class="field">
          <span>{t('site.theme')}</span>
          <input type="color" bind:value={settings.themeColor} />
        </label>
        <label class="field">
          <span>{t('site.apple')}</span>
          <input type="color" bind:value={settings.appleBackground} />
        </label>
        <p class="hint">{t('site.apple_hint')}</p>
        {#if appleUrl}
          <figure class="apple">
            <img src={appleUrl} width="90" height="90" alt="" />
            <figcaption>{t('site.apple_preview')}</figcaption>
          </figure>
        {/if}
        <p class="hint">{t('site.snippet')}</p>
        <pre class="snippet">{snippet}</pre>
        <button type="button" class="quiet" onclick={copySnippet}>{copied ? t('site.copied') : t('site.copy')}</button>
      </div>
    {/if}
    <div class="download-actions">
      <button type="button" class="primary" onclick={download} disabled={!converted || working || packing}>
        <Icon name="download" />
        {packing ? t('site.building') : t('download.button', { name: downloadName(file.name, settings.format) })}
      </button>
      {#if settings.format !== 'favicon'}
        <button type="button" class="outline" onclick={downloadPngZip} disabled={!converted || working}>
          <Icon name="archive" />
          {t('download.png_zip')}
        </button>
      {/if}
      {#if editId !== undefined}
        <button
          type="button"
          class="outline"
          onclick={() => saveEditing()}
          disabled={!converted || working || packing || settings.format === 'favicon'}
          title={settings.format === 'favicon' ? t('queue.add_favicon') : t('queue.add_hint')}
        >
          <Icon name="queueAdd" />
          {t('queue.update')}
        </button>
      {/if}
      <button
        type="button"
        class="outline"
        onclick={addToQueue}
        disabled={!converted || working || packing || settings.format === 'favicon'}
        title={settings.format === 'favicon' ? t('queue.add_favicon') : t('queue.add_hint')}
      >
        <Icon name="queueAdd" />
        {editId !== undefined ? t('queue.add_new') : t('queue.add')}
      </button>
    </div>
    {#if settings.format === 'icns'}<p class="hint">{t('download.icns_note')}</p>{/if}
  </section>
  </div>
  <Queue activeId={editId} onopen={jump} onnext={nextPicture} onpicture={nextPicture} />
  </div>
{/if}
