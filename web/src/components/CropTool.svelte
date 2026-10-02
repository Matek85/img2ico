<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from '../i18n';
  import Icon from './Icon.svelte';
  import { HANDLES, type Handle, type Rect, type Size, clampRect, dragRect } from '../lib/crop';

  let {
    src,
    size,
    rect = $bindable(),
    aspect = null,
  }: { src: string; size: Size; rect: Rect; aspect?: number | null } = $props();

  let stage = $state<HTMLDivElement>();
  let viewport = $state<HTMLDivElement>();

  // The wheel zooms the picture, with the pointer anywhere on the preview box; the
  // point under the pointer stays where it is.
  const MAX_ZOOM = 3;
  let zoom = $state(1);

  function zoomBy(factor: number, clientX: number, clientY: number) {
    if (!viewport) return;
    const next = Math.min(MAX_ZOOM, Math.max(1, zoom * factor));
    if (next === zoom) return;
    const box = viewport.getBoundingClientRect();
    const px = clientX - box.left;
    const py = clientY - box.top;
    const fx = (viewport.scrollLeft + px) / viewport.scrollWidth;
    const fy = (viewport.scrollTop + py) / viewport.scrollHeight;
    zoom = next;
    // Once the new size is laid out, put the same point under the pointer again.
    requestAnimationFrame(() => {
      if (!viewport) return;
      viewport.scrollLeft = fx * viewport.scrollWidth - px;
      viewport.scrollTop = fy * viewport.scrollHeight - py;
    });
  }

  onMount(() => {
    const box = viewport?.closest<HTMLElement>('.stage') ?? viewport;
    if (!box) return;
    const wheel = (event: WheelEvent) => {
      event.preventDefault();
      zoomBy(Math.exp(-event.deltaY * 0.0015), event.clientX, event.clientY);
    };
    box.addEventListener('wheel', wheel, { passive: false });
    return () => box.removeEventListener('wheel', wheel);
  });
  let drag: { handle: Handle; startX: number; startY: number; from: Rect } | undefined;

  /** Source pixels per screen pixel, as the picture is currently shown. */
  function scale(): number {
    return stage ? size.width / stage.clientWidth : 1;
  }

  function begin(event: PointerEvent, handle: Handle) {
    event.preventDefault();
    event.stopPropagation();
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    drag = { handle, startX: event.clientX, startY: event.clientY, from: { ...rect } };
  }

  function move(event: PointerEvent) {
    if (!drag) return;
    const k = scale();
    rect = dragRect(
      drag.from,
      drag.handle,
      Math.round((event.clientX - drag.startX) * k),
      Math.round((event.clientY - drag.startY) * k),
      size,
      aspect,
    );
  }

  function end() {
    drag = undefined;
  }

  // The arrow keys move the frame by a pixel, ten with Shift - the way to
  // place it exactly, and the way without a pointer.
  function key(event: KeyboardEvent) {
    const step = event.shiftKey ? 10 : 1;
    const delta: Record<string, [number, number]> = {
      ArrowLeft: [-step, 0],
      ArrowRight: [step, 0],
      ArrowUp: [0, -step],
      ArrowDown: [0, step],
    };
    const d = delta[event.key];
    if (!d) return;
    event.preventDefault();
    rect = clampRect({ ...rect, x: rect.x + d[0], y: rect.y + d[1] }, size);
  }

  let style = $derived(
    `left:${(rect.x / size.width) * 100}%;top:${(rect.y / size.height) * 100}%;` +
      `width:${(rect.width / size.width) * 100}%;height:${(rect.height / size.height) * 100}%`,
  );
</script>

<div class="crop-viewport" class:zoomed={zoom > 1} bind:this={viewport}>
<div class="crop-stage" style:width="{zoom * 100}%" bind:this={stage}>
  <img {src} alt="" draggable="false" />
  <!-- The frame is a small custom control: it takes the pointer and the arrow keys. -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
  <div
    class="frame"
    {style}
    role="group"
    tabindex="0"
    aria-label={t('crop.frame')}
    onpointerdown={(e) => begin(e, 'move')}
    onpointermove={move}
    onpointerup={end}
    onpointercancel={end}
    onkeydown={key}
  >
    {#each HANDLES as handle (handle)}
      <span
        class="handle {handle}"
        aria-hidden="true"
        onpointerdown={(e) => begin(e, handle)}
        onpointermove={move}
        onpointerup={end}
        onpointercancel={end}
      ></span>
    {/each}
  </div>
</div>
</div>

{#if zoom > 1}
  <button type="button" class="chip crop-zoom" title={t('crop.zoom_reset')} onclick={() => (zoom = 1)}>
    <Icon name="fit" size={14} />
    {Math.round(zoom * 100)}%
  </button>
{/if}

