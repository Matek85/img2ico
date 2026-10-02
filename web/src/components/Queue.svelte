<script lang="ts">
  import { onMount } from 'svelte';
  import { buildZip, describeIcon, extractPng, selectImages } from '../engine/client';
  import { t } from '../i18n';
  import { ICNS_TYPE, ICO_TYPE, ZIP_TYPE, saveBytes } from '../lib/download';
  import { type QueueItem, queue } from '../lib/queue.svelte';
  import { sizesText } from '../lib/queue';
  import { stemOf } from '../lib/batch';
  import Compare from './Compare.svelte';
  import Icon from './Icon.svelte';

  let {
    activeId,
    onopen,
    onnext,
    onpicture,
  }: {
    /** The icon being edited just now, if any. */
    activeId?: number;
    /** Opens an icon of the queue for editing. */
    onopen?: (item: QueueItem) => void;
    /** Chooses the next picture(s), from inside the editor. */
    onnext?: (files: File[]) => void;
    /** Opens a picture made from an icon's image in the editor (and puts it in the queue). */
    onpicture?: (files: File[]) => void;
  } = $props();

  let picker = $state<HTMLInputElement>();

  // Clearing the queue cannot be undone, so it asks first, in place of the button.
  let confirmClear = $state(false);
  let keepButton = $state<HTMLButtonElement>();
  $effect(() => {
    if (confirmClear) keepButton?.focus();
  });
  // A queue that has changed or emptied is not the one that was asked about.
  $effect(() => {
    queue.items.length;
    confirmClear = false;
  });

  let root = $state<HTMLElement>();
  let flash = $state(false);
  let working = $state(false);
  let failure = $state('');

  // Two icons ticked are shown side by side, with a divider as in "Before and after".
  let selected = $state<number[]>([]);
  let backdrop = $state<'checker' | 'light' | 'dark' | 'gray'>('checker');
  const BACKDROPS = ['checker', 'light', 'dark', 'gray'] as const;
  let pair = $derived(selected.map((id) => queue.find(id)).filter((item): item is QueueItem => item !== undefined));
  let comparing = $derived(pair.length === 2 ? pair : null);

  function toggle(id: number) {
    selected = selected.includes(id) ? selected.filter((other) => other !== id) : [...selected.slice(-1), id];
  }

  // When icons were just added or changed, show the queue: scroll to it and let it flash once.
  function announce() {
    root?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
    flash = false;
    requestAnimationFrame(() => {
      flash = true;
      setTimeout(() => (flash = false), 1600);
    });
  }

  // Changes made while the queue is shown (from the editor next to it) ...
  let seen = queue.ticks;
  $effect(() => {
    const now = queue.ticks;
    if (now === seen) return;
    seen = now;
    queue.takeNotice();
    announce();
  });

  // ... and changes made before it was shown (the page the editor went back to).
  onMount(() => {
    if (queue.takeNotice()) announce();
  });

  // --- Picking images out of an icon with several: a small queue of its own ---------

  interface SubImage {
    index: number;
    width: number;
    height: number;
    bits: number;
    format: string;
    url: string;
  }
  let sub = $state<{ id: number; images: SubImage[] } | null>(null);
  let subTicked = $state<number[]>([]);
  let subRoot = $state<HTMLElement>();

  // An icon file's largest image (or the one at `index`) as a picture of its own: the editor makes new sizes from it.
  // (Only for .ico files added as they are; an icon made from a picture is edited from that picture.)
  async function asPicture(item: QueueItem, index?: number) {
    await run(async () => {
      const description = await describeIcon(item.bytes.slice());
      const image =
        index === undefined
          ? description.images.reduce((a, b) => (b.width > a.width || (b.width === a.width && b.bits_per_pixel > a.bits_per_pixel) ? b : a))
          : description.images.find((candidate) => candidate.index === index);
      if (!image) return;
      const png = await extractPng(item.bytes.slice(), image.index);
      const name = `${stemOf(item.fileName)}${index === undefined ? '' : '_' + image.width}_edited.png`;
      onpicture?.([new File([png as BlobPart], name, { type: 'image/png' })]);
    });
  }

  // Only an .ico with more than one image has something to pick from.
  const pickable = (item: QueueItem) => item.format === 'ico' && item.sizes.length > 1;

  function closeSub() {
    sub?.images.forEach((image) => URL.revokeObjectURL(image.url));
    sub = null;
    subTicked = [];
  }

  async function openSub(item: QueueItem) {
    if (sub?.id === item.id) return closeSub();
    closeSub();
    await run(async () => {
      const description = await describeIcon(item.bytes.slice());
      const images: SubImage[] = [];
      for (const image of description.images) {
        const png = await extractPng(item.bytes.slice(), image.index);
        images.push({
          index: image.index,
          width: image.width,
          height: image.height,
          bits: image.bits_per_pixel,
          format: image.format,
          url: URL.createObjectURL(new Blob([png as BlobPart], { type: 'image/png' })),
        });
      }
      sub = { id: item.id, images };
    });
    subRoot?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
  }

  // The chosen images become a new icon of the queue, right after the one they came from.
  async function addImages(chosen: number[]) {
    // A plain copy: the ticked list is reactive state, which cannot be sent to the engine.
    const indices = [...chosen];
    const source = sub ? queue.find(sub.id) : undefined;
    if (!sub || !source || indices.length === 0) return;
    await run(async () => {
      const wanted = sub!.images.filter((image) => indices.includes(image.index));
      const bytes = await selectImages(source.bytes.slice(), indices);
      const largest = wanted.reduce((a, b) => (b.width > a.width ? b : a), wanted[0]);
      const name = wanted.length === 1 ? `${stemOf(source.fileName)}_${wanted[0].width}.ico` : `${stemOf(source.fileName)}_selection.ico`;
      queue.addIcon(
        name,
        new File([bytes as BlobPart], name),
        bytes,
        [...new Set(wanted.map((image) => image.width))].sort((a, b) => a - b),
        await extractPng(source.bytes.slice(), largest.index),
        source.id,
      );
      subTicked = [];
    });
  }

  function tickImage(index: number) {
    subTicked = subTicked.includes(index) ? subTicked.filter((other) => other !== index) : [...subTicked, index];
  }

  // The icon the images came from was removed: its small queue goes with it.
  $effect(() => {
    if (sub && !queue.find(sub.id)) closeSub();
  });

  function chosen() {
    const files = Array.from(picker?.files ?? []);
    if (picker) picker.value = '';
    if (files.length > 0) onnext?.(files);
  }

  async function run(job: () => Promise<void>) {
    working = true;
    failure = '';
    try {
      // The icon being edited next to the queue puts its last changes in first.
      await queue.flush();
      await job();
    } catch (error) {
      failure = error instanceof Error ? error.message : String(error);
    } finally {
      working = false;
    }
  }

  const files = () => queue.items.map((item) => ({ name: item.fileName, bytes: item.bytes }));

  // All icons, each as its own file, in one ZIP.
  const downloadZip = () =>
    run(async () => {
      saveBytes(await buildZip(files()), 'img2ico_icons.zip', ZIP_TYPE);
    });

  const downloadOne = () =>
    run(async () => {
      const item = queue.items[0];
      saveBytes(item.bytes, item.fileName, item.format === 'ico' ? ICO_TYPE : ICNS_TYPE);
    });
