<script lang="ts">
  import { t } from '../i18n';
  import { beforeLayout } from '../lib/compare';
  import type { Rect, Size } from '../lib/crop';

  let {
    before,
    after,
    picture,
    crop,
  }: {
    /** The original picture's address, and the icon image's. */
    before: string;
    after: string;
    picture: Size;
    crop: Rect | null;
  } = $props();

  /** Where the divider is, 0 (all "after") to 100 (all "before"), in percent. */
  let position = $state(50);
  let box = $state<HTMLDivElement>();
  let dragging = false;

  // The "before" is drawn in a square of 1000 units, scaled by the page.
  const UNITS = 1000;
  let layout = $derived(beforeLayout(picture, crop, UNITS));
  // Percentages of the square: for a positioned image, left/top/width/height
  // are relative to the square it sits in.
  let beforeImageStyle = $derived(
    `left:${(layout.x / UNITS) * 100}%;top:${(layout.y / UNITS) * 100}%;` +
      `width:${(layout.width / UNITS) * 100}%;height:${(layout.height / UNITS) * 100}%`,
  );
  let beforeStyle = $derived(`clip-path:inset(0 ${100 - position}% 0 0)`);

  function place(event: PointerEvent) {
    if (!box) return;
    const rect = box.getBoundingClientRect();
    position = Math.round(Math.min(Math.max(((event.clientX - rect.left) / rect.width) * 100, 0), 100));
  }

  function down(event: PointerEvent) {
    dragging = true;
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    place(event);
  }

  function move(event: PointerEvent) {
    if (dragging) place(event);
  }
</script>

<div class="compare">
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="box"
    bind:this={box}
    onpointerdown={down}
    onpointermove={move}
    onpointerup={() => (dragging = false)}
    onpointercancel={() => (dragging = false)}
  >
    <img class="after" src={after} alt="" draggable="false" />
    <div class="before" style={beforeStyle}>
      <img src={before} alt="" draggable="false" style={beforeImageStyle} />
    </div>
    <div class="divider" style="left:{position}%"></div>
    <span class="tag left">{t('compare.before')}</span>
    <span class="tag right">{t('compare.after')}</span>
  </div>
  <label class="slider">
    <span>{t('compare.divider')}</span>
    <input type="range" min="0" max="100" bind:value={position} />
    <output>{position}%</output>
  </label>
</div>
