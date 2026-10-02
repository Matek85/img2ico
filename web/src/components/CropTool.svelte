<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from '../i18n';
  import { type Rect, type Size, panRect, zoomRect } from '../lib/crop';

  // The frame stays where it is on the screen (it has the shape of the crop); the
  // picture moves and grows beneath it. `rect` is the part of the picture under
  // the frame, in the picture's own pixels.
  let {
    src,
    size,
    rect = $bindable(),
  }: { src: string; size: Size; rect: Rect; aspect?: number | null } = $props();

  const MARGIN = 14;
  let box = $state<HTMLDivElement>();
  let width = $state(0);
  let height = $state(0);

  // Screen pixels per picture pixel, so that the frame is as big as the box allows.
  let scale = $derived(
    width > 0 && height > 0 ? Math.min((width - 2 * MARGIN) / rect.width, (height - 2 * MARGIN) / rect.height) : 1,
  );
  let frameW = $derived(rect.width * scale);
  let frameH = $derived(rect.height * scale);
  let frameX = $derived((width - frameW) / 2);
  let frameY = $derived((height - frameH) / 2);

  let imageStyle = $derived(
    `left:${frameX - rect.x * scale}px;top:${frameY - rect.y * scale}px;` +
      `width:${size.width * scale}px;height:${size.height * scale}px`,
  );
  let frameStyle = $derived(`left:${frameX}px;top:${frameY}px;width:${frameW}px;height:${frameH}px`);

  // Dragging moves the picture under the frame.
  let drag: { x: number; y: number; from: Rect } | undefined;

  function begin(event: PointerEvent) {
    event.preventDefault();
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    drag = { x: event.clientX, y: event.clientY, from: { ...rect } };
  }

  function move(event: PointerEvent) {
    if (!drag) return;
    rect = panRect(drag.from, -(event.clientX - drag.x) / scale, -(event.clientY - drag.y) / scale, size);
  }

  function end() {
    drag = undefined;
  }

  // The wheel zooms, with the pointer anywhere on the preview box: the point under
  // the pointer stays where it is.
  function zoom(factor: number, clientX: number, clientY: number) {
    if (!box) return;
    const at = box.getBoundingClientRect();
    const ax = (clientX - at.left - frameX) / frameW;
    const ay = (clientY - at.top - frameY) / frameH;
    rect = zoomRect(rect, factor, ax, ay, size);
  }

  onMount(() => {
    const area = box?.closest<HTMLElement>('.stage') ?? box;
    if (!area) return;
    const wheel = (event: WheelEvent) => {
      event.preventDefault();
      zoom(Math.exp(event.deltaY * 0.0015), event.clientX, event.clientY);
    };
    area.addEventListener('wheel', wheel, { passive: false });
    return () => area.removeEventListener('wheel', wheel);
  });

  // The arrow keys move the picture by a pixel (ten with Shift), + and - zoom:
  // the way to place it exactly, and the way without a pointer.
  function key(event: KeyboardEvent) {
    const step = event.shiftKey ? 10 : 1;
    const delta: Record<string, [number, number]> = {
      ArrowLeft: [-step, 0],
      ArrowRight: [step, 0],
      ArrowUp: [0, -step],
      ArrowDown: [0, step],
    };
    if (event.key === '+' || event.key === '=') {
      event.preventDefault();
      rect = zoomRect(rect, 0.9, 0.5, 0.5, size);
    } else if (event.key === '-') {
      event.preventDefault();
      rect = zoomRect(rect, 1 / 0.9, 0.5, 0.5, size);
    } else if (delta[event.key]) {
      event.preventDefault();
      rect = panRect(rect, delta[event.key][0], delta[event.key][1], size);
    }
  }
</script>

<div class="crop-view" bind:this={box} bind:clientWidth={width} bind:clientHeight={height}>
  <img {src} alt="" draggable="false" style={imageStyle} />
  <!-- The frame is a small custom control: it takes the pointer and the keys. -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
  <div
    class="crop-pane"
    role="group"
    tabindex="0"
    aria-label={t('crop.frame')}
    onpointerdown={begin}
    onpointermove={move}
    onpointerup={end}
    onpointercancel={end}
    onkeydown={key}
  >
    <span class="crop-frame" style={frameStyle}></span>
  </div>
</div>
