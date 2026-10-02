<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import {
    type IconDescription,
    describeIcon,
    extractPng,
    mergeIcons,
    pngZip,
    selectImages,
    validateIco,
  } from '../engine/client';
  import { formatBytes, t } from '../i18n';
  import Icon from './Icon.svelte';
  import { baseName, stemOf } from '../lib/batch';
  import { ICO_TYPE, PNG_TYPE, ZIP_TYPE, saveBytes } from '../lib/download';
  import { queue } from '../lib/queue.svelte';
  import type { ValidationReport } from '../lib/report';

  let {
    file,
    onback,
    onpicture,
  }: {
    file: File;
    onback: () => void;
    /** Opens a picture made from one of the images in the editor (it is put in the queue). */
    onpicture: (files: File[]) => void;
  } = $props();

  let bytes = new Uint8Array();
  let report = $state<ValidationReport>();
  let description = $state<IconDescription>();
  let thumbnails = $state<Record<number, string>>({});
  let failure = $state('');

  let selected = $state<number[]>([]);
  let working = $state(false);
  let actionFailure = $state('');

  // Other icon files to combine this one with.
  // (Not a deep $state: the engine worker cannot be sent proxies.)
  let others = $state.raw<{ name: string; bytes: Uint8Array }[]>([]);
  let mergeNotes = $state<string[]>([]);
  let mergedSummary = $state('');
  let input = $state<HTMLInputElement>();

  onMount(async () => {
    try {
      bytes = new Uint8Array(await file.arrayBuffer());
      // validateIco hands its bytes over to the engine, so it gets a copy.
      report = await validateIco(bytes.slice());
    } catch (error) {
      failure = messageOf(error);
      return;
    }
    // A file too damaged to describe still has its validation report.
    try {
      description = await describeIcon(bytes);
      selected = description.images.map((image) => image.index);
      for (const image of description.images) {
        extractPng(bytes, image.index).then(
          (png) => {
            thumbnails[image.index] = URL.createObjectURL(new Blob([png as BlobPart], { type: PNG_TYPE }));
          },
          () => {},
        );
      }
    } catch {
      description = undefined;
    }
  });

  onDestroy(() => {
    for (const url of Object.values(thumbnails)) URL.revokeObjectURL(url);
  });

  function messageOf(error: unknown): string {
    return error instanceof Error ? error.message : String(error);
  }

  async function run(action: () => Promise<void>) {
    working = true;
    actionFailure = '';
    try {
      await action();
    } catch (error) {
      actionFailure = messageOf(error);
    } finally {
      working = false;
    }
  }

  const savePng = (index: number, width: number) =>
    run(async () => {
      saveBytes(await extractPng(bytes, index), `${stemOf(file.name)}_${width}.png`, PNG_TYPE);
    });

  const savePngZip = () =>
    run(async () => {
      saveBytes(await pngZip(bytes, stemOf(file.name)), `${stemOf(file.name)}_png.zip`, ZIP_TYPE);
    });

  const saveSelected = () =>
    run(async () => {
      const indices = [...selected].sort((a, b) => a - b);
      saveBytes(await selectImages(bytes, indices), `${stemOf(file.name)}_selected.ico`, ICO_TYPE);
    });

  // One image of this file as a picture of its own (the largest one when no index is given).
  const asPicture = (index?: number) =>
    run(async () => {
      const images = description?.images ?? [];
      const image =
        index === undefined
          ? images.reduce((a, b) => (b.width > a.width || (b.width === a.width && b.bits_per_pixel > a.bits_per_pixel) ? b : a))
          : images.find((candidate) => candidate.index === index);
      if (!image) return;
      const png = await extractPng(bytes, image.index);
      const name = `${stemOf(file.name)}${index === undefined ? '' : '_' + image.width}_edited.png`;
      onpicture([new File([png as BlobPart], name, { type: PNG_TYPE })]);
    });

  // This icon as it is, or only the chosen images of it as one icon, into the queue.
  const addToQueue = (indices?: number[]) =>
    run(async () => {
      const images = description?.images ?? [];
      const wanted = indices ? images.filter((image) => indices.includes(image.index)) : images;
      if (wanted.length === 0) return;
      const made = indices ? await selectImages(bytes, [...indices].sort((a, b) => a - b)) : bytes.slice();
      const largest = wanted.reduce((a, b) => (b.width > a.width ? b : a));
      const name = indices && indices.length < images.length ? `${stemOf(file.name)}_selected.ico` : baseName(file.name);
      queue.addIcon(
        name,
        new File([made as BlobPart], name),
        made,
        [...new Set(wanted.map((image) => image.width))].sort((a, b) => a - b),
        await extractPng(bytes, largest.index),
      );
    });

  async function addOthers() {
    const files = Array.from(input?.files ?? []);
    if (input) input.value = '';
    for (const added of files) {
      others = [...others, { name: added.name, bytes: new Uint8Array(await added.arrayBuffer()) }];
    }
    mergedSummary = '';
    mergeNotes = [];
  }

  const combine = () =>
    run(async () => {
      const result = await mergeIcons([{ name: file.name, bytes }, ...others]);
      mergeNotes = result.warnings;
      mergedSummary = t('merge.done', { count: result.sizes.length, sizes: result.sizes.join(', ') });
      saveBytes(result.bytes, `${stemOf(file.name)}_merged.ico`, ICO_TYPE);
    });
</script>

