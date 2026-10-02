<script lang="ts">
  import { iconPixels } from '../engine/client';
  import type { Pixels } from '../engine/protocol';
  import { t } from '../i18n';
  import {
    MAX_ZOOM,
    cellAt,
    fitZoom,
    hexOf,
    moveCell,
    opacityPercent,
    pixelAt,
  } from '../lib/pixels';

  let { bytes, sizes }: { bytes: Uint8Array; sizes: number[] } = $props();

  // Which image is looked at: the largest until another is chosen.
  let picked = $state<number | null>(null);
  let index = $derived(Math.min(picked ?? sizes.length - 1, sizes.length - 1));
  let pixels = $state<Pixels>();
  let failure = $state('');
  let zoom = $state(1);
  let grid = $state(true);
  let width = $state(0);

  let canvas = $state<HTMLCanvasElement>();
  /** The pixel under the pointer, and the one that was clicked or reached by the keys. */
  let hover = $state<{ x: number; y: number } | null>(null);
  let pinned = $state<{ x: number; y: number } | null>(null);
  let shown = $derived(pinned ?? hover);

  // A new file (a setting changed) or a new choice of image: read its pixels.
  $effect(() => {
    const chosen = index;
    let current = true;
    iconPixels(bytes, chosen).then(
      (result) => {
        if (!current) return;
        const first = !pixels || pixels.width !== result.width;
        pixels = result;
        failure = '';
        if (first) zoom = fitZoom(width, result.width);
        if (pinned && (pinned.x >= result.width || pinned.y >= result.height)) pinned = null;
      },
      (error) => {
        if (current) failure = error instanceof Error ? error.message : String(error);
      },
    );
    return () => {
      current = false;
    };
  });

  // Draw the exact pixels; the browser scales the canvas without smoothing.
  $effect(() => {
    if (!pixels || !canvas) return;
    canvas.width = pixels.width;
    canvas.height = pixels.height;
    const data = new Uint8ClampedArray(pixels.rgba);
    canvas.getContext('2d')?.putImageData(new ImageData(data, pixels.width, pixels.height), 0, 0);
  });

  // The chosen pixel may lie outside a smaller image that has just been loaded.
  let inside = $derived(
    pixels !== undefined && shown !== null && shown.x < pixels.width && shown.y < pixels.height,
  );
  let info = $derived(pixels && shown && inside ? pixelAt(pixels.rgba, pixels.width, shown.x, shown.y) : null);

  function pointed(event: PointerEvent): { x: number; y: number } | null {
    if (!pixels) return null;
    const box = (event.currentTarget as HTMLElement).getBoundingClientRect();
    return cellAt(event.clientX - box.left, event.clientY - box.top, zoom, pixels.width, pixels.height);
  }

  function key(event: KeyboardEvent) {
    if (!pixels) return;
    if (event.key === 'Escape') {
      pinned = null;
      return;
    }
    const start = pinned ?? hover ?? { x: Math.floor(pixels.width / 2), y: Math.floor(pixels.height / 2) };
    const next = moveCell(start, event.key, pixels.width, pixels.height);
    if (next !== start) {
      event.preventDefault();
      pinned = next;
    }
  }

  function setZoom(value: number) {
    zoom = Math.min(Math.max(Math.round(value), 1), MAX_ZOOM);
  }
</script>

<div class="inspector">
  <div class="tools">
    <label class="select">
      <span>{t('pixels.image')}</span>
      <select value={index} onchange={(e) => (picked = Number(e.currentTarget.value))}>
        {#each sizes as size, i (size)}
          <option value={i}>{size} × {size}</option>
        {/each}
      </select>
    </label>
    <div class="zoom">
      <button type="button" class="quiet" aria-label={t('pixels.zoom_out')} onclick={() => setZoom(zoom - 1)} disabled={zoom <= 1}>−</button>
      <input
        type="range"
        min="1"
        max={MAX_ZOOM}
        value={zoom}
        aria-label={t('pixels.zoom')}
        oninput={(e) => setZoom(e.currentTarget.valueAsNumber)}
      />
      <button type="button" class="quiet" aria-label={t('pixels.zoom_in')} onclick={() => setZoom(zoom + 1)} disabled={zoom >= MAX_ZOOM}>+</button>
      <output>{zoom}×</output>
      <button type="button" class="quiet" onclick={() => pixels && setZoom(fitZoom(width, pixels.width))}>{t('pixels.fit')}</button>
    </div>
    <label><input type="checkbox" bind:checked={grid} /> {t('pixels.grid')}</label>
  </div>

  <div class="viewport" bind:clientWidth={width}>
    {#if failure}
      <p class="failure" role="alert">{failure}</p>
    {:else if pixels}
      <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
      <div
        class="surface"
        role="group"
        tabindex="0"
        aria-label={t('pixels.surface')}
        style="width:{pixels.width * zoom}px;height:{pixels.height * zoom}px"
        onpointermove={(e) => (hover = pointed(e))}
        onpointerleave={() => (hover = null)}
        onclick={(e) => (pinned = pointed(e as unknown as PointerEvent))}
        onkeydown={key}
      >
        <canvas bind:this={canvas} style="width:100%;height:100%"></canvas>
        {#if grid && zoom >= 6}
          <div class="grid" style="background-size:{zoom}px {zoom}px"></div>
        {/if}
        {#if shown && inside}
          <div
            class="cell"
            class:pinned={pinned !== null}
            style="left:{shown.x * zoom}px;top:{shown.y * zoom}px;width:{zoom}px;height:{zoom}px"
          ></div>
        {/if}
      </div>
    {/if}
  </div>

  <div class="values" aria-live="polite">
    {#if info && shown}
      <span class="swatch" aria-hidden="true">
        <span style="background-color: rgba({info.r}, {info.g}, {info.b}, {info.a / 255})"></span>
      </span>
      <dl>
        <div><dt>{t('pixels.position')}</dt><dd>{shown.x}, {shown.y}</dd></div>
        <div><dt>{t('pixels.color')}</dt><dd>{info.r}, {info.g}, {info.b}</dd></div>
        <div><dt>{t('pixels.alpha')}</dt><dd>{info.a} ({opacityPercent(info.a)} %)</dd></div>
        <div><dt>{t('pixels.hex')}</dt><dd>{hexOf(info)}</dd></div>
      </dl>
    {:else}
      <p class="hint">{t('pixels.hint')}</p>
    {/if}
  </div>
</div>
