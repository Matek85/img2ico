<script lang="ts">
  import { onMount } from 'svelte';
  import { buildZip, mergeIcons } from '../engine/client';
  import { t } from '../i18n';
  import { ICNS_TYPE, ICO_TYPE, ZIP_TYPE, saveBytes } from '../lib/download';
  import { type QueueItem, queue } from '../lib/queue.svelte';
  import { sizesText } from '../lib/queue';
  import Compare from './Compare.svelte';

  let {
    activeId,
    onopen,
    onnext,
  }: {
    /** The icon being edited just now, if any. */
    activeId?: number;
    /** Opens an icon of the queue for editing. */
    onopen?: (item: QueueItem) => void;
    /** Chooses the next picture(s), from inside the editor. */
    onnext?: (files: File[]) => void;
  } = $props();

  let picker = $state<HTMLInputElement>();

  let root = $state<HTMLElement>();
  let flash = $state(false);
  let working = $state(false);
  let failure = $state('');
  let notes = $state<string[]>([]);

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

  function chosen() {
    const files = Array.from(picker?.files ?? []);
    if (picker) picker.value = '';
    if (files.length > 0) onnext?.(files);
  }

  async function run(job: () => Promise<void>) {
    working = true;
    failure = '';
    notes = [];
    try {
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

  // The images of all icons in one .ico; what the engine had to skip is shown.
  const downloadCombined = () =>
    run(async () => {
      const merged = await mergeIcons(files());
      notes = merged.warnings.map((warning) => warning.replace(/^Warning:\s*/, ''));
      saveBytes(merged.bytes, 'combined.ico', ICO_TYPE);
    });

  function downloadOne() {
    const item = queue.items[0];
    saveBytes(item.bytes, item.fileName, item.format === 'ico' ? ICO_TYPE : ICNS_TYPE);
  }
</script>

{#if queue.items.length > 0}
  <section class="queue" class:flash bind:this={root} aria-labelledby="queue-title">
    <h2 id="queue-title">{t('queue.title', { count: queue.items.length })}</h2>
    {#if onopen}<p class="hint">{t('queue.jump_hint')}</p>{/if}
    <ol class="queue-list">
      {#each queue.items as item, at (item.id)}
        <li class:active={item.id === activeId}>
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
            aria-label={t('queue.edit', { name: item.fileName })}
            aria-current={item.id === activeId ? 'true' : undefined}
            disabled={!onopen || item.id === activeId}
            onclick={() => onopen?.(item)}
          >
            {#if item.thumb}<img src={item.thumb} alt="" width="40" height="40" />{:else}<span class="blank"></span>{/if}
            <span class="what">
              <strong>{item.fileName}</strong>
              <span class="hint">
                {item.id === activeId ? t('queue.editing') + ' · ' : ''}{item.format === 'ico' ? sizesText(item.sizes) : t('queue.icns_sizes')}
              </span>
            </span>
          </button>
          <div class="item-buttons">
            <button type="button" class="quiet" aria-label={t('queue.up', { name: item.fileName })} disabled={at === 0} onclick={() => queue.move(item.id, -1)}>↑</button>
            <button type="button" class="quiet" aria-label={t('queue.down', { name: item.fileName })} disabled={at === queue.items.length - 1} onclick={() => queue.move(item.id, 1)}>↓</button>
            <button type="button" class="quiet" aria-label={t('queue.remove', { name: item.fileName })} onclick={() => queue.remove(item.id)}>×</button>
          </div>
        </li>
      {/each}
    </ol>

    {#if queue.items.length > 1 && !comparing}
      <p class="hint">{t('queue.compare_hint')}</p>
    {/if}
    {#if comparing}
      <div class="queue-compare">
        <div class="queue-compare-head">
          <h3>{t('queue.compare_title', { a: comparing[0].fileName, b: comparing[1].fileName })}</h3>
          <button type="button" class="quiet" onclick={() => (selected = [])}>{t('queue.compare_close')}</button>
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
        <button type="button" onclick={downloadCombined} disabled={working || !queue.canCombine}>{t('queue.combine')}</button>
      {/if}
      <button type="button" class="quiet" onclick={() => queue.clear()}>{t('queue.clear')}</button>
    </div>
    {#if queue.items.length > 1}
      <p class="hint">{queue.canCombine ? t('queue.combine_hint') : t('queue.combine_icns')}</p>
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
    {#if notes.length > 0}
      <ul class="findings warn">
        {#each notes as note}<li>{note}</li>{/each}
      </ul>
    {/if}
    <p class="hint queue-note">{t('queue.note')}</p>
  </section>
{/if}
