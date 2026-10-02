<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from '../i18n';
  import { hexOf, pixelAt } from '../lib/pick';

  let {
    src,
    size,
    onpick,
    oncancel,
  }: {
    src: string;
    size: { width: number; height: number };
    onpick: (color: string) => void;
    oncancel: () => void;
  } = $props();

  const LOUPE = 9; // pixels shown around the pointer, per side
  let img = $state<HTMLImageElement>();
  let loupe = $state<HTMLCanvasElement>();
  let source: CanvasRenderingContext2D | undefined;
  let failed = $state(false);
  let hover = $state<{ color: string; clear: boolean } | null>(null);

  // The picture is drawn on a canvas of its own, so a single pixel can be read.
  async function ready() {
    try {
      await img!.decode();
      const canvas = document.createElement('canvas');
      canvas.width = size.width;
      canvas.height = size.height;
      const context = canvas.getContext('2d', { willReadFrequently: true });
      if (!context) throw new Error('no canvas');
      context.drawImage(img!, 0, 0, size.width, size.height);
      context.getImageData(0, 0, 1, 1);
      source = context;
    } catch {
      failed = true;
    }
  }

  function read(event: PointerEvent | MouseEvent) {
    if (!source || !img) return null;
    const point = pixelAt(img.getBoundingClientRect(), size, event.clientX, event.clientY);
    if (!point) return null;
    const [r, g, b, a] = source.getImageData(point.x, point.y, 1, 1).data;
    return { point, color: hexOf(r, g, b), clear: a === 0 };
  }

  function move(event: PointerEvent) {
    const found = read(event);
    if (!found) {
      hover = null;
      return;
    }
    hover = { color: found.color, clear: found.clear };
    const out = loupe?.getContext('2d');
    if (out && source) {
      out.imageSmoothingEnabled = false;
      out.clearRect(0, 0, loupe!.width, loupe!.height);
      const half = Math.floor(LOUPE / 2);
      out.drawImage(source.canvas, found.point.x - half, found.point.y - half, LOUPE, LOUPE, 0, 0, loupe!.width, loupe!.height);
    }
  }

  function choose(event: MouseEvent) {
    const found = read(event);
    if (found && !found.clear) onpick(found.color);
  }

  onMount(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') oncancel();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });
</script>

<div class="picker">
  <p class="hint">{t('picker.hint')}</p>
  {#if failed}
    <p class="failure" role="alert">{t('picker.failed')}</p>
  {/if}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions, a11y_click_events_have_key_events -->
  <img
    bind:this={img}
    {src}
    alt={t('editor.source')}
    class:ready={!failed}
    onload={ready}
    onpointermove={move}
    onpointerleave={() => (hover = null)}
    onclick={choose}
  />
  <div class="readout">
    <canvas bind:this={loupe} width={LOUPE * 8} height={LOUPE * 8} aria-hidden="true"></canvas>
    <span class="swatch" style:background={hover && !hover.clear ? hover.color : 'transparent'}></span>
    <span>{hover ? (hover.clear ? t('picker.clear') : hover.color) : ''}</span>
    <button type="button" class="outline" onclick={oncancel}>{t('picker.cancel')}</button>
  </div>
</div>
