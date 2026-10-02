<script lang="ts">
  import { ICONS, type IconName } from '../lib/icons';

  let { name, size = 18 }: { name: IconName; size?: number } = $props();
</script>

<svg viewBox="0 0 24 24" width={size} height={size} fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">{@html ICONS[name]}</svg>
