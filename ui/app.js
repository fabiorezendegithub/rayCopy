(function () {
  'use strict';

  // ---------------------------------------------------------------------------
  // Constants (must stay in sync with src/model.rs and src/theme.rs)
  // ---------------------------------------------------------------------------

  const LANGS = {
    auto: 'Auto-Detect',
    csharp: 'C#',
    html: 'HTML',
    css: 'CSS',
    javascript: 'JavaScript',
  };

  const THEMES = ['candy', 'sand'];
  const PADDINGS = [16, 32, 64, 128];
  const SCALES = [2, 4, 6];

  const FONTS = {
    'jetbrains-mono': { name: 'JetBrains Mono' },
    'geist-mono': { name: 'Geist Mono' },
    'ibm-plex-mono': { name: 'IBM Plex Mono' },
    'fira-code': { name: 'Fira Code' },
    'soehne-mono': { name: 'Söhne Mono' },
    'roboto-mono': { name: 'Roboto Mono' },
    'commit-mono': { name: 'Commit Mono' },
    'space-mono': { name: 'Space Mono' },
    'source-code-pro': { name: 'Source Code Pro' },
    'google-sans-code': { name: 'Google Sans Code' },
  };

  // Palettes copied from src/theme.rs (only the fields the preview uses are
  // needed for the frame, the rest are kept so the JS mirrors the Rust table).
  const PALETTES = {
    candy: {
      light: {
        bgFrom: '#A58EFB', bgTo: '#E9BFF8', foreground: '#434447', comment: '#8D949B',
        keyword: '#DC155E', string: '#B2762E', function: '#009033', parameter: '#009033',
        number: '#676DFF', constant: '#2286A6', property: '#2286A6', punctuation: '#d15a8b',
        highlight: 'rgba(220,21,94,0.10)', highlightBorder: '#DC155E', window: 'rgba(255,255,255,0.75)',
      },
      dark: {
        bgFrom: '#A58EFB', bgTo: '#E9BFF8', foreground: '#FFFFFF', comment: '#807796',
        keyword: '#FF659C', string: '#DFD473', function: '#73DFA5', parameter: '#1AC8FF',
        number: '#7A7FFD', constant: '#1AC8FF', property: '#1AC8FF', punctuation: '#FF659C',
        highlight: 'rgba(255,101,156,0.14)', highlightBorder: '#FF659C', window: '#322a3f',
      },
    },
    sand: {
      light: {
        bgFrom: '#EED5B6', bgTo: '#AF8856', foreground: '#262217', comment: '#C4B39C',
        keyword: '#906937', string: '#A28C4E', function: '#DA8744', parameter: '#DA8744',
        number: '#A28C4E', constant: '#A28C4E', property: '#A28C4E', punctuation: '#DA8744',
        highlight: 'rgba(218,135,68,0.10)', highlightBorder: '#DA8744', window: 'rgba(255,255,255,0.75)',
      },
      dark: {
        bgFrom: '#EED5B6', bgTo: '#AF8856', foreground: '#FFFFFF', comment: '#837E77',
        keyword: '#D3B48C', string: '#C2B181', function: '#F4A361', parameter: '#F4A361',
        number: '#C2B181', constant: '#C2B181', property: '#C2B181', punctuation: '#F4A361',
        highlight: 'rgba(244,163,97,0.14)', highlightBorder: '#F4A361', window: 'rgba(0,0,0,0.75)',
      },
    },
  };

  const DEFAULT_CODE = 'function greet(name) {\n  return `Hello, ${name}!`;\n}\n';
  const DEFAULTS = {
    code: DEFAULT_CODE,
    title: 'Untitled-1',
    subtitle: '',
    language: 'auto',
    theme: 'candy',
    dark: false,
    background: true,
    lineNumbers: false,
    padding: 64,
    font: 'jetbrains-mono',
    highlighted: '',
    width: null,
    windowChrome: true,
    scale: 4,
  };

  const CODE_KEY = 'raycopy.code';
  const SCALE_KEY = 'raycopy.scale';
  const HASH_CODE_LIMIT = 1000; // longer code lives in localStorage, not in the URL

  // ---------------------------------------------------------------------------
  // DOM handles
  // ---------------------------------------------------------------------------

  const $ = (id) => document.getElementById(id);

  const els = {
    previewPane: $('previewPane'),
    stage: $('stage'),
    frame: $('frame'),
    window: $('window'),
    bar: $('bar'),
    body: $('body'),
    lines: $('lines'),
    frameCode: $('frameCode'),
    frameTitle: $('frameTitle'),
    frameSub: $('frameSub'),
    theme: $('theme'),
    themeBtn: $('themeBtn'),
    themeMenu: $('themeMenu'),
    background: $('background'),
    darkMode: $('darkMode'),
    lineNumbers: $('lineNumbers'),
    language: $('language'),
    font: $('font'),
    windowChrome: $('windowChrome'),
    subtitle: $('subtitle'),
    width: $('width'),
    highlighted: $('highlighted'),
    paddingRadios: Array.from(document.querySelectorAll('input[name="padding"]')),
    scaleRadios: Array.from(document.querySelectorAll('input[name="scale"]')),
    formatBtn: $('formatBtn'),
    aboutBtn: $('aboutBtn'),
    aboutDialog: $('aboutDialog'),
    exportPng: $('exportPng'),
    exportMore: $('exportMore'),
    exportMenu: $('exportMenu'),
    toast: $('toast'),
  };

  // ---------------------------------------------------------------------------
  // State
  // ---------------------------------------------------------------------------

  const state = Object.assign({}, DEFAULTS);
  let busy = false;

  function hasBackend() {
    const core = window.__TAURI__ && window.__TAURI__.core;
    return !!(core && typeof core.invoke === 'function');
  }

  function call(name, args) {
    if (!hasBackend()) {
      return Promise.reject(new Error('Backend not available: open rayCopy as the desktop app'));
    }
    return window.__TAURI__.core.invoke(name, args);
  }

  function errMessage(e) {
    if (typeof e === 'string') return e;
    if (e && typeof e.message === 'string') return e.message;
    return String(e);
  }

  /** Snapshot in the exact camelCase shape of src/model.rs::Snapshot. */
  function snapshot() {
    return {
      code: state.code,
      language: state.language,
      title: state.title,
      subtitle: state.subtitle,
      theme: state.theme,
      dark: state.dark,
      background: state.background,
      lineNumbers: state.lineNumbers,
      padding: state.padding,
      font: state.font,
      highlightedLines: parseLines(state.highlighted),
      width: state.width,
      scale: state.scale,
      windowChrome: state.windowChrome,
    };
  }

  /** "2-4,7" -> [2, 3, 4, 7]. Invalid pieces are ignored. */
  function parseLines(text) {
    const seen = new Set();
    String(text || '')
      .split(',')
      .forEach((part) => {
        const m = /^\s*(\d+)\s*(?:-\s*(\d+))?\s*$/.exec(part);
        if (!m) return;
        let a = parseInt(m[1], 10);
        let b = m[2] !== undefined ? parseInt(m[2], 10) : a;
        if (a > b) [a, b] = [b, a];
        if (a < 1) a = 1;
        for (let i = a; i <= b && i - a < 5000; i++) seen.add(i);
      });
    return Array.from(seen).sort((x, y) => x - y);
  }

  // ---------------------------------------------------------------------------
  // Persistence: URL hash (state) + localStorage (code when long, scale)
  // ---------------------------------------------------------------------------

  function readHash() {
    try {
      return new URLSearchParams(location.hash.replace(/^#/, ''));
    } catch (e) {
      return new URLSearchParams();
    }
  }

  function readStorage(key) {
    try {
      return localStorage.getItem(key);
    } catch (e) {
      return null;
    }
  }

  function writeStorage(key, value) {
    try {
      localStorage.setItem(key, value);
    } catch (e) {
      /* storage unavailable: nothing to persist */
    }
  }

  function flag(v) {
    return v ? '1' : '0';
  }

  function encodeState() {
    const p = new URLSearchParams();
    p.set('theme', state.theme);
    p.set('dark', flag(state.dark));
    p.set('bg', flag(state.background));
    p.set('ln', flag(state.lineNumbers));
    p.set('pad', String(state.padding));
    p.set('font', state.font);
    p.set('lang', state.language);
    p.set('chrome', flag(state.windowChrome));
    p.set('title', state.title);
    p.set('sub', state.subtitle);
    p.set('hl', state.highlighted);
    if (state.width) p.set('w', String(state.width));
    p.set('scale', String(state.scale));
    if (state.code.length <= HASH_CODE_LIMIT) p.set('code', state.code);
    return p.toString();
  }

  function persist() {
    try {
      history.replaceState(null, '', '#' + encodeState());
    } catch (e) {
      /* hash not writable in this context */
    }
    writeStorage(CODE_KEY, state.code);
    writeStorage(SCALE_KEY, String(state.scale));
  }

  let saveTimer = null;
  function scheduleSave() {
    clearTimeout(saveTimer);
    saveTimer = setTimeout(persist, 300);
  }

  function loadState() {
    const p = readHash();
    const pick = (key, allowed, fallback) => {
      const v = p.get(key);
      return v !== null && allowed.includes(v) ? v : fallback;
    };
    const bool = (key, fallback) => (p.has(key) ? p.get(key) === '1' : fallback);

    state.theme = pick('theme', THEMES, DEFAULTS.theme);
    state.dark = bool('dark', DEFAULTS.dark);
    state.background = bool('bg', DEFAULTS.background);
    state.lineNumbers = bool('ln', DEFAULTS.lineNumbers);
    state.padding = parseInt(pick('pad', PADDINGS.map(String), String(DEFAULTS.padding)), 10);
    state.font = pick('font', Object.keys(FONTS), DEFAULTS.font);
    state.language = pick('lang', Object.keys(LANGS), DEFAULTS.language);
    state.windowChrome = bool('chrome', DEFAULTS.windowChrome);
    state.title = p.has('title') ? p.get('title') : DEFAULTS.title;
    state.subtitle = p.get('sub') || '';
    state.highlighted = p.get('hl') || '';

    const w = parseInt(p.get('w'), 10);
    state.width = w > 0 && w <= 10000 ? w : null;

    const scaleRaw = p.has('scale') ? p.get('scale') : readStorage(SCALE_KEY);
    const scale = parseInt(scaleRaw, 10);
    state.scale = SCALES.includes(scale) ? scale : DEFAULTS.scale;

    if (p.has('code')) {
      state.code = p.get('code');
    } else {
      const stored = readStorage(CODE_KEY);
      state.code = stored !== null ? stored : DEFAULT_CODE;
    }
  }

  // ---------------------------------------------------------------------------
  // Controls <-> state
  // ---------------------------------------------------------------------------

  // ---------------------------------------------------------------------------
  // Theme picker: a swatch button and a list, mirroring the hidden #theme select
  // ---------------------------------------------------------------------------

  function syncThemePicker() {
    const value = els.theme.value;
    els.themeBtn.querySelector('[data-swatch]').className = 'swatch swatch-' + value;
    els.themeMenu.querySelectorAll('[role="option"]').forEach((li) => {
      li.setAttribute('aria-selected', String(li.dataset.value === value));
    });
  }

  function setThemeMenu(open) {
    els.themeMenu.hidden = !open;
    els.themeBtn.setAttribute('aria-expanded', String(open));
    if (open) {
      const selected = els.themeMenu.querySelector('[aria-selected="true"]') || els.themeMenu.firstElementChild;
      if (selected) selected.focus();
    }
  }

  function pickTheme(value) {
    if (els.theme.value !== value) {
      els.theme.value = value;
      els.theme.dispatchEvent(new Event('change'));
    }
    setThemeMenu(false);
    els.themeBtn.focus();
  }

  function bindThemePicker() {
    els.themeBtn.addEventListener('click', (e) => {
      e.stopPropagation();
      setThemeMenu(els.themeMenu.hidden);
    });
    els.themeMenu.addEventListener('click', (e) => {
      const li = e.target.closest('[role="option"]');
      if (li) pickTheme(li.dataset.value);
    });
    els.themeMenu.addEventListener('keydown', (e) => {
      const items = Array.from(els.themeMenu.querySelectorAll('[role="option"]'));
      const i = items.indexOf(document.activeElement);
      if (e.key === 'Escape') {
        setThemeMenu(false);
        els.themeBtn.focus();
      } else if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
        e.preventDefault();
        const next = items[(i + (e.key === 'ArrowDown' ? 1 : items.length - 1)) % items.length];
        next.focus();
      } else if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        if (i >= 0) pickTheme(items[i].dataset.value);
      }
    });
    document.addEventListener('click', (e) => {
      if (!els.themeMenu.hidden && !els.themeMenu.contains(e.target) && e.target !== els.themeBtn) {
        setThemeMenu(false);
      }
    });
  }

  function setControls() {
    els.theme.value = state.theme;
    syncThemePicker();
    els.background.checked = state.background;
    els.darkMode.checked = state.dark;
    els.lineNumbers.checked = state.lineNumbers;
    els.language.value = state.language;
    els.font.value = state.font;
    els.windowChrome.checked = state.windowChrome;
    els.subtitle.value = state.subtitle;
    els.width.value = state.width === null ? '' : String(state.width);
    els.highlighted.value = state.highlighted;
    els.paddingRadios.forEach((r) => {
      r.checked = parseInt(r.value, 10) === state.padding;
    });
    els.scaleRadios.forEach((r) => {
      r.checked = parseInt(r.value, 10) === state.scale;
    });
  }

  // ---------------------------------------------------------------------------
  // Preview
  // ---------------------------------------------------------------------------

  let cache = { key: null, lines: null };
  let previewTimer = null;
  let previewSeq = 0;

  function cacheKey() {
    // Only these inputs change the tokens; everything else is pure layout.
    return JSON.stringify([state.code, state.language, state.theme, state.dark]);
  }

  function plainLines(code) {
    const pal = PALETTES[state.theme][state.dark ? 'dark' : 'light'];
    return code.split('\n').map((text) =>
      text.length ? [{ text, color: pal.foreground, italic: false }] : []
    );
  }

  function normalizeLines(lines) {
    if (!Array.isArray(lines) || lines.length === 0) return [[]];
    return lines.map((tokens) => (Array.isArray(tokens) ? tokens : []));
  }

  function schedulePreview() {
    clearTimeout(previewTimer);
    previewTimer = setTimeout(updatePreview, 120);
  }

  async function updatePreview() {
    const key = cacheKey();
    const seq = ++previewSeq;
    let lines;
    if (!hasBackend()) {
      lines = plainLines(state.code);
    } else {
      try {
        lines = await call('highlight', { snap: snapshot() });
      } catch (e) {
        toast(errMessage(e), 'error');
        lines = plainLines(state.code);
      }
    }
    if (seq !== previewSeq) return; // a newer request superseded this one
    cache = { key, lines: normalizeLines(lines) };
    renderFrame(cache.lines);
  }

  /** Re-render immediately when only layout changed; otherwise re-highlight (debounced). */
  function refresh() {
    if (cache.lines && cache.key === cacheKey()) {
      renderFrame(cache.lines);
    } else {
      schedulePreview();
    }
    scheduleSave();
  }

  function el(tag, className, text) {
    const node = document.createElement(tag);
    if (className) node.className = className;
    if (text !== undefined) node.textContent = text;
    return node;
  }

  let lastRowsKey = null;

  function renderFrame(lines) {
    const snap = snapshot();
    const mode = snap.dark ? 'dark' : 'light';
    const pal = PALETTES[snap.theme][mode];
    const frame = els.frame;
    const win = els.window;

    // Palette tokens for the frame, exposed as custom properties.
    frame.dataset.mode = mode;
    frame.style.setProperty('--bg-from', pal.bgFrom);
    frame.style.setProperty('--bg-to', pal.bgTo);
    frame.style.setProperty('--fg', pal.foreground);
    frame.style.setProperty('--comment', pal.comment);
    frame.style.setProperty('--highlight', pal.highlight);
    frame.style.setProperty('--highlight-border', pal.highlightBorder);
    frame.style.setProperty('--window-bg', pal.window);
    frame.style.setProperty('--pad', snap.padding + 'px');
    frame.classList.toggle('no-bg', !snap.background);

    win.style.setProperty('--code-font', fontStack(snap.font));
    win.style.width = snap.width ? snap.width + 'px' : '';

    // The window skeleton (title bar, text fields, code textarea) is static markup in
    // index.html and is never rebuilt here, so focus and the caret survive re-renders.
    els.bar.hidden = !snap.windowChrome;
    syncInput(els.frameTitle, snap.title);
    syncInput(els.frameSub, snap.subtitle);

    const digits = String(lines.length).length;
    // The line-number column is always in the DOM and animates in/out through CSS
    // (see .show-ln). The textarea overlay reads the same width, so typed text stays aligned.
    els.body.style.setProperty('--ln-col', `calc(${digits}ch + 34px)`);
    els.body.classList.toggle('show-ln', !!snap.lineNumbers);

    // Rebuild the rows only when their content changes. Toggling line numbers must keep
    // the same elements, otherwise the column would appear at full width without animating.
    const rowsKey = JSON.stringify([lines, snap.highlightedLines, digits]);
    if (rowsKey !== lastRowsKey) {
      lastRowsKey = rowsKey;
      const highlighted = new Set(snap.highlightedLines);
      const rows = lines.map((tokens, i) => {
        const lineNo = i + 1;
        const row = el('div', 'line');
        if (highlighted.has(lineNo)) row.classList.add('hl');

        row.append(el('span', 'ln', String(lineNo)));

        const code = el('span', 'code');
        if (tokens.length === 0) {
          code.textContent = ' ';
        } else {
          tokens.forEach((t) => {
            const span = document.createElement('span');
            span.textContent = t.text;
            if (t.color) span.style.color = t.color;
            if (t.italic) span.style.fontStyle = 'italic';
            code.append(span);
          });
        }
        row.append(code);
        return row;
      });
      els.lines.replaceChildren(...rows);
    }

    syncFrameCode();
    fitStage();
  }

  /** Writes a value into an input only when it differs and the user is not typing in it. */
  function syncInput(input, value) {
    if (document.activeElement === input) return;
    if (input.value !== value) input.value = value;
  }

  /** Keeps the in-image textarea equal to the source of truth (state.code). */
  function syncFrameCode() {
    if (els.frameCode.value !== state.code) els.frameCode.value = state.code;
  }

  function fontStack(id) {
    const f = FONTS[id] || FONTS['jetbrains-mono'];
    return `"${f.name}", ui-monospace, "SFMono-Regular", Menlo, Consolas, monospace`;
  }

  /** Scales the frame down (never up) so it fits the preview pane width. */
  function fitStage() {
    const w = els.frame.offsetWidth;
    const h = els.frame.offsetHeight;
    if (!w || !h) return;
    const avail = Math.max(0, els.previewPane.clientWidth - 48);
    const s = Math.min(1, avail / w);
    els.stage.style.width = w * s + 'px';
    els.stage.style.height = h * s + 'px';
    els.frame.style.transform = s < 1 ? `scale(${s})` : '';
  }

  // ---------------------------------------------------------------------------
  // Editor
  // ---------------------------------------------------------------------------

  function setCode(value) {
    setCodeFrom(null, value);
  }

  /** Single entry point for code edits (the code is edited inside the image). */
  function setCodeFrom(source, value) {
    state.code = value;
    if (source !== els.frameCode) els.frameCode.value = value;
    onCodeChanged();
  }

  function onCodeChanged() {
    syncFrameCode();
    if (state.language === 'auto') scheduleDetect();
    refresh();
  }

  function handleEditorKeys(e) {
    if (e.key !== 'Tab' || e.ctrlKey || e.altKey || e.metaKey) return;
    e.preventDefault();

    const ta = e.currentTarget;
    const value = ta.value;
    const s = ta.selectionStart;
    const en = ta.selectionEnd;

    if (!e.shiftKey && s === en) {
      ta.setRangeText('  ', s, en, 'end');
      ta.dispatchEvent(new Event('input', { bubbles: true }));
      return;
    }

    // Indent / outdent every line touched by the selection.
    const lineStart = value.lastIndexOf('\n', s - 1) + 1;
    let blockEnd = value.indexOf('\n', en);
    if (blockEnd === -1) blockEnd = value.length;

    const block = value.slice(lineStart, blockEnd);
    const lines = block.split('\n');
    const newLines = e.shiftKey
      ? lines.map((l) => l.replace(/^(?: {1,2}|\t)/, ''))
      : lines.map((l) => '  ' + l);
    const newBlock = newLines.join('\n');

    const firstDelta = newLines[0].length - lines[0].length;
    const totalDelta = newBlock.length - block.length;

    ta.setRangeText(newBlock, lineStart, blockEnd, 'preserve');
    ta.setSelectionRange(Math.max(lineStart, s + firstDelta), en + totalDelta);
    ta.dispatchEvent(new Event('input', { bubbles: true }));
  }

  let detectTimer = null;
  let detectSeq = 0;

  function scheduleDetect() {
    clearTimeout(detectTimer);
    detectTimer = setTimeout(detectNow, 400);
  }

  async function detectNow() {
    const seq = ++detectSeq;
    const autoOption = els.language.querySelector('option[value="auto"]');
    if (state.language !== 'auto' || !state.code.trim() || !hasBackend()) {
      autoOption.textContent = LANGS.auto;
      return;
    }
    try {
      const detected = await call('detect_language', { code: state.code });
      if (seq !== detectSeq) return;
      autoOption.textContent =
        detected && detected !== 'auto' && LANGS[detected]
          ? `Auto-Detect (${LANGS[detected]})`
          : LANGS.auto;
    } catch (e) {
      if (seq === detectSeq) toast(errMessage(e), 'error');
    }
  }

  async function formatCode() {
    try {
      const formatted = await call('format_code', { code: state.code, language: state.language });
      setCode(String(formatted));
    } catch (e) {
      toast(errMessage(e), 'error');
    }
  }

  // ---------------------------------------------------------------------------
  // Toasts
  // ---------------------------------------------------------------------------

  let toastTimer = null;

  function toast(message, kind) {
    const level = kind || 'info';
    els.toast.textContent = message;
    els.toast.className = 'toast show ' + level;
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => {
      els.toast.classList.remove('show');
    }, level === 'error' ? 4000 : 2000);
  }

  // ---------------------------------------------------------------------------
  // Export / copy
  // ---------------------------------------------------------------------------

  function setBusy(value) {
    busy = value;
    document.querySelectorAll('[data-export]').forEach((b) => {
      b.disabled = value;
    });
  }

  async function runExclusive(fn) {
    if (busy) return;
    setBusy(true);
    try {
      await fn();
    } catch (e) {
      toast(errMessage(e), 'error');
    } finally {
      setBusy(false);
    }
  }

  function exportPng() {
    return runExclusive(async () => {
      const path = await call('export_png', { snap: snapshot() });
      if (path) toast('Saved to ' + path, 'success');
      else toast('Export cancelled', 'info');
    });
  }

  function exportSvg() {
    return runExclusive(async () => {
      const path = await call('export_svg', { snap: snapshot() });
      if (path) toast('Saved to ' + path, 'success');
      else toast('Export cancelled', 'info');
    });
  }

  function copyImage() {
    return runExclusive(async () => {
      await call('copy_png', { snap: snapshot() });
      toast('PNG copied to clipboard', 'success');
    });
  }

  function legacyCopy(text) {
    const ta = document.createElement('textarea');
    ta.value = text;
    ta.setAttribute('readonly', '');
    ta.style.position = 'fixed';
    ta.style.opacity = '0';
    document.body.appendChild(ta);
    ta.select();
    let ok = false;
    try {
      ok = document.execCommand('copy');
    } catch (e) {
      ok = false;
    }
    ta.remove();
    return ok;
  }

  async function copyUrl() {
    persist();
    const url = location.href;
    let copied = false;
    try {
      if (navigator.clipboard && navigator.clipboard.writeText) {
        await navigator.clipboard.writeText(url);
        copied = true;
      }
    } catch (e) {
      copied = false;
    }
    if (!copied) copied = legacyCopy(url);
    if (copied) toast('URL copied to clipboard', 'success');
    else toast('Could not copy the URL', 'error');
  }

  // ---------------------------------------------------------------------------
  // Export menu
  // ---------------------------------------------------------------------------

  function setMenu(open) {
    els.exportMenu.hidden = !open;
    els.exportMore.setAttribute('aria-expanded', String(open));
  }

  const actions = {
    'save-png': exportPng,
    'save-svg': exportSvg,
    'copy-image': copyImage,
    'copy-url': copyUrl,
  };

  // ---------------------------------------------------------------------------
  // Wiring
  // ---------------------------------------------------------------------------

  function isTextField(target) {
    return !!(target && target.matches &&
      target.matches('input:not([type="checkbox"]):not([type="radio"]), textarea, select, [contenteditable="true"]'));
  }

  function bind() {
    // Editor
    // Editing directly in the image (like ray.so): code, title and subtitle.
    els.frameCode.addEventListener('input', () => setCodeFrom(els.frameCode, els.frameCode.value));
    els.frameCode.addEventListener('keydown', handleEditorKeys);
    els.frameTitle.addEventListener('input', () => {
      state.title = els.frameTitle.value;
      refresh();
    });
    els.frameSub.addEventListener('input', () => {
      state.subtitle = els.frameSub.value;
      els.subtitle.value = state.subtitle;
      refresh();
    });
    // Controls
    bindThemePicker();

    els.theme.addEventListener('change', () => {
      state.theme = els.theme.value;
      syncThemePicker();
      refresh();
    });
    els.background.addEventListener('change', () => {
      state.background = els.background.checked;
      refresh();
    });
    els.darkMode.addEventListener('change', () => {
      state.dark = els.darkMode.checked;
      refresh();
    });
    els.lineNumbers.addEventListener('change', () => {
      state.lineNumbers = els.lineNumbers.checked;
      refresh();
    });
    els.paddingRadios.forEach((r) => {
      r.addEventListener('change', () => {
        if (r.checked) {
          state.padding = parseInt(r.value, 10);
          refresh();
        }
      });
    });
    els.language.addEventListener('change', () => {
      state.language = els.language.value;
      scheduleDetect();
      refresh();
    });
    els.font.addEventListener('change', () => {
      state.font = els.font.value;
      refresh();
    });
    els.windowChrome.addEventListener('change', () => {
      state.windowChrome = els.windowChrome.checked;
      refresh();
    });
    els.subtitle.addEventListener('input', () => {
      state.subtitle = els.subtitle.value;
      refresh();
    });
    els.width.addEventListener('input', () => {
      const v = parseInt(els.width.value, 10);
      state.width = v > 0 && v <= 10000 ? v : null;
      refresh();
    });
    els.highlighted.addEventListener('input', () => {
      state.highlighted = els.highlighted.value;
      refresh();
    });

    // Header
    els.formatBtn.addEventListener('click', formatCode);
    els.aboutBtn.addEventListener('click', () => {
      if (typeof els.aboutDialog.showModal === 'function') els.aboutDialog.showModal();
    });
    els.exportPng.addEventListener('click', exportPng);

    els.exportMore.addEventListener('click', (e) => {
      e.stopPropagation();
      setMenu(els.exportMenu.hidden);
    });

    els.exportMenu.addEventListener('click', (e) => {
      const item = e.target.closest('[data-action]');
      if (!item) return;
      const fn = actions[item.dataset.action];
      setMenu(false);
      if (fn) fn();
    });

    els.scaleRadios.forEach((r) => {
      r.addEventListener('change', () => {
        if (!r.checked) return;
        state.scale = parseInt(r.value, 10);
        writeStorage(SCALE_KEY, String(state.scale));
        scheduleSave();
      });
    });

    document.addEventListener('click', (e) => {
      if (els.exportMenu.hidden) return;
      if (els.exportMenu.contains(e.target) || e.target === els.exportMore) return;
      setMenu(false);
    });

    document.addEventListener('keydown', (e) => {
      if (e.key === 'Escape' && !els.exportMenu.hidden) {
        setMenu(false);
        els.exportMore.focus();
        return;
      }
      if (!(e.ctrlKey || e.metaKey) || !e.shiftKey || e.altKey) return;
      const key = e.key.toLowerCase();
      if (key === 'e') {
        e.preventDefault();
        exportPng();
      } else if (key === 'c') {
        if (isTextField(e.target)) return;
        e.preventDefault();
        copyImage();
      }
    });

    // Keep the preview scaled to the pane as the window is resized.
    if (typeof ResizeObserver === 'function') {
      new ResizeObserver(fitStage).observe(els.previewPane);
    } else {
      window.addEventListener('resize', fitStage);
    }
  }

  // ---------------------------------------------------------------------------
  // Start
  // ---------------------------------------------------------------------------

  function start() {
    loadState();
    setControls();
    bind();
    renderFrame(plainLines(state.code));
    refresh();
    if (state.language === 'auto') scheduleDetect();
  }

  start();
})();
