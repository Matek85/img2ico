<script lang="ts">
  import { onMount } from 'svelte';
  import { buildZip, mergeIcons } from '../engine/client';
  import { t } from '../i18n';
  import { ICNS_TYPE, ICO_TYPE, ZIP_TYPE, saveBytes } from '../lib/download';
  import { queue } from '../lib/queue.svelte';
  import { sizesText } from '../lib/queue';

  let root = $state<HTMLElement>();
  let flash = $state(false);
  let working = $state(false);
  let failure = $state('');
  let notes = $state<string[]>([]);

  // When icons were just added, show the queue: scroll to it and let it flash once.
  onMount(() => {
    if (!queue.takeNotice()) return;
    root?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
    flash = true;
    setTimeout(() => (flash = false), 1600);
  });

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
    <ol class="queue-list">
      {#each queue.items as item, at (item.id)}
        <li>
          {#if item.thumb}<img src={item.thumb} alt="" width="40" height="40" />{:else}<span class="blank"></span>{/if}
          <div class="what">
            <strong>{item.fileName}</strong>
            <span class="hint">
              {item.format === 'ico' ? sizesText(item.sizes) : t('queue.icns_sizes')}
            </span>
          </div>
          <div class="item-buttons">
            <button type="button" class="quiet" aria-label={t('queue.up', { name: item.fileName })} disabled={at === 0} onclick={() => queue.move(item.id, -1)}>↑</button>
            <button type="button" class="quiet" aria-label={t('queue.down', { name: item.fileName })} disabled={at === queue.items.length - 1} onclick={() => queue.move(item.id, 1)}>↓</button>
            <button type="button" class="quiet" aria-label={t('queue.remove', { name: item.fileName })} onclick={() => queue.remove(item.id)}>×</button>
          </div>
        </li>
      {/each}
    </ol>

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
    {#if failure}<p class="failure" role="alert">{failure}</p>{/if}
    {#if notes.length > 0}
      <ul class="findings warn">
        {#each notes as note}<li>{note}</li>{/each}
      </ul>
    {/if}
  </section>
{/if}
