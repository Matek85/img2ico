<script lang="ts">
  import { t } from '../i18n';

  let { onfiles }: { onfiles: (files: File[]) => void } = $props();

  let dragging = $state(false);
  let input = $state<HTMLInputElement>();

  function onDrop(event: DragEvent) {
    event.preventDefault();
    dragging = false;
    const files = Array.from(event.dataTransfer?.files ?? []);
    if (files.length > 0) onfiles(files);
  }

  function onChoose() {
    const files = Array.from(input?.files ?? []);
    if (files.length > 0) onfiles(files);
    if (input) input.value = '';
  }

  // A picture copied to the clipboard (a screenshot, say) can be pasted
  // anywhere on the page.
  function onPaste(event: ClipboardEvent) {
    const file = Array.from(event.clipboardData?.files ?? []).find((f) => f.type.startsWith('image/'));
    if (file) onfiles([file]);
  }
</script>

<svelte:window onpaste={onPaste} />

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
  <input
    bind:this={input}
    type="file"
    accept="image/*,.svg,.icns,.ico,.cur,.zip"
    multiple
    onchange={onChoose}
    hidden
  />
  <p class="hint">{t('drop.formats')}</p>
  <p class="hint">{t('drop.check')}</p>
  <p class="hint">{t('drop.many')}</p>
</section>
