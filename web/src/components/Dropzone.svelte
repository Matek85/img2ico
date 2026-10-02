<script lang="ts">
  import Icon from './Icon.svelte';
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
  <svg class="up" viewBox="0 0 24 24" width="44" height="44" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M7 18a4.5 4.5 0 0 1-.6-8.96A6 6 0 0 1 18 8.5a4 4 0 0 1-.5 9.5"/><path d="M12 21v-9m0 0-3 3m3-3 3 3"/></svg>
  <p class="prompt">{dragging ? t('drop.active') : t('drop.prompt')}</p>
  <p class="or">{t('drop.or')}</p>
  <button type="button" class="choose" onclick={() => input?.click()}><Icon name="upload" />{t('drop.choose')}</button>
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
