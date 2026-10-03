<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from '../i18n';
  import { HANDLES, type Handle, type Rect, type Size, dragRect, panRect, zoomRect } from '../lib/crop';

  // Locked: the frame stays in the middle of the box (it has the shape of the crop) and
  // the picture is dragged and zoomed beneath it. `rect` is the part of the picture under
  // the frame, in the picture's own pixels.
  // Unlocked: the picture stays, and the frame is moved and resized with its handles;
  // the wheel still zooms the picture.
  let {
    src,
    size,
    rect = $bindable(),
    aspect = null,
    locked = true,
    ontogglelock,
  }: {
    src: string;
    size: Size;
    rect: Rect;
    aspect?: number | null;
    locked?: boolean;
    /** The space bar was pressed: lock or unlock the frame. */
    ontogglelock?: () => void;
  } = $props();

  const MARGIN = 14;
  let box = $state<HTMLDivElement>();
  let pane = $state<HTMLDivElement>();
  let width = $state(0);
  let height = $state(0);

  // Locked: screen pixels per picture pixel, so that the frame is as big as the box allows.
  let fit = $derived(
    width > 0 && height > 0 ? Math.min((width - 2 * MARGIN) / rect.width, (height - 2 * MARGIN) / rect.height) : 1,
  );
  let fitLeft = $derived((width - rect.width * fit) / 2 - rect.x * fit);
  let fitTop = $derived((height - rect.height * fit) / 2 - rect.y * fit);

  // Unlocked: the picture as it was shown when the frame was unlocked, and as it is zoomed.
  let free = $state({ scale: 1, left: 0, top: 0 });
  $effect(() => {
    if (locked) free = { scale: fit, left: fitLeft, top: fitTop };
  });

  let scale = $derived(locked ? fit : free.scale);
  let left = $derived(locked ? fitLeft : free.left);
  let top = $derived(locked ? fitTop : free.top);

  let imageStyle = $derived(`left:${left}px;top:${top}px;width:${size.width * scale}px;height:${size.height * scale}px`);
  let frameStyle = $derived(
    `left:${left + rect.x * scale}px;top:${top + rect.y * scale}px;width:${rect.width * scale}px;height:${rect.height * scale}px`,
  );

  type Drag =
    | { kind: 'picture'; x: number; y: number; from: Rect }
    | { kind: 'frame'; handle: Handle; x: number; y: number; from: Rect };
  let drag: Drag | undefined;

  // Locked: dragging moves the picture under the frame.
  function beginPicture(event: PointerEvent) {
    if (!locked) return;
    event.preventDefault();
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    drag = { kind: 'picture', x: event.clientX, y: event.clientY, from: { ...rect } };
  }

  // Unlocked: dragging the frame or one of its handles.
  function beginFrame(event: PointerEvent, handle: Handle) {
    if (locked) return;
    event.preventDefault();
    event.stopPropagation();
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    drag = { kind: 'frame', handle, x: event.clientX, y: event.clientY, from: { ...rect } };
  }

  function move(event: PointerEvent) {
    if (!drag) return;
    const dx = event.clientX - drag.x;
    const dy = event.clientY - drag.y;
    if (drag.kind === 'picture') {
      rect = panRect(drag.from, -dx / scale, -dy / scale, size);
    } else {
      rect = dragRect(drag.from, drag.handle, Math.round(dx / scale), Math.round(dy / scale), size, aspect);
    }
  }

  function end() {
    drag = undefined;
  }

  // Zooming while the button is held: the drag goes on from the zoomed picture, not from the one it
  // began with (or the next move would undo the zoom).
  function rebase(clientX: number, clientY: number) {
    if (drag) drag = { ...drag, x: clientX, y: clientY, from: { ...rect } };
  }

  // The wheel zooms, with the pointer anywhere on the preview box: the point under
  // the pointer stays where it is.
  function zoom(factor: number, clientX: number, clientY: number) {
    if (!box) return;
    const at = box.getBoundingClientRect();
    const px = clientX - at.left;
    const py = clientY - at.top;
    if (locked) {
      rect = zoomRect(rect, factor, (px - (left + rect.x * scale)) / (rect.width * scale), (py - (top + rect.y * scale)) / (rect.height * scale), size);
      rebase(clientX, clientY);
      return;
    }
    // Unlocked: the picture (and the frame on it) grows around the pointer.
    const wanted = free.scale / factor;
    const next = Math.min(Math.max(wanted, Math.min(width / size.width, height / size.height) / 2), 12);
    const k = next / free.scale;
    free = { scale: next, left: px - (px - free.left) * k, top: py - (py - free.top) * k };
    rebase(clientX, clientY);
  }

  onMount(() => {
    // The keys work at once, without a click first.
    pane?.focus({ preventScroll: true });
    const area = box?.closest<HTMLElement>('.stage') ?? box;
    if (!area) return;
    const wheel = (event: WheelEvent) => {
      event.preventDefault();
      zoom(Math.exp(event.deltaY * 0.0015), event.clientX, event.clientY);
    };
    area.addEventListener('wheel', wheel, { passive: false });
    return () => area.removeEventListener('wheel', wheel);
  });

  // The arrow keys move the picture (locked) or the frame (unlocked) by a pixel, ten with
  // Shift; plus and minus zoom: the way to place it exactly, and the way without a pointer.
  // The space bar locks or unlocks the frame.
  function key(event: KeyboardEvent) {
    // The number pad places the frame (the editor handles it), whatever NumLock makes of the key.
    if (/^Numpad\d$/.test(event.code)) return;
    const step = event.shiftKey ? 10 : 1;
    const delta: Record<string, [number, number]> = {
      ArrowLeft: [-step, 0],
      ArrowRight: [step, 0],
      ArrowUp: [0, -step],
      ArrowDown: [0, step],
    };
    if (event.key === ' ') {
      event.preventDefault();
      ontogglelock?.();
    } else if (event.key === '+' || event.key === '=' || event.key === '-') {
      event.preventDefault();
      const factor = event.key === '-' ? 1 / 0.9 : 0.9;
      const at = box?.getBoundingClientRect();
      if (at) zoom(factor, at.left + width / 2, at.top + height / 2);
    } else if (delta[event.key]) {
      event.preventDefault();
      const [dx, dy] = delta[event.key];
      // Locked, the picture moves under the frame (so the frame's place on it goes the other way).
      rect = locked ? panRect(rect, -dx, -dy, size) : panRect(rect, dx, dy, size);
    }
  }
</script>

<div class="crop-view" class:unlocked={!locked} bind:this={box} bind:clientWidth={width} bind:clientHeight={height}>
  <img {src} alt="" draggable="false" style={imageStyle} />
  <!-- The pane is a small custom control: it takes the pointer and the keys. -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
  <div
    class="crop-pane"
    bind:this={pane}
    role="group"
    tabindex="0"
    aria-label={t(locked ? 'crop.frame_locked' : 'crop.frame_unlocked')}
    onpointerdown={(e) => {
      pane?.focus({ preventScroll: true });
      beginPicture(e);
    }}
    onpointermove={move}
    onpointerup={end}
    onpointercancel={end}
    onkeydown={key}
  >
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <span
      class="crop-frame"
      style={frameStyle}
      onpointerdown={(e) => beginFrame(e, 'move')}
      onpointermove={move}
      onpointerup={end}
      onpointercancel={end}
    >
      {#if !locked}
        {#each HANDLES as handle (handle)}
          <span
            class="handle {handle}"
            aria-hidden="true"
            onpointerdown={(e) => beginFrame(e, handle)}
            onpointermove={move}
            onpointerup={end}
            onpointercancel={end}
          ></span>
        {/each}
      {/if}
    </span>
  </div>
</div>
