/* st-select: a shared, accessible dropdown (Bits UI style) used across SpeakType.
 *
 * The component keeps a real native <select> in its light DOM so existing code
 * (document.getElementById, .value, .options, .replaceChildren, 'change' events)
 * keeps working untouched. The shadow root renders a styled trigger + a floating
 * popover panel with the selected item checked, mouse + keyboard navigation, and
 * full theme awareness.
 */

(function () {
  const CHECK_SVG =
    '<svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M20 6L9 17l-5-5"/></svg>';

  const CHEVRON_SVG =
    '<svg viewBox="0 0 20 20" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M6 8l4 4 4-4"/></svg>';

  class STSelect extends HTMLElement {
    static get observedAttributes() {
      return ['disabled', 'placeholder'];
    }

    constructor() {
      super();
      this._raf = null;
      this._lastSignature = '';
      this._open = false;
      this._activeIndex = -1;

      this.attachShadow({ mode: 'open' });
      this.shadowRoot.innerHTML = TEMPLATE;
      this._trigger = this.shadowRoot.querySelector('.st-trigger');
      this._valueEl = this.shadowRoot.querySelector('.st-value');
      this._chevron = this.shadowRoot.querySelector('.st-chevron');
      this._popover = this.shadowRoot.querySelector('.st-popover');
      this._list = this.shadowRoot.querySelector('.st-list');

      this._trigger.addEventListener('click', (e) => {
        e.stopPropagation();
        this.toggle();
      });
      this._trigger.addEventListener('keydown', (e) => this._onTriggerKeydown(e));

      this._boundOnDocClick = (e) => {
        if (!this.contains(e.target) && !this.shadowRoot.contains(e.target)) {
          this.close(false);
        }
      };
      this._boundOnDocKeydown = (e) => this._onDocumentKeydown(e);
      this._boundOnScroll = () => this._reposition();
      this._boundOnResize = () => this._reposition();
    }

    connectedCallback() {
      this._native = this.querySelector('select');
      if (!this._native) return;

      // Hide the native select inline (self-contained, works anywhere) — its
      // options still live in the document so existing JS stays functional.
      this._native.classList.add('st-native');
      const s = this._native.style;
      s.position = 'absolute';
      s.width = '1px';
      s.height = '1px';
      s.padding = '0';
      s.margin = '0';
      s.border = '0';
      s.background = 'none';
      s.opacity = '0';
      s.pointerEvents = 'none';
      this._native.setAttribute('aria-hidden', 'true');
      this._native.tabIndex = -1;

      if (this._native.hasAttribute('disabled')) this.setAttribute('disabled', '');

      // Keep the popover rebuilt when options are swapped in/out (replaceChildren)
      if (this._mo) this._mo.disconnect();
      this._mo = new MutationObserver(() => this._rebuildOptions());
      this._mo.observe(this._native, { childList: true, subtree: true });

      // Listen to user-driven changes on the native select.
      this._native.addEventListener('change', this._onNativeChange = () => this._syncFromNative());

      this._rebuildOptions();
      this._syncFromNative();

      // Lightweight value poll: programmatic select.value= writes do not fire
      // 'change', so compare a cheap signature each frame to stay in sync.
      if (!this._raf) {
        const tick = () => {
          this._syncFromNative();
          this._raf = requestAnimationFrame(tick);
        };
        this._raf = requestAnimationFrame(tick);
      }

      this._applyDisabled();
    }

    disconnectedCallback() {
      if (this._raf) {
        cancelAnimationFrame(this._raf);
        this._raf = null;
      }
      if (this._mo) {
        this._mo.disconnect();
        this._mo = null;
      }
      if (this._nativeChange && this._native) {
        this._native.removeEventListener('change', this._nativeChange);
      }
      this.close(false);
    }

    attributeChangedCallback(name) {
      if (name === 'disabled') this._applyDisabled();
      if (name === 'placeholder') this._syncFromNative();
    }

    get native() {
      return this._native;
    }

    get value() {
      return this._native ? this._native.value : '';
    }

    set value(v) {
      if (this._native) this._native.value = v;
      this._syncFromNative();
    }

    get open() {
      return this._open;
    }

    _applyDisabled() {
      const disabled = this.hasAttribute('disabled');
      this._trigger.disabled = disabled;
      this._trigger.classList.toggle('disabled', disabled);
      this._trigger.setAttribute('aria-disabled', String(disabled));
    }

    _syncFromNative() {
      if (!this._native) return;
      const sig = this._native.selectedIndex + '|' + this._native.value;
      if (sig === this._lastSignature) return;
      this._lastSignature = sig;

      const selected = this._native.options[this._native.selectedIndex];
      const text = selected ? selected.textContent : '';
      this._valueEl.textContent = text || this.getAttribute('placeholder') || '';
      this._valueEl.classList.toggle('placeholder', !text);

      if (this._open) this._highlightSelection();
    }

    _rebuildOptions() {
      if (!this._native) return;
      this._list.innerHTML = '';
      this._activeIndex = -1;
      const options = Array.from(this._native.options);
      options.forEach((opt, i) => {
        const li = document.createElement('li');
        li.className = 'st-option';
        li.setAttribute('role', 'option');
        li.dataset.index = String(i);
        if (opt.disabled) li.classList.add('disabled');

        const label = document.createElement('span');
        label.className = 'st-option-label';
        label.textContent = opt.textContent;
        li.appendChild(label);

        const check = document.createElement('span');
        check.className = 'st-option-check';
        check.innerHTML = CHECK_SVG;
        li.appendChild(check);

        li.addEventListener('click', (e) => {
          e.stopPropagation();
          this._select(i);
        });
        li.addEventListener('mousemove', () => {
          this._setActive(i, true);
        });

        this._list.appendChild(li);
      });
      this._highlightSelection();
    }

    _highlightSelection() {
      if (!this._native) return;
      const idx = this._native.selectedIndex;
      Array.from(this._list.children).forEach((li, i) => {
        li.classList.toggle('selected', i === idx);
        li.setAttribute('aria-selected', String(i === idx));
        if (typeof li.scrollIntoView === 'function' && i === idx && this._open) {
          const box = this._popover.getBoundingClientRect();
          const itemBox = li.getBoundingClientRect();
          if (itemBox.bottom > box.bottom || itemBox.top < box.top) {
            li.scrollIntoView({ block: 'nearest' });
          }
        }
      });
    }

    _setActive(i, scroll) {
      if (i < 0 || i >= this._list.children.length) return;
      const items = Array.from(this._list.children);
      if (items[i].classList.contains('disabled')) return;
      this._activeIndex = i;
      items.forEach((li, j) => li.classList.toggle('active', j === i));
      if (scroll) items[i].scrollIntoView({ block: 'nearest' });
    }

    toggle() {
      if (this.hasAttribute('disabled')) return;
      this.open ? this.close(true) : this.openPanel();
    }

    openPanel() {
      if (this._open || this.hasAttribute('disabled') || !this._native) return;
      this._open = true;
      this._trigger.setAttribute('aria-expanded', 'true');
      this._trigger.classList.add('open');
      this._popover.classList.add('open');
      this._rebuildOptions();
      this._highlightSelection();
      this._positionPanel();
      if (this._native.selectedIndex >= 0) this._setActive(this._native.selectedIndex, true);
      document.addEventListener('mousedown', this._boundOnDocClick, true);
      document.addEventListener('keydown', this._boundOnDocKeydown, true);
      document.addEventListener('scroll', this._boundOnScroll, true);
      window.addEventListener('resize', this._boundOnResize, true);
      this._trigger.focus();
    }

    close(focusTrigger) {
      if (!this._open) return;
      this._open = false;
      this._trigger.setAttribute('aria-expanded', 'false');
      this._trigger.classList.remove('open');
      this._popover.classList.remove('open');
      document.removeEventListener('mousedown', this._boundOnDocClick, true);
      document.removeEventListener('keydown', this._boundOnDocKeydown, true);
      document.removeEventListener('scroll', this._boundOnScroll, true);
      window.removeEventListener('resize', this._boundOnResize, true);
      if (focusTrigger) this._trigger.focus();
    }

    _positionPanel() {
      const rect = this._trigger.getBoundingClientRect();
      const panel = this._popover;
      const viewW = window.innerWidth;
      const viewH = window.innerHeight;
      const margin = 8;
      const panelW = Math.min(rect.width, viewW - margin * 2);
      let panelH = panel.offsetHeight;

      let left = Math.min(rect.left, viewW - panelW - margin);
      if (left < margin) left = margin;
      let top = rect.bottom + 6;
      const flip = top + panelH > viewH - margin && rect.top - margin > panelH;
      if (flip) top = rect.top - panelH - 6;
      if (top < margin) top = margin;
      if (top + panelH > viewH - margin) panelH = viewH - margin - top;

      panel.style.width = panelW + 'px';
      panel.style.left = left + 'px';
      panel.style.top = top + 'px';
      panel.style.maxHeight = panelH + 'px';
      if (flip) panel.classList.add('above');
      else panel.classList.remove('above');
    }

    _reposition() {
      if (this._open) this._positionPanel();
    }

    _select(i) {
      if (!this._native || !this._list.children[i]) return;
      const opt = this._native.options[i];
      if (!opt || opt.disabled) return;
      this._native.selectedIndex = i;
      this._lastSignature = '';
      this._syncFromNative();
      this._native.dispatchEvent(new Event('change', { bubbles: true }));
      this.close(true);
    }

    _onTriggerKeydown(e) {
      // While open, navigation is handled by the document handler below.
      if (this._open) return;
      if (e.key === 'Enter' || e.key === ' ' || e.key === 'ArrowDown' || e.key === 'ArrowUp') {
        e.preventDefault();
        this.openPanel();
      }
    }

    _onListKeydown(e) {
      const count = this._list.children.length;
      switch (e.key) {
        case 'ArrowDown':
          e.preventDefault();
          this._moveActive(1);
          break;
        case 'ArrowUp':
          e.preventDefault();
          this._moveActive(-1);
          break;
        case 'Home':
          e.preventDefault();
          this._setActive(0, true);
          break;
        case 'End':
          e.preventDefault();
          this._setActive(count - 1, true);
          break;
        case 'Enter':
        case ' ':
          e.preventDefault();
          if (this._activeIndex >= 0) this._select(this._activeIndex);
          else this.close(true);
          break;
        case 'Escape':
        case 'Tab':
          e.preventDefault();
          this.close(true);
          break;
        default:
          this._typeahead(e.key);
      }
    }

    _moveActive(dir) {
      const count = this._list.children.length;
      if (count === 0) return;
      let i = this._activeIndex < 0 ? (dir > 0 ? -1 : count) : this._activeIndex + dir;
      const iter = (n) => {
        if (n < 0) n = count - 1;
        if (n >= count) n = 0;
        return n;
      };
      let guard = 0;
      while (guard < count) {
        i = iter(i);
        if (!this._list.children[i].classList.contains('disabled')) break;
        i += dir;
        guard++;
      }
      this._setActive(iter(i), true);
    }

    _typeahead(key) {
      if (!/^[\w -]$/i.test(key)) return;
      const char = key.toLowerCase();
      const items = Array.from(this._list.children);
      const start = this._activeIndex < 0 ? 0 : this._activeIndex + 1;
      const len = items.length;
      for (let n = 0; n < len; n++) {
        const i = (start + n) % len;
        const label = items[i].querySelector('.st-option-label');
        if (!label) continue;
        if (!items[i].classList.contains('disabled') && label.textContent.toLowerCase().charAt(0) === char) {
          this._setActive(i, true);
          return;
        }
      }
    }

    _onDocumentKeydown(e) {
      // Only intercept keys that originate inside this component so that other
      // widgets (e.g. the language search input) keep receiving their own keys.
      const inside = this.shadowRoot.contains(e.target) || this.contains(e.target);
      if (!inside) return;
      if (this._open) this._onListKeydown(e);
      else if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
        e.preventDefault();
        this.openPanel();
      }
    }
  }

  const TEMPLATE = `
    <style>
      :host {
        display: block;
        position: relative;
        font-family: inherit;
      }
      .st-trigger {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 8px;
        width: 100%;
        padding: 10px 12px;
        background: var(--surface-2, #252525);
        border: 1px solid var(--border, #333);
        border-radius: 8px;
        color: var(--text-strong, #fff);
        font-size: 14px;
        font-family: inherit;
        text-align: left;
        cursor: pointer;
        transition: border-color .15s, box-shadow .15s, background-color .15s;
        box-sizing: border-box;
        margin: 0;
      }
      .st-trigger:hover:not(:disabled):not(.disabled) {
        background: var(--surface-3, #2a2a2a);
      }
      .st-trigger:focus-visible {
        outline: none;
        border-color: var(--accent, #3b82f6);
        box-shadow: 0 0 0 3px var(--accent-ring, rgba(59,130,246,.35));
      }
      .st-trigger.open {
        border-color: var(--accent, #3b82f6);
      }
      .st-trigger.disabled, .st-trigger:disabled {
        cursor: default;
        opacity: .55;
      }
      .st-value {
        flex: 1;
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        color: var(--text-strong, #fff);
      }
      .st-value.placeholder {
        color: var(--text-faint, #666);
      }
      .st-chevron {
        flex: 0 0 auto;
        display: inline-flex;
        align-items: center;
        justify-content: center;
        color: var(--text-muted, #94949c);
        transition: transform .18s ease;
      }
      .st-trigger.open .st-chevron {
        transform: rotate(180deg);
      }
      .st-popover {
        position: fixed;
        z-index: 9999;
        min-width: 200px;
        max-width: min(320px, calc(100vw - 16px));
        max-height: min(300px, 60vh);
        display: none;
        overflow: auto;
        padding: 4px;
        background: var(--bg-elevated, #18181b);
        border: 1px solid var(--border-strong, #555);
        border-radius: 8px;
        box-shadow: var(--elevation-2, 0 4px 14px rgba(0,0,0,.42));
        opacity: 0;
        transform: translateY(-4px);
        transition: opacity .14s ease, transform .14s ease;
      }
      .st-popover.open {
        display: block;
        opacity: 1;
        transform: translateY(0);
      }
      .st-list {
        list-style: none;
        margin: 0;
        padding: 0;
        outline: none;
      }
      .st-option {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 10px;
        padding: 8px 10px;
        border-radius: 6px;
        font-size: 14px;
        color: var(--text, #e0e0e0);
        cursor: pointer;
        user-select: none;
      }
      .st-option .st-option-label {
        flex: 1;
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
      }
      .st-option:hover, .st-option.active {
        background: var(--accent-soft, rgba(59,130,246,.15));
        color: var(--text-strong, #fff);
      }
      .st-option.disabled {
        opacity: .45;
        cursor: default;
        background: transparent;
        color: var(--text-faint, #666);
      }
      .st-option-check {
        flex: 0 0 auto;
        display: inline-flex;
        align-items: center;
        justify-content: center;
        color: var(--accent, #3b82f6);
        opacity: 0;
      }
      .st-option.selected .st-option-check {
        opacity: 1;
      }
      .st-option.selected .st-option-label {
        font-weight: 600;
        color: var(--accent-muted, #8faee8);
      }
    </style>
    <button type="button" class="st-trigger" aria-haspopup="listbox" aria-expanded="false" aria-controls="st-list">
      <span class="st-value"></span>
      <span class="st-chevron">${CHEVRON_SVG}</span>
    </button>
    <div class="st-popover">
      <ul class="st-list" id="st-list" role="listbox" tabindex="-1"></ul>
    </div>
  `;

  if (!customElements.get('st-select')) {
    customElements.define('st-select', STSelect);
  }
})();
