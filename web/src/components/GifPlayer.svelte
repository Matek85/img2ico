<script lang="ts">
  import { onDestroy } from 'svelte';
  import { t } from '../i18n';
  import Icon from './Icon.svelte';

  // A small player for an animated GIF: it plays the frames, steps through them one by
  // one, and the frame it stops at is the one the icon is made from (`frame`, from 0).
  let {
    count,
    delays,
    frame = $bindable(),
    load,
  }: {
    count: number;
    delays: number[];
    frame: number;
    /** The address of a picture of frame `index`. */
    load: (index: number) => Promise<string>;
  } = $props();

  let playing = $state(false);
  // The frame being shown: it is `frame` while stopped and runs on while playing.
  let shown = $state(0);
  let shownUrl = $state('');
  let timer: ReturnType<typeof setTimeout> | undefined;
  // Which showing is the latest, so a slow picture does not overwrite a newer one.
  let showing = 0;

  async function show(index: number) {
    const mine = ++showing;
    const url = await load(index);
    if (mine === showing) shownUrl = url;
  }

  // Stopped: the player follows the chosen frame (also when it is changed from outside).
  $effect(() => {
    if (playing) return;
    shown = frame;
    void show(frame);
  });

  async function tick() {
    const next = (shown + 1) % count;
    // The next picture is ready before it is due, so the animation does not stutter.
    const [url] = await Promise.all([load(next), new Promise((done) => (timer = setTimeout(done, delays[shown] ?? 100)))]);
    if (!playing) return;
    shown = next;
    shownUrl = url;
    void tick();
  }

  function play() {
    playing = true;
    void tick();
  }

  function pause() {
    playing = false;
    clearTimeout(timer);
    showing += 1;
    frame = shown;
  }

  function step(delta: number) {
    if (playing) pause();
    frame = (frame + delta + count) % count;
  }

  onDestroy(() => {
    playing = false;
    clearTimeout(timer);
  });
</script>

<div class="gif-player" role="group" aria-label={t('gif.title')}>
  <div class="gif-frame">
    {#if shownUrl}<img src={shownUrl} alt="" />{/if}
  </div>
  <div class="gif-controls">
    <button
      type="button"
      class="primary icon-button"
      title={t(playing ? 'gif.pause' : 'gif.play')}
      aria-label={t(playing ? 'gif.pause' : 'gif.play')}
      onclick={() => (playing ? pause() : play())}
    >
      <Icon name={playing ? 'pause' : 'play'} />
    </button>
    <button type="button" class="outline icon-button" title={t('gif.prev')} aria-label={t('gif.prev')} onclick={() => step(-1)}>
      <Icon name="framePrev" />
    </button>
    <input
      type="range"
      min="0"
      max={count - 1}
      step="1"
      value={playing ? shown : frame}
      aria-label={t('gif.slider')}
      oninput={(event) => {
        if (playing) pause();
        frame = event.currentTarget.valueAsNumber;
      }}
    />
    <button type="button" class="outline icon-button" title={t('gif.next')} aria-label={t('gif.next')} onclick={() => step(1)}>
      <Icon name="frameNext" />
    </button>
    <span class="gif-count" aria-live="polite">{t('gif.frame', { frame: (playing ? shown : frame) + 1, count })}</span>
  </div>
  <p class="hint">{t('gif.hint')}</p>
</div>
