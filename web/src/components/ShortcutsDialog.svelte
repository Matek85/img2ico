<script lang="ts">
  import { t } from '../i18n';
  import { keys, setShortcutsOn } from '../lib/keys.svelte';
  import { CROP_KEYS, GROUPS, HISTORY_KEYS, SHORTCUTS, comboText, labelOf } from '../lib/shortcuts';
  import Icon from './Icon.svelte';

  // The list of the keys, with the switch for them. Opened by the keyboard button or by "?".
  let dialog = $state<HTMLDialogElement>();

  export function show() {
    if (dialog && !dialog.open) dialog.showModal();
  }
</script>

<dialog class="site-dialog keys-dialog" bind:this={dialog} aria-labelledby="keys-title" onclick={(e) => e.target === dialog && dialog?.close()}>
  <h3 id="keys-title"><Icon name="keyboard" />{t('keys.title')}</h3>
  <p class="hint">{t('keys.intro')}</p>
  <label class="autosave keys-switch">
    <input type="checkbox" checked={keys.on} onchange={(e) => setShortcutsOn(e.currentTarget.checked)} />
    {t('keys.on')}
  </label>
  <div class="keys-groups" class:off={!keys.on}>
    {#each GROUPS as group (group)}
      <section>
        <h4>{t(`keys.group_${group}`)}</h4>
        <dl>
          {#each SHORTCUTS.filter((s) => s.group === group) as shortcut (shortcut.id)}
            <div>
              <dt><kbd>{labelOf(shortcut)}</kbd></dt>
              <dd>{t(`keys.act_${shortcut.id}`)}</dd>
            </div>
          {/each}
        </dl>
      </section>
    {/each}
    <section>
      <h4>{t('keys.group_history')}</h4>
      <dl>
        {#each HISTORY_KEYS as entry (entry.action)}
          <div>
            <dt>
              {#each entry.combos as combo, at}{#if at > 0}<span class="or">{' / '}</span>{/if}<kbd>{comboText(combo, { ctrl: t('keys.ctrl'), shift: t('keys.shift') })}</kbd>{/each}
            </dt>
            <dd>{t(`keys.act_${entry.action}`)}</dd>
          </div>
        {/each}
      </dl>
    </section>
    <section>
      <h4>{t('keys.group_crop')}</h4>
      <dl>
        {#each CROP_KEYS as crop (crop.action)}
          <div>
            <dt><kbd>{crop.labelKey ? t(crop.labelKey) : labelOf(crop)}</kbd></dt>
            <dd>{t(`keys.crop_${crop.action}`)}</dd>
          </div>
        {/each}
      </dl>
    </section>
  </div>
  <p class="hint">{t('keys.crop_note')}</p>
  <div class="dialog-actions">
    <button type="button" class="primary" onclick={() => dialog?.close()}><Icon name="check" />{t('keys.close')}</button>
  </div>
</dialog>
