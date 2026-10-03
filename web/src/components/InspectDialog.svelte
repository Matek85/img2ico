<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from '../i18n';
  import { shortName } from '../lib/names';
  import IconFile from './IconFile.svelte';

  // The look into an icon file of the queue, over the page: the visitor stays where they were (the editor keeps its
  // picture and settings behind it). Closed by the button, Esc or a click beside it.
  let { file, onclose }: { file: File; onclose: () => void } = $props();
  let dialog = $state<HTMLDialogElement>();

  onMount(() => dialog?.showModal());
</script>

<dialog
  class="site-dialog inspect-dialog"
  bind:this={dialog}
  aria-label={t('queue.look', { name: shortName(file.name) })}
  onclose={onclose}
  onclick={(event) => event.target === dialog && dialog?.close()}
>
  <IconFile {file} inspect onback={() => dialog?.close()} onpicture={() => {}} />
</dialog>
