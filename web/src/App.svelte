<script lang="ts">
  import { onMount } from 'svelte';
  import { engineVersion, validateIco } from './engine/client';
  import { formatBytes, t } from './i18n';
  import type { ValidationReport } from './lib/report';

  type View =
    | { kind: 'idle' }
    | { kind: 'working'; name: string }
    | { kind: 'done'; name: string; report: ValidationReport }
    | { kind: 'failed'; reason: string };

  let view = $state<View>({ kind: 'idle' });
  let dragging = $state(false);
  let version = $state('');
  let input = $state<HTMLInputElement>();

  onMount(() => {
    engineVersion().then((v) => (version = v), () => {});
  });

  async function check(file: File) {
    view = { kind: 'working', name: file.name };
    try {
      const bytes = new Uint8Array(await file.arrayBuffer());
      view = { kind: 'done', name: file.name, report: await validateIco(bytes) };
    } catch (error) {
      view = { kind: 'failed', reason: error instanceof Error ? error.message : String(error) };
    }
  }

  function onDrop(event: DragEvent) {
    event.preventDefault();
    dragging = false;
    const file = event.dataTransfer?.files[0];
    if (file) check(file);
  }

  function onChoose() {
    const file = input?.files?.[0];
    if (file) check(file);
    if (input) input.value = '';
  }

  function reset() {
    view = { kind: 'idle' };
  }
</script>

<main>
  <header>
    <h1>{t('app.name')}</h1>
    <p class="tagline">{t('app.tagline')}</p>
  </header>

  {#if view.kind === 'idle' || view.kind === 'failed'}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <section
      class="drop"
      class:active={dragging}
      ondragover={(e) => {
        e.preventDefault();
        dragging = true;
      }}
      ondragleave={() => (dragging = false)}
      ondrop={onDrop}
    >
      <p class="prompt">{dragging ? t('drop.active') : t('drop.prompt')}</p>
      <p class="or">{t('drop.or')}</p>
      <button type="button" onclick={() => input?.click()}>{t('drop.choose')}</button>
      <input bind:this={input} type="file" accept=".ico,.cur,image/x-icon" onchange={onChoose} hidden />
      <p class="hint">{t('drop.hint')}</p>
    </section>
    {#if view.kind === 'failed'}
      <p class="failure" role="alert">{t('state.failed', { reason: view.reason })}</p>
    {/if}
  {:else if view.kind === 'working'}
    <p class="working" role="status">{t('state.working', { name: view.name })}</p>
  {:else}
    {@const report = view.report}
    <section class="result" aria-live="polite">
      <h2 class:ok={report.valid} class:bad={!report.valid}>
        {report.valid ? t('result.valid') : t('result.invalid')}
      </h2>
      <p class="meta">
        {t('result.file', { name: view.name, size: formatBytes(report.bytes) })} ·
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

      <button type="button" onclick={reset}>{t('result.another')}</button>
    </section>
  {/if}

  <footer>
    <p>{t('app.privacy')}</p>
    {#if version}<p class="engine">{t('footer.engine', { version })}</p>{/if}
  </footer>
</main>
