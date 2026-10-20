// Graphics for certain maps: the button under "Optimise for my PC" opens the maps; the one
// picked shows the graphics settings of the Graphics page to change and keep for that map
// alone. The game loads the map with them (omsi_launcher_lib::map_graphics); every other map
// keeps the settings of the page, and a change of those leaves these maps as they are.

import { html, useState, useEffect } from '../../vendor/preact-htm.js';
import { useStore, toast } from '../store.js';
import { t } from '../i18n.js';
import { call } from '../api.js';
import { Button, Card, Dialog, Search, Picture, Segmented, Empty } from '../ui.js';
import { mapPicture } from './drive.js';
import { SETTINGS_TABS, GRAPHICS_PROFILE_KEYS } from './settings-schema.js';
import { SettingRow, SettingsScope } from './settings.js';

/** The name a map is kept under: the one `omsi_launcher_lib::map_graphics::key` makes. */
export function mapKey(file) {
  const p = String(file || '').trim().replace(/\\/g, '/').replace(/^\.\//, '');
  const lower = p.toLowerCase();
  const absolute = lower.startsWith('/') || lower[1] === ':';
  const at = absolute ? lower.lastIndexOf('/maps/') : -1;
  return at >= 0 ? lower.slice(at + 1) : lower;
}

const nameOf = (m) => m.friendly || m.name || m.file;

/** The rows of the Graphics tab a map can keep of its own: the settings of a graphics profile
 * and the quality presets, in the columns and sections of the tab. A section with no row left
 * is left out. */
function editorCols(settings, platform) {
  const tab = SETTINGS_TABS.find((x) => x.id === 'graphics');
  const mine = (row) => {
    if (row.type === 'custom') return row.id === 'preset';
    if (row.type !== 'select' && row.type !== 'toggle' && row.type !== 'slider') return false;
    const key = typeof row.key === 'function' ? row.key(settings) : row.key;
    return GRAPHICS_PROFILE_KEYS.includes(key);
  };
  return tab.cols
    .map((col) => {
      const rows = [];
      let section = null;
      for (const row of col.rows) {
        if (row.type === 'section') {
          section = row;
        } else if (mine(row) && (!row.when || row.when(settings, platform))) {
          if (section) rows.push(section);
          section = null;
          rows.push(row);
        }
      }
      return { title: col.title, rows };
    })
    .filter((col) => col.rows.length);
}

/** One map's graphics: a copy of the page's settings (or what the map keeps already) to change,
 * and Save. */
function MapEditor({ map, kept, onBack, onDone, onClose }) {
  const s = useStore();
  const own = kept[mapKey(map.file)];
  const [draft, setDraft] = useState(() => {
    const page = Object.fromEntries(GRAPHICS_PROFILE_KEYS.filter((k) => s.settings?.[k] !== undefined).map((k) => [k, s.settings[k]]));
    return { ...page, ...(own || {}) };
  });
  const [busy, setBusy] = useState(false);
  const name = nameOf(map);
  // the rows read and change the draft of this map, not the settings of the page
  const settings = { ...s.settings, ...draft };
  const scope = { settings, save: async (patch) => (setDraft((d) => ({ ...d, ...patch })), true) };
  const finish = async (cmd, args, message) => {
    setBusy(true);
    try {
      onDone(await call(cmd, args), message);
    } catch (e) {
      toast(e.message, 'error');
      setBusy(false);
    }
  };
  const save = () => finish('map_graphics_save', { map: map.file, settings }, t('Graphics saved for {map}', { map: name }));
  const reset = () => finish('map_graphics_delete', { map: map.file }, t('{map} uses your normal settings again', { map: name }));
  const platform = s.boot?.platform || '';
  return html`<${Dialog} wide title=${t('Graphics of {map}', { map: name })} onClose=${onClose} footer=${html`
      <${Button} label="Back" disabled=${busy} onClick=${onBack} />
      ${own && html`<${Button} label="Use my normal settings for this map" disabled=${busy} onClick=${reset} />`}
      <${Button} kind="primary" label="Save for this map" disabled=${busy} onClick=${save} />`}>
    <p class="muted small" style="margin:0 0 12px">${t('The game loads this map with these settings when it is started for it from the launcher. Every other map keeps the settings of the Settings page. Window size, full screen, graphics API and the simulation thread stay as they are there.')}</p>
    <${SettingsScope.Provider} value=${scope}>
      <div class="settings-grid">
        ${editorCols(settings, platform).map((col) => html`<${Card} title=${col.title}>
          ${col.rows.map((row, i) => html`<${SettingRow} key=${i + (row.key || row.id || row.title || '')} row=${row} />`)}
        </${Card}>`)}
      </div>
    </${SettingsScope.Provider}>
  </${Dialog}>`;
}

/** The installed maps as pictures; the ones with graphics of their own are marked. */
function MapList({ kept, onPick, onClose }) {
  const s = useStore();
  const [q, setQ] = useState('');
  const [which, setWhich] = useState('all');
  const ql = q.trim().toLowerCase();
  const has = (m) => !!kept[mapKey(m.file)];
  const maps = (s.maps || [])
    .filter((m) => which === 'all' || has(m))
    .filter((m) => !ql || `${m.friendly} ${m.name} ${m.file}`.toLowerCase().includes(ql))
    .sort((a, b) => nameOf(a).localeCompare(nameOf(b)));
  const own = (s.maps || []).filter(has).length;
  return html`<${Dialog} wide title="Change settings for certain maps" onClose=${onClose}>
    <p class="muted small" style="margin:0 0 12px">${t('Choose a map to give it graphics settings of its own. Maps marked as having their own settings can be changed again at any time.')}</p>
    <div class="row" style="margin-bottom:16px">
      <div class="grow" style="max-width:360px"><${Search} value=${q} onInput=${setQ} placeholder="Search maps" autofocus /></div>
      <${Segmented} value=${which} options=${[['all', 'All'], ['own', 'With own settings']]} onChange=${setWhich} />
      <span class="muted small right">${t('{n} maps with their own settings', { n: own })}</span>
    </div>
    ${maps.length === 0 && html`<${Empty} icon="map" title="No map found" />`}
    <div class="tiles">
      ${maps.map((m) => html`<button class=${'tile' + (has(m) ? ' on' : '')} onClick=${() => onPick(m)}>
        <${Picture} src=${mapPicture(m.file)} />
        <div class="cap"><b>${nameOf(m)}</b><span>${has(m) ? t('Own settings') : t('Settings of the Settings page')}</span></div>
      </button>`)}
    </div>
  </${Dialog}>`;
}

/** The button of the Graphics tab, and the dialogs behind it. */
export function MapGraphicsBlock() {
  const [kept, setKept] = useState(null);
  const [open, setOpen] = useState(false);
  const [picked, setPicked] = useState(null);
  useEffect(() => {
    let gone = false;
    call('map_graphics')
      .then((k) => !gone && setKept(k || {}))
      .catch(() => !gone && setKept({}));
    return () => (gone = true);
  }, []);
  const n = Object.keys(kept || {}).length;
  const close = () => (setOpen(false), setPicked(null));
  return html`<div class="map-graphics-block">
    <div class="section-title">${t('Graphics for certain maps')}</div>
    <p class="muted small" style="margin:0 0 4px">${t('A map can have graphics settings of its own and is loaded with them. Every other map keeps the settings on this page, and a change of them leaves these maps as they are.')}</p>
    <div class="setting-row full">
      <div class="row">
        <${Button} icon="map" label="Change settings for certain maps" disabled=${!kept} onClick=${() => setOpen(true)} />
        ${n > 0 && html`<span class="muted small">${t('{n} maps with their own settings', { n })}</span>`}
      </div>
    </div>
    ${open && kept && !picked && html`<${MapList} kept=${kept} onPick=${setPicked} onClose=${close} />`}
    ${open &&
    kept &&
    picked &&
    html`<${MapEditor}
      key=${picked.file}
      map=${picked}
      kept=${kept}
      onBack=${() => setPicked(null)}
      onClose=${close}
      onDone=${(all, message) => {
        setKept(all || {});
        setPicked(null);
        toast(message, 'ok');
      }}
    />`}
  </div>`;
}
