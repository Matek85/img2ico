<script lang="ts">
  import { onMount } from 'svelte';
  import { validateIco } from '../engine/client';
  import { formatBytes, t } from '../i18n';
  import type { ValidationReport } from '../lib/report';

  let { file, onback }: { file: File; onback: () => void } = $props();

  let report = $state<ValidationReport>();
  let failure = $state('');

  onMount(async () => {
    try {
      report = await validateIco(new Uint8Array(await file.arrayBuffer()));
    } catch (error) {
      failure = error instanceof Error ? error.message : String(error);
    }
  });
</script>

{#if failure}
  <p class="failure" role="alert">{t('state.failed', { reason: failure })}</p>
  <button type="button" onclick={onback}>{t('state.back')}</button>
{:else if !report}
  <p class="working" role="status">{t('state.working', { name: file.name })}</p>
{:else}
  <section class="result" aria-live="polite">
    <h2 class:ok={report.valid} class:bad={!report.valid}>
      {report.valid ? t('result.valid') : t('result.invalid')}
    </h2>
    <p class="meta">
      {t('result.file', { name: file.name, size: formatBytes(report.bytes) })} ·
      {t('result.images', { count: report.images.length })}
    </p>

    {#if report.images.length > 0}
      <ul class="images">
        {#each report.images as image (image.index)}
          <li>
            {t('image.format', {
              width: image.width,
              height: image.height,
              format: image.format,
              bits: image.bits_per_pixel,
            })}
          </li>
        {/each}
      </ul>
    {/if}

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

    <button type="button" onclick={onback}>{t('state.back')}</button>
  </section>
{/if}
