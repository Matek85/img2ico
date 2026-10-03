<script lang="ts">
  import { onMount } from 'svelte';
  import Dropzone from './components/Dropzone.svelte';
  import Editor from './components/Editor.svelte';
  import IconFile from './components/IconFile.svelte';
  import Queue from './components/Queue.svelte';
  import { type QueueItem, queue } from './lib/queue.svelte';
  import { type Settings, toEngineOptions } from './lib/settings';
  import { loadSettings } from './lib/storage';
  import {
    closePicture,
    convert,
    describeIcon,
    engineVersion,
    extractPng,
    openPicture,
    openZip,
    readZipFile,
  } from './engine/client';
  import { t } from './i18n';
  import { type BatchItem, MAX_BATCH, baseName, isIconName, isPictureName, isZipName } from './lib/batch';
  import { explain } from './lib/messages';
  import { shortName } from './lib/names';

  type View =
    | { kind: 'start' }
    | { kind: 'editor'; file: File; editing?: { id: number; settings: Settings } }
    | { kind: 'validate'; file: File }
    | { kind: 'filling' };

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
    document.body.classList.toggle('wide', view.kind === 'editor');
    document.body.classList.toggle('working', view.kind !== 'start');
    document.body.classList.toggle('queue-side', view.kind === 'editor' && queue.items.length > 0);
  });

  let problem = $state('');

  /**
   * What was dropped decides what happens: one icon file is looked into; one
   * picture is opened in the editor; a ZIP, or several pictures, are turned into
   * icons all at once and put in the queue, where each can be opened and changed.
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
        await queueFiles(files);
      }
    } catch (error) {
      problem = t('state.failed', { reason: explain(error) });
    }
  }

  // Several files at once: pictures become icons in the queue, .ico files go in as they are.
  async function queueFiles(files: File[]) {
    const pictures = files.filter((file) => isQueueName(file.name));
    if (pictures.length === 0) {
      problem = t('batch.none');
      // A queue that is shown (beside the editor) says so, too: the start page is not.
      if (queue.items.length > 0) queue.setProblems([t('batch.none')]);
      return;
    }
    const notes = pictures.length < files.length ? [t('batch.ignored', { count: files.length - pictures.length })] : [];
    const items = pictures.map((file) => ({
      name: file.name,
      load: async () => new Uint8Array(await file.arrayBuffer()),
    }));
    await fillQueue(items, notes);
  }

  // "Add the next picture": whatever is chosen is in the queue at once (even a single picture),
  // and the first picture opens in the editor as the icon being edited.
  async function chooseNext(files: File[]) {
    problem = '';
    try {
      if (files.length === 1 && isZipName(files[0].name)) {
        await chooseZip(files[0]);
      } else {
        await queueFiles(files);
      }
    } catch (error) {
      problem = t('state.failed', { reason: explain(error) });
    }
  }

  async function chooseZip(zip: File) {
    const listed = await openZip(new Uint8Array(await zip.arrayBuffer()));
    const pictures = listed.filter((entry) => isQueueName(entry.name));
    if (pictures.length === 0) {
      problem = t('batch.empty_zip');
      return;
    }
    const items: BatchItem[] = pictures.map((entry) => ({
      name: entry.name,
      load: () => readZipFile(entry.index),
    }));
    const skipped = listed.length - pictures.length;
    await fillQueue(items, skipped > 0 ? [t('batch.ignored', { count: skipped })] : []);
  }

  // Pictures become icons for the queue; .ico files go in as they are.
  const isQueueName = (name: string) => isPictureName(name) || isIconName(name);

  // How far the making of the queue has come.
  let filling = $state<{ done: number; total: number; name: string } | null>(null);

  /**
   * Turns each picture into an icon with the settings used last and puts it in
   * the queue; then the first of them is opened in the editor. What failed is
   * listed with the queue.
   */
  async function fillQueue(all: BatchItem[], notes: string[]) {
    const items = all.slice(0, MAX_BATCH);
    const problems = all.length > MAX_BATCH ? [...notes, t('batch.too_many', { max: MAX_BATCH })] : [...notes];
    // Icon files only (nothing to convert) are added where you are: the editor stays open on its picture.
    const onlyIcons = items.every((item) => isIconName(item.name));
    if (!onlyIcons) view = { kind: 'filling' };
    const remembered = loadSettings();
    const settings: Settings = { ...remembered, crop: null, format: remembered.format === 'icns' ? 'icns' : 'ico' };
    let first: QueueItem | undefined;
    try {
      for (const [at, item] of items.entries()) {
        filling = { done: at, total: items.length, name: item.name };
        try {
          const bytes = await item.load();
          if (isIconName(item.name)) {
            // An icon file stays as it is; the engine tells its images (and fails on a damaged one).
            const description = await describeIcon(bytes.slice());
            const largest = description.images.reduce((a, b) => (b.width > a.width ? b : a), description.images[0]);
            const sizes = [...new Set(description.images.map((image) => image.width))].sort((a, b) => a - b);
            queue.addIcon(
              baseName(item.name),
              new File([bytes as BlobPart], baseName(item.name)),
              bytes,
              sizes,
              largest ? await extractPng(bytes.slice(), largest.index) : null,
            );
            continue;
          }
          await openPicture(bytes.slice(), item.name);
          const preview = await convert(toEngineOptions(settings, 'ico'));
          const out = settings.format === 'icns' ? (await convert(toEngineOptions(settings, 'icns'))).bytes : preview.bytes;
          const added = queue.add(baseName(item.name), {
            file: new File([bytes as BlobPart], baseName(item.name)),
            settings,
            format: settings.format === 'icns' ? 'icns' : 'ico',
            bytes: out,
            preview: preview.bytes,
          });
          first ??= added;
        } catch (error) {
          problems.push(`${item.name}: ${explain(error)}`);
        }
      }
    } finally {
      // (The editor's own picture is still open in the engine when only icon files were added.)
      if (!onlyIcons) await closePicture().catch(() => {});
      filling = null;
    }
    queue.setProblems(problems);
    if (first) {
      // The first picture is opened in the editor.
      openFromQueue(first);
    } else if (queue.items.length === 0) {
      // Nothing could be added: say why where it can be seen.
      view = { kind: 'start' };
      problem = problems.length > 0 ? problems.join(' ') : t('batch.none');
    } else if (!onlyIcons) {
      view = { kind: 'start' };
    }
  }

  function back() {
    view = { kind: 'start' };
  }

  // An icon of the queue is opened again, with the picture and settings it was made with.
  function openFromQueue(item: QueueItem) {
    // An .ico file that was added as it is is looked into (checked, taken apart); a picture is edited.
    view =
      item.kind === 'icon' || !item.settings
        ? { kind: 'validate', file: item.file }
        : { kind: 'editor', file: item.file, editing: { id: item.id, settings: $state.snapshot(item.settings) as Settings } };
    window.scrollTo({ top: 0 });
  }

  // Closing or reloading the page would lose the queue: the browser asks first.
  $effect(() => {
    if (queue.items.length === 0) return;
    const warn = (event: BeforeUnloadEvent) => {
      event.preventDefault();
      event.returnValue = '';
    };
    window.addEventListener('beforeunload', warn);
    return () => window.removeEventListener('beforeunload', warn);
  });
</script>

{#if view.kind === 'start'}
  <Dropzone onfiles={choose} />
  {#if problem}<p class="failure" role="alert">{problem}</p>{/if}
  {#if queue.items.length > 0}<p class="hint next">{t('queue.next')}</p>{/if}
  <Queue onopen={openFromQueue} onpicture={chooseNext} />
{:else if view.kind === 'editor'}
  {#key view}
    <Editor file={view.file} editing={view.editing} onopenitem={openFromQueue} onnext={chooseNext} onback={back} />
  {/key}
{:else if view.kind === 'filling'}
  <section class="filling" role="status">
    <h2>{t('queue.filling')}</h2>
    {#if filling}
      <progress max={filling.total} value={filling.done} aria-label={t('batch.progress')}></progress>
      <p class="hint">{t('batch.working', { done: filling.done + 1, total: filling.total, name: shortName(filling.name) })}</p>
    {/if}
  </section>
{:else}
  {#key view.file}
    <IconFile file={view.file} onback={back} onpicture={chooseNext} />
  {/key}
  <Queue onopen={openFromQueue} onpicture={chooseNext} />
{/if}
