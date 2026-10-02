<script lang="ts">
  import { onMount } from 'svelte';
  import Dropzone from './components/Dropzone.svelte';
  import Editor from './components/Editor.svelte';
  import Validator from './components/Validator.svelte';
  import { engineVersion } from './engine/client';
  import { t } from './i18n';

  type View = { kind: 'start' } | { kind: 'editor'; file: File } | { kind: 'validate'; file: File };

  let view = $state<View>({ kind: 'start' });
  let version = $state('');

  onMount(() => {
    engineVersion().then((v) => (version = v), () => {});
  });

  /** An icon file is checked; anything else is a picture to make an icon from. */
  function choose(file: File) {
    view = /\.(ico|cur)$/i.test(file.name) ? { kind: 'validate', file } : { kind: 'editor', file };
  }

  function back() {
    view = { kind: 'start' };
  }
</script>

<main class:wide={view.kind === 'editor'}>
  <header>
    <h1>{t('app.name')}</h1>
    <p class="tagline">{t('app.tagline')}</p>
  </header>

  {#if view.kind === 'start'}
    <Dropzone onfile={choose} />
  {:else if view.kind === 'editor'}
    {#key view.file}
      <Editor file={view.file} onback={back} />
    {/key}
  {:else}
    {#key view.file}
      <Validator file={view.file} onback={back} />
    {/key}
  {/if}

  <footer>
    <p>{t('app.privacy')}</p>
    {#if version}<p class="engine">{t('footer.engine', { version })}</p>{/if}
  </footer>
</main>
