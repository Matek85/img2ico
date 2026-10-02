<script lang="ts">
  import { t } from '../i18n';
  import { HANDLES, type Handle, type Rect, type Size, clampRect, dragRect } from '../lib/crop';

  let {
    src,
    size,
    rect = $bindable(),
    aspect = null,
  }: { src: string; size: Size; rect: Rect; aspect?: number | null } = $props();

  let stage = $state<HTMLDivElement>();
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

<div class="crop-stage" bind:this={stage}>
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
