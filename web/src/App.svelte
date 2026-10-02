<script lang="ts">
  import { onMount } from 'svelte';
  import Dropzone from './components/Dropzone.svelte';
  import Editor from './components/Editor.svelte';
  import IconFile from './components/IconFile.svelte';
  import { engineVersion, openZip, readZipFile } from './engine/client';
  import { t } from './i18n';
  import { type BatchItem, MAX_BATCH, baseName, isIconName, isPictureName, isZipName, stemOf } from './lib/batch';

  type View =
    | { kind: 'start' }
    | { kind: 'editor'; file: File }
    | { kind: 'validate'; file: File }
    | { kind: 'batch'; file: File; items: BatchItem[]; archive: string; notes: string[] };

  let view = $state<View>({ kind: 'start' });

  onMount(() => {
    // The page around the app (heading, text, footer) is plain HTML in
    // index.html. Start the engine once the page has settled, so the first
    // picture is converted without waiting for it, and show its version.
    const start = () =>
      engineVersion().then(
        (version) => {
          const line = document.getElementById('engine');
          if (line) {
            line.textContent = t('footer.engine', { version });
            line.hidden = false;
          }
        },
        () => {},
      );
    if ('requestIdleCallback' in window) requestIdleCallback(() => void start(), { timeout: 3000 });
    else setTimeout(() => void start(), 1000);
  });

  // The explanatory text belongs to the start page only, and a wide editor
  // widens the whole page.
  $effect(() => {
    const about = document.getElementById('about');
    if (about) about.hidden = view.kind !== 'start';
    document.body.classList.toggle('wide', view.kind === 'editor' || view.kind === 'batch');
  });

  let problem = $state('');

  /**
   * What was dropped decides what happens: one icon file is looked into; one
   * picture is turned into an icon; a ZIP, or several pictures, are turned into
   * icons all at once.
   */
  async function choose(files: File[]) {
    problem = '';
    try {
      if (files.length === 1 && isIconName(files[0].name)) {
        view = { kind: 'validate', file: files[0] };
      } else if (files.length === 1 && isZipName(files[0].name)) {
        await chooseZip(files[0]);
      } else if (files.length === 1) {
        view = { kind: 'editor', file: files[0] };
      } else {
        const pictures = files.filter((file) => isPictureName(file.name));
        if (pictures.length === 0) {
          problem = t('batch.none');
          return;
        }
        const notes = pictures.length < files.length ? [t('batch.ignored', { count: files.length - pictures.length })] : [];
        const items = pictures.map((file) => ({
          name: file.name,
          load: async () => new Uint8Array(await file.arrayBuffer()),
        }));
        view = { kind: 'batch', file: pictures[0], items: items.slice(0, MAX_BATCH), archive: 'icons', notes: tooMany(items.length, notes) };
      }
    } catch (error) {
      problem = t('state.failed', { reason: error instanceof Error ? error.message : String(error) });
    }
  }

  function tooMany(count: number, notes: string[]): string[] {
    return count > MAX_BATCH ? [...notes, t('batch.too_many', { max: MAX_BATCH })] : notes;
  }

  async function chooseZip(zip: File) {
    const listed = await openZip(new Uint8Array(await zip.arrayBuffer()));
    const pictures = listed.filter((entry) => isPictureName(entry.name));
    if (pictures.length === 0) {
      problem = t('batch.empty_zip');
      return;
    }
    const items: BatchItem[] = pictures.slice(0, MAX_BATCH).map((entry) => ({
      name: entry.name,
      load: () => readZipFile(entry.index),
    }));
    const first = new File([await items[0].load() as BlobPart], baseName(items[0].name));
    const skipped = listed.length - pictures.length;
    const notes = skipped > 0 ? [t('batch.ignored', { count: skipped })] : [];
    view = { kind: 'batch', file: first, items, archive: stemOf(zip.name), notes: tooMany(pictures.length, notes) };
  }

  function back() {
    view = { kind: 'start' };
  }
</script>

{#if view.kind === 'start'}
  <Dropzone onfiles={choose} />
  {#if problem}<p class="failure" role="alert">{problem}</p>{/if}
{:else if view.kind === 'editor'}
  {#key view.file}
    <Editor file={view.file} onback={back} />
  {/key}
{:else if view.kind === 'batch'}
  {#key view.items}
    <Editor file={view.file} batch={{ items: view.items, archive: view.archive, notes: view.notes }} onback={back} />
  {/key}
{:else}
  {#key view.file}
    <IconFile file={view.file} onback={back} />
  {/key}
{/if}
