<script lang="ts">
  import { t } from '../i18n';

  // Files dropped anywhere on the page go to `onfiles`; while they are dragged over it,
  // a veil says where they will end up.
  let { onfiles, label }: { onfiles: (files: File[]) => void; label: string } = $props();

  let depth = $state(0);

  const carriesFiles = (event: DragEvent) => Array.from(event.dataTransfer?.types ?? []).includes('Files');

  function enter(event: DragEvent) {
    if (carriesFiles(event)) depth += 1;
  }

  function leave(event: DragEvent) {
    if (carriesFiles(event)) depth = Math.max(0, depth - 1);
  }

  function over(event: DragEvent) {
    if (carriesFiles(event)) event.preventDefault();
  }

  function drop(event: DragEvent) {
    if (!carriesFiles(event)) return;
    event.preventDefault();
    depth = 0;
    const files = Array.from(event.dataTransfer?.files ?? []);
    if (files.length > 0) onfiles(files);
  }
</script>

<svelte:window ondragenter={enter} ondragleave={leave} ondragover={over} ondrop={drop} />

{#if depth > 0}
  <div class="drop-veil" aria-hidden="true">
    <p>{label}</p>
  </div>
{/if}