</script>

{#if queue.items.length > 0}
  <section class="queue" class:flash bind:this={root} aria-labelledby="queue-title">
    <h2 id="queue-title">{t('queue.title', { count: queue.items.length })}</h2>
    {#if onopen}<p class="hint">{t('queue.jump_hint')}</p>{/if}
    <ol class="queue-list">
      {#each queue.items as item, at (item.id)}
        <li class:active={item.id === activeId} class:fresh={item.id === queue.fresh}>
          {#if queue.items.length > 1}
            <input
              type="checkbox"
              class="pick"
              checked={selected.includes(item.id)}
              aria-label={t('queue.compare_pick', { name: item.fileName })}
              onchange={() => toggle(item.id)}
            />
          {/if}
          <button
            type="button"
            class="open"
            aria-label={t(item.kind === 'icon' ? 'queue.look' : 'queue.edit', { name: item.fileName })}
            aria-current={item.id === activeId ? 'true' : undefined}
            disabled={!onopen || item.id === activeId}
            onclick={() => onopen?.(item)}
          >
            {#if item.thumb}<img src={item.thumb} alt="" width="40" height="40" />{:else}<span class="blank"></span>{/if}
            <span class="what">
              <strong>{item.fileName}</strong>
              <span class="hint">
                {item.id === activeId ? t('queue.editing') + ' · ' : ''}{item.format === 'ico' ? sizesText(item.sizes) : t('queue.icns_sizes')}{item.kind === 'icon' ? ' · ' + t('queue.as_it_is') : ''}
              </span>
            </span>
          </button>
          <div class="item-buttons">
            {#if onpicture && item.kind === 'icon'}
              <button
                type="button"
                class="quiet icon-button"
                title={t('queue.as_picture_hint')}
                aria-label={t('queue.as_picture_for', { name: item.fileName })}
                disabled={working}
                onclick={() => asPicture(item)}
              ><Icon name="edit" /></button>
            {/if}
            {#if pickable(item)}
              <button
                type="button"
                class="quiet icon-button pick-images"
                class:on={sub?.id === item.id}
                aria-expanded={sub?.id === item.id}
                title={t('queue.pick_hint')}
                aria-label={t('queue.pick_for', { name: item.fileName })}
                disabled={working}
                onclick={() => openSub(item)}
              ><Icon name="layers" /></button>
            {/if}
            <button type="button" class="quiet icon-button" title={t('queue.up_title')} aria-label={t('queue.up', { name: item.fileName })} disabled={at === 0} onclick={() => queue.move(item.id, -1)}><Icon name="up" /></button>
            <button type="button" class="quiet icon-button" title={t('queue.down_title')} aria-label={t('queue.down', { name: item.fileName })} disabled={at === queue.items.length - 1} onclick={() => queue.move(item.id, 1)}><Icon name="down" /></button>
            <button type="button" class="quiet icon-button" title={t('queue.remove_title')} aria-label={t('queue.remove', { name: item.fileName })} onclick={() => queue.remove(item.id)}><Icon name="close" /></button>
          </div>
        </li>
      {/each}
    </ol>

    {#if sub}
      {@const source = queue.find(sub.id)}
      <div class="subqueue" bind:this={subRoot}>
        <div class="queue-compare-head">
          <h3>{t('queue.sub_title', { name: source?.fileName ?? '' })}</h3>
          <button type="button" class="quiet" onclick={closeSub}>{t('queue.sub_close')}</button>
        </div>
        <p class="hint">{t('queue.sub_hint')}</p>
        <ul class="sub-images">
          {#each sub.images as image (image.index)}
            <li>
              <input
                type="checkbox"
                checked={subTicked.includes(image.index)}
                aria-label={t('queue.sub_tick', { size: image.width })}
                onchange={() => tickImage(image.index)}
              />
              <img src={image.url} alt="" width={Math.min(image.width, 64)} height={Math.min(image.height, 64)} />
              <span class="what">
                <strong>{image.width} × {image.height}</strong>
                <span class="hint">{t('queue.sub_detail', { format: image.format, bits: image.bits })}</span>
              </span>
              {#if onpicture && source?.kind === 'icon'}
                <button
                  type="button"
                  class="quiet icon-button"
                  title={t('queue.as_picture_this_hint')}
                  aria-label={t('queue.as_picture_for', { name: image.width + ' × ' + image.height })}
                  disabled={working}
                  onclick={() => asPicture(source, image.index)}
                ><Icon name="edit" /></button>
              {/if}
              <button type="button" class="outline" disabled={working} onclick={() => addImages([image.index])}>{t('queue.sub_add')}</button>
            </li>
          {/each}
        </ul>
        <div class="queue-actions">
          <button type="button" disabled={working || subTicked.length < 2} onclick={() => addImages(subTicked)}>
            {subTicked.length >= 2 ? t('queue.sub_add_chosen', { count: subTicked.length }) : t('queue.sub_add_chosen_none')}
          </button>
          <button type="button" class="quiet" onclick={() => (subTicked = sub ? sub.images.map((image) => image.index) : [])}>{t('iconfile.select_all')}</button>
          <button type="button" class="quiet" onclick={() => (subTicked = [])}>{t('iconfile.select_none')}</button>
        </div>
      </div>
    {/if}

    {#if queue.items.length > 1 && !comparing}
      <p class="hint">{t('queue.compare_hint')}</p>
    {/if}
    {#if comparing}
      <div class="queue-compare">
        <div class="queue-compare-head">
          <h3>{t('queue.compare_title', { a: comparing[0].fileName, b: comparing[1].fileName })}</h3>
          <button type="button" class="quiet icon-button" title={t('queue.compare_close')} aria-label={t('queue.compare_close')} onclick={() => (selected = [])}><Icon name="close" /></button>
        </div>
        <div class="backdrops" role="radiogroup" aria-label={t('editor.background')}>
          {#each BACKDROPS as choice (choice)}
            <label class:chosen={backdrop === choice}>
              <input type="radio" name="queue-backdrop" value={choice} bind:group={backdrop} />
              {t(`editor.bg_${choice}`)}
            </label>
          {/each}
        </div>
        <div class="stage {backdrop}">
          {#key comparing[0].id + ':' + comparing[1].id}
            <Compare
              before={comparing[0].thumb}
              after={comparing[1].thumb}
              picture={{ width: 1, height: 1 }}
              crop={null}
              beforeLabel={comparing[0].fileName}
              afterLabel={comparing[1].fileName}
            />
          {/key}
        </div>
      </div>
    {/if}

    {#if onnext}
      <div class="queue-next">
        <button type="button" class="outline" onclick={() => picker?.click()}>+ {t('queue.next_button')}</button>
        <span class="hint">{t('queue.next_hint')}</span>
        <input bind:this={picker} type="file" accept="image/*,.svg,.icns,.zip" multiple hidden onchange={chosen} />
      </div>
    {/if}

    <div class="queue-actions">
      {#if queue.items.length === 1}
        <button type="button" class="primary" onclick={downloadOne}>{t('queue.single', { name: queue.items[0].fileName })}</button>
      {:else}
        <button type="button" class="primary" onclick={downloadZip} disabled={working}>{t('queue.zip')}</button>
      {/if}
      {#if !confirmClear}
        <button type="button" class="quiet" onclick={() => (confirmClear = true)}>{t('queue.clear')}</button>
      {/if}
    </div>
    {#if confirmClear}
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
      <div class="confirm" role="group" aria-label={t('queue.clear')} onkeydown={(e) => e.key === 'Escape' && (confirmClear = false)}>
        <p>{t('queue.clear_ask', { count: queue.items.length })}</p>
        <button type="button" class="danger" onclick={() => { confirmClear = false; queue.clear(); }}>{t('queue.clear_yes')}</button>
        <button type="button" class="outline" bind:this={keepButton} onclick={() => (confirmClear = false)}>{t('queue.clear_no')}</button>
      </div>
    {/if}
    {#if queue.problems.length > 0}
      <div class="findings warn" role="status">
        <p>{t('queue.problems')}</p>
        <ul>
          {#each queue.problems as problem}<li>{problem}</li>{/each}
        </ul>
        <button type="button" class="quiet" onclick={() => queue.setProblems([])}>{t('picker.dismiss')}</button>
      </div>
    {/if}
    {#if failure}<p class="failure" role="alert">{failure}</p>{/if}
    <p class="hint queue-note">{t('queue.note')}</p>
  </section>
{/if}