{#if failure}
  <p class="failure" role="alert">{t('state.failed', { reason: failure })}</p>
  <button type="button" onclick={onback}>{t('state.back')}</button>
{:else if !report}
  <p class="working" role="status">{t('state.working', { name: file.name })}</p>
{:else}
  <div class="bar">
    <button type="button" class="quiet" onclick={onback}>← {t('state.back')}</button>
    <span class="file">{file.name}</span>
  </div>

  <section class="result" aria-live="polite">
    <h2 class:ok={report.valid} class:bad={!report.valid}>
      {report.valid ? t('result.valid') : t('result.invalid')}
    </h2>
    <p class="meta">
      {t('result.file', { name: file.name, size: formatBytes(report.bytes) })} ·
      {t('result.images', { count: report.images.length })}
    </p>

    {#each [{ title: 'result.errors', list: report.errors, cls: 'bad' }, { title: 'result.warnings', list: report.warnings, cls: 'warn' }] as group (group.title)}
      {#if group.list.length > 0}
        <h3>{t(group.title)}</h3>
        <ul class="findings {group.cls}">
          {#each group.list as finding}
            <li>
              {finding.image === null
                ? finding.message
                : t('result.about_image', { number: finding.image + 1, message: finding.message })}
            </li>
          {/each}
        </ul>
      {/if}
    {/each}
  </section>

  {#if description && description.images.length > 0}
    <section class="images-panel" aria-labelledby="images-title">
      <h2 id="images-title">{t('iconfile.images')}</h2>
      <ul class="cards">
        {#each description.images as image (image.index)}
          <li class="card">
            <label class="pick">
              <input type="checkbox" value={image.index} bind:group={selected} />
              <span class="size">{image.width} × {image.height}</span>
            </label>
            <div class="thumb checker">
              {#if thumbnails[image.index]}
                <img
                  src={thumbnails[image.index]}
                  alt=""
                  style="width:{Math.min(image.width, 96)}px;height:{Math.min(image.height, 96)}px"
                />
              {/if}
            </div>
            <p class="facts">
              {image.format} · {t('iconfile.bits', { bits: image.bits_per_pixel })} · {formatBytes(image.bytes)}
            </p>
            <p class="facts">{t('iconfile.alpha', { alpha: image.alpha })}</p>
            <button type="button" class="quiet" disabled={working} onclick={() => savePng(image.index, image.width)}>
              <Icon name="download" /> {t('iconfile.save_png')}
            </button>
            <button type="button" class="quiet" disabled={working} title={t('queue.as_picture_this_hint')} onclick={() => asPicture(image.index)}>
              <Icon name="edit" /> {t('queue.as_picture')}
            </button>
          </li>
        {/each}
      </ul>

      {#if description.missing_windows_sizes.length > 0}
        <p class="hint">
          {t('iconfile.missing', { sizes: description.missing_windows_sizes.join(', ') })}
        </p>
      {/if}

      <div class="actions">
        <button
          type="button"
          class="primary"
          disabled={working || selected.length === 0}
          onclick={saveSelected}
        >
          <Icon name="download" /> {t('iconfile.save_selected', { count: selected.length })}
        </button>
        <button type="button" disabled={working} onclick={savePngZip}><Icon name="archive" /> {t('iconfile.save_zip')}</button>
        <button type="button" class="quiet" onclick={() => (selected = description!.images.map((i) => i.index))}>
          <Icon name="checkAll" /> {t('iconfile.select_all')}
        </button>
        <button type="button" class="quiet" onclick={() => (selected = [])}><Icon name="checkNone" /> {t('iconfile.select_none')}</button>
      </div>
      <div class="actions">
        <button type="button" class="outline" disabled={working} title={t('queue.as_picture_hint')} onclick={() => asPicture()}>
          <Icon name="edit" /> {t('iconfile.as_picture_largest')}
        </button>
        <button type="button" class="outline" disabled={working} onclick={() => addToQueue()}><Icon name="queueAdd" /> {t('iconfile.add_all')}</button>
        <button type="button" class="outline" disabled={working || selected.length === 0} onclick={() => addToQueue([...selected])}>
          <Icon name="queueAdd" /> {t('iconfile.add_selected', { count: selected.length })}
        </button>
      </div>
    </section>
  {/if}

  <section class="merge-panel" aria-labelledby="merge-title">
    <h2 id="merge-title">{t('merge.title')}</h2>
    <p class="hint">{t('merge.hint')}</p>
    <ul class="others">
      <li>{file.name} <span class="hint">({t('merge.this')})</span></li>
      {#each others as other, i (i)}
        <li>
          {other.name}
          <button type="button" class="quiet" onclick={() => (others = others.filter((_, at) => at !== i))}>{t('merge.remove')}</button>
        </li>
      {/each}
    </ul>
    <div class="actions">
      <button type="button" onclick={() => input?.click()}><Icon name="plus" /> {t('merge.add')}</button>
      <input bind:this={input} type="file" accept=".ico" multiple hidden onchange={addOthers} />
      <button type="button" class="primary" disabled={working || others.length === 0} onclick={combine}>
        <Icon name="download" /> {t('merge.combine')}
      </button>
    </div>
    {#if mergedSummary}<p class="ok" role="status">{mergedSummary}</p>{/if}
    {#if mergeNotes.length > 0}
      <ul class="findings warn">
        {#each mergeNotes as note}<li>{note}</li>{/each}
      </ul>
    {/if}
  </section>

  {#if actionFailure}<p class="failure" role="alert">{actionFailure}</p>{/if}
{/if}
