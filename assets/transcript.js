/**
 * cclog transcript interactivity — self-contained, no external dependencies.
 *
 * Features:
 *   1. Message-type filter chips (OR within category, AND across categories)
 *   2. In-page session search (150ms debounce)
 *   3. Light/dark theme toggle (localStorage persistence)
 */
(function () {
  'use strict';

  // -----------------------------------------------------------------------
  // 1. Message-type filter chips
  //
  // Filter model:
  //   - Chips are grouped into categories: roles (User, Assistant) and
  //     tools (Bash, Read, Write, Edit).
  //   - OR within a category: selecting User + Assistant shows cards that
  //     are EITHER user OR assistant.
  //   - Union across categories: selecting User + Bash shows cards that
  //     match the role selection OR the tool selection.  This is
  //     deliberately NOT an intersection — a card is either a user card
  //     or a tool card, never both, so AND-across-categories would
  //     always produce an empty set for mixed role+tool selections.
  //   - No chips selected = show everything.
  //   - Filter reads data-role and data-tools attributes on each card
  //     wrapper, rendered from the Rust tool classifier so the chip set
  //     and data stay in sync.
  // -----------------------------------------------------------------------

  var FILTER_CHIPS = [
    { key: 'user',       label: 'User',      category: 'role' },
    { key: 'assistant',  label: 'Assistant',  category: 'role' },
    { key: 'Bash',       label: 'Bash',       category: 'tool' },
    { key: 'Read',       label: 'Read',       category: 'tool' },
    { key: 'Write',      label: 'Write',      category: 'tool' },
    { key: 'Edit',       label: 'Edit',       category: 'tool' }
  ];

  /** Read active filter set from URL hash, e.g. #filter=user,Bash */
  function readFilterFromHash() {
    var m = window.location.hash.match(/filter=([^&]*)/);
    if (m && m[1]) {
      return m[1].split(',').reduce(function (acc, k) { acc[k] = true; return acc; }, {});
    }
    // Default: all chips active.
    var all = {};
    FILTER_CHIPS.forEach(function (f) { all[f.key] = true; });
    return all;
  }

  function writeFilterToHash(active) {
    var keys = Object.keys(active).filter(function (k) { return active[k]; });
    var h = window.location.hash.replace(/filter=[^&]*&?/, '').replace(/&$/, '').replace(/^#$/, '');
    var params = h ? h.replace(/^#/, '') : '';
    var parts = params.split('&').filter(Boolean);
    parts = parts.filter(function (p) { return !p.startsWith('filter='); });
    if (keys.length > 0 && keys.length < FILTER_CHIPS.length) {
      parts.push('filter=' + keys.join(','));
    }
    var searchQ = window.location.hash.match(/q=([^&]*)/);
    if (searchQ && searchQ[1]) {
      parts.push('q=' + searchQ[1]);
    }
    window.location.hash = parts.length ? '#' + parts.join('&') : '';
  }

  function applyFilter(active) {
    var anyActive = Object.keys(active).some(function (k) { return active[k]; });
    // All chips selected == no filter: show everything (prevents hiding unmatched tool types).
    var allActive = FILTER_CHIPS.every(function (f) { return active[f.key]; });
    var wrappers = document.querySelectorAll('.message-card-wrapper');
    wrappers.forEach(function (w) {
      if (!anyActive || allActive) {
        w.hidden = false;
        return;
      }
      var role = w.getAttribute('data-role') || '';
      var tools = (w.getAttribute('data-tools') || '').split(/\s+/).filter(Boolean);

      // Does this card match any active role chip?
      var roleMatch = active['user'] && role === 'user' ||
                      active['assistant'] && role === 'assistant';
      // Does this card match any active tool chip?
      var toolMatch = tools.some(function (t) { return active[t]; });

      // Union across categories: the card is visible if it matches a
      // selected role OR a selected tool.  See the block comment at the
      // top of this section for why this is union, not intersection.
      var hasRoleSelection = active['user'] || active['assistant'];
      var hasToolSelection = active['Bash'] || active['Read'] || active['Write'] || active['Edit'];

      if (hasRoleSelection && hasToolSelection) {
        w.hidden = !(roleMatch || toolMatch);
      } else if (hasRoleSelection) {
        w.hidden = !roleMatch;
      } else if (hasToolSelection) {
        w.hidden = !toolMatch;
      } else {
        w.hidden = false;
      }
    });
  }

  function updateChipUI(active) {
    var chips = document.querySelectorAll('[data-filter-chip]');
    chips.forEach(function (chip) {
      var key = chip.getAttribute('data-filter-chip');
      var isActive = !!active[key];
      chip.setAttribute('aria-pressed', isActive ? 'true' : 'false');
      if (isActive) {
        chip.classList.add('filter-chip--active');
      } else {
        chip.classList.remove('filter-chip--active');
      }
    });
  }

  function initFilterChips() {
    var active = readFilterFromHash();

    // Build filter chip buttons in the main filter bar.
    var chipContainer = document.querySelector('.filter-chips');
    if (!chipContainer) return;
    chipContainer.innerHTML = '';

    FILTER_CHIPS.forEach(function (ft) {
      var btn = document.createElement('button');
      btn.type = 'button';
      btn.className = 'filter-chip';
      btn.setAttribute('data-filter-chip', ft.key);
      btn.setAttribute('aria-pressed', active[ft.key] ? 'true' : 'false');
      if (active[ft.key]) btn.classList.add('filter-chip--active');
      btn.textContent = ft.label;
      btn.addEventListener('click', function () {
        var k = this.getAttribute('data-filter-chip');
        active[k] = !active[k];
        applyFilter(active);
        updateChipUI(active);
        writeFilterToHash(active);
      });
      chipContainer.appendChild(btn);
    });

    // Apply initial filter state.
    applyFilter(active);
    updateChipUI(active);
  }

  // -----------------------------------------------------------------------
  // 2. In-page session search
  // -----------------------------------------------------------------------

  var searchDebounceTimer = null;
  var currentSearchTerm = '';

  function readSearchFromHash() {
    var m = window.location.hash.match(/q=([^&]*)/);
    return m ? decodeURIComponent(m[1]) : '';
  }

  function writeSearchToHash(term) {
    var h = window.location.hash.replace(/q=[^&]*&?/, '').replace(/&$/, '').replace(/^#$/, '');
    var parts = h ? h.replace(/^#/, '').split('&').filter(Boolean) : [];
    parts = parts.filter(function (p) { return !p.startsWith('q='); });
    if (term) {
      parts.push('q=' + encodeURIComponent(term));
    }
    // Preserve filter.
    var filterM = window.location.hash.match(/filter=([^&]*)/);
    if (filterM && filterM[1]) {
      parts.push('filter=' + filterM[1]);
    }
    window.location.hash = parts.length ? '#' + parts.join('&') : '';
  }

  function clearHighlights() {
    document.querySelectorAll('mark.search-highlight').forEach(function (mark) {
      var parent = mark.parentNode;
      if (!parent) return;
      parent.replaceChild(document.createTextNode(mark.textContent), mark);
      parent.normalize();
    });
  }

  function highlightInNode(node, term, lowerTerm) {
    if (node.nodeType === 3 /* TEXT_NODE */) {
      var text = node.textContent;
      var lowerText = text.toLowerCase();
      var idx = lowerText.indexOf(lowerTerm);
      if (idx === -1) return;
      var frag = document.createDocumentFragment();
      var last = 0;
      while (idx !== -1) {
        if (idx > last) frag.appendChild(document.createTextNode(text.slice(last, idx)));
        var mark = document.createElement('mark');
        mark.className = 'search-highlight';
        mark.textContent = text.slice(idx, idx + term.length);
        frag.appendChild(mark);
        last = idx + term.length;
        idx = lowerText.indexOf(lowerTerm, last);
      }
      if (last < text.length) frag.appendChild(document.createTextNode(text.slice(last)));
      node.parentNode.replaceChild(frag, node);
      return;
    }
    if (node.nodeType === 1 /* ELEMENT_NODE */) {
      var tag = node.tagName.toLowerCase();
      if (tag === 'script' || tag === 'style' || tag === 'mark') return;
    }
    Array.from(node.childNodes).forEach(function (child) {
      highlightInNode(child, term, lowerTerm);
    });
  }

  function applySearch(term) {
    currentSearchTerm = term;
    clearHighlights();
    var wrappers = document.querySelectorAll('.message-card-wrapper');
    var lowerTerm = term.toLowerCase();
    wrappers.forEach(function (w) {
      if (w.hasAttribute('data-search-hidden')) {
        w.removeAttribute('data-search-hidden');
      }
      if (lowerTerm && w.textContent.toLowerCase().indexOf(lowerTerm) === -1) {
        w.setAttribute('data-search-hidden', '');
      } else if (lowerTerm) {
        highlightInNode(w, term, lowerTerm);
      }
    });
    syncVisibility();
  }

  /** Combine search + filter: a card is visible only if it passes both. */
  function syncVisibility() {
    var active = readFilterFromHash();
    var anyActive = Object.keys(active).some(function (k) { return active[k]; });
    // All chips selected == no filter: show everything (prevents hiding unmatched tool types).
    var allActive = FILTER_CHIPS.every(function (f) { return active[f.key]; });
    var wrappers = document.querySelectorAll('.message-card-wrapper');
    wrappers.forEach(function (w) {
      var filterVisible;
      if (!anyActive || allActive) {
        filterVisible = true;
      } else {
        var role = w.getAttribute('data-role') || '';
        var tools = (w.getAttribute('data-tools') || '').split(/\s+/).filter(Boolean);

        var roleMatch = active['user'] && role === 'user' ||
                        active['assistant'] && role === 'assistant';
        var toolMatch = tools.some(function (t) { return active[t]; });

        var hasRoleSelection = active['user'] || active['assistant'];
        var hasToolSelection = active['Bash'] || active['Read'] || active['Write'] || active['Edit'];

        if (hasRoleSelection && hasToolSelection) {
          filterVisible = roleMatch || toolMatch;
        } else if (hasRoleSelection) {
          filterVisible = roleMatch;
        } else if (hasToolSelection) {
          filterVisible = toolMatch;
        } else {
          filterVisible = true;
        }
      }
      var searchVisible = !w.hasAttribute('data-search-hidden');
      w.hidden = !(filterVisible && searchVisible);
    });
  }

  function initSearch() {
    var input = document.querySelector('.filter-search-input');
    if (!input) return;
    var initialTerm = readSearchFromHash();
    if (initialTerm) {
      input.value = initialTerm;
      applySearch(initialTerm);
    }

    input.addEventListener('input', function () {
      var term = this.value;
      clearTimeout(searchDebounceTimer);
      searchDebounceTimer = setTimeout(function () {
        applySearch(term);
        writeSearchToHash(term);
      }, 150);
    });
  }

  // -----------------------------------------------------------------------
  // 3. Theme toggle
  // -----------------------------------------------------------------------

  function initThemeToggle() {
    var btn = document.getElementById('theme-toggle');
    if (!btn) return;

    function updateIcon() {
      var theme = document.documentElement.getAttribute('data-theme');
      btn.innerHTML = theme === 'light' ? '&#x2600;' : '&#x25D0;';
    }

    btn.addEventListener('click', function () {
      var current = document.documentElement.getAttribute('data-theme');
      var next = current === 'light' ? 'dark' : 'light';
      document.documentElement.setAttribute('data-theme', next);
      localStorage.setItem('cclog-theme', next);
      updateIcon();
    });

    updateIcon();
  }

  // -----------------------------------------------------------------------
  // 4. Expand-all / collapse-all toggle
  // -----------------------------------------------------------------------

  function initDetailsToggle() {
    var btn = document.getElementById('details-toggle-btn');
    if (!btn) return;
    var expanded = false;

    function updateLabel() {
      btn.textContent = expanded ? 'Collapse all' : 'Expand all';
    }

    btn.addEventListener('click', function () {
      var details = document.querySelectorAll('.message-list details');
      if (expanded) {
        details.forEach(function (d) { d.removeAttribute('open'); });
        expanded = false;
      } else {
        details.forEach(function (d) { d.setAttribute('open', ''); });
        expanded = true;
      }
      updateLabel();
    });
  }

  // -----------------------------------------------------------------------
  // Shared modal
  // -----------------------------------------------------------------------

  // Hoisted so initClamp (T20) can call openModal without depending on
  // initModal having run first.
  var openModal = null;

  function initModal() {
    var overlay = document.getElementById('cclog-modal');
    if (!overlay) return;
    var bodyEl = overlay.querySelector('.modal-body');
    var closeBtn = overlay.querySelector('.modal-close');

    openModal = function (html) {
      bodyEl.innerHTML = html;
      overlay.removeAttribute('hidden');
      document.body.classList.add('modal-open');
    };

    function closeModal() {
      overlay.setAttribute('hidden', '');
      document.body.classList.remove('modal-open');
      bodyEl.innerHTML = '';
    }

    // Backdrop click closes modal.
    overlay.addEventListener('click', function (e) {
      if (e.target === overlay) closeModal();
    });

    // Close button.
    closeBtn.addEventListener('click', closeModal);

    // Esc key closes modal.
    document.addEventListener('keydown', function (e) {
      if (e.key === 'Escape' && !overlay.hasAttribute('hidden')) closeModal();
    });

    // Open modal from any [data-modal] trigger.
    // data-modal="template-id"  → reads innerHTML from <template id="...">
    // data-modal-html="..."     → uses the attribute value directly as HTML
    document.addEventListener('click', function (e) {
      var trigger = e.target.closest('[data-modal]');
      if (!trigger) return;
      var templateId = trigger.getAttribute('data-modal');
      var tmpl = document.getElementById(templateId);
      var html = tmpl ? tmpl.innerHTML : (trigger.getAttribute('data-modal-html') || '');
      if (html) openModal(html);
    });
  }

  // -----------------------------------------------------------------------
  // T20 — IN/OUT clamp: detect overflow, add fade, wire click → modal
  // -----------------------------------------------------------------------

  function clampWrap(wrap) {
    if (wrap.classList.contains('is-clamped') || wrap.dataset.clampChecked) return;
    wrap.dataset.clampChecked = '1';
    var pre = wrap.querySelector('pre');
    if (!pre || pre.scrollHeight <= pre.clientHeight) return;
    wrap.classList.add('is-clamped');
    wrap.addEventListener('click', function () {
      if (openModal) {
        openModal('<pre class="modal-pre">' + escapeHtml(pre.textContent) + '</pre>');
      }
    });
  }

  function initClamp() {
    // Initial pass: catch any visible (non-details) sections.
    document.querySelectorAll('.tool-section-pre-wrap').forEach(clampWrap);

    // Re-check when a <details> is opened — content only gets layout then.
    document.addEventListener('toggle', function (e) {
      if (!e.target.open) return;
      e.target.querySelectorAll('.tool-section-pre-wrap').forEach(clampWrap);
    }, true);
  }

  // -----------------------------------------------------------------------
  // 5. Custom tooltip (data-tooltip="<text>")
  //
  // One tooltip element is shared and repositioned on each hover. Text is
  // HTML-escaped so callers can safely pass arbitrary path strings.
  // -----------------------------------------------------------------------

  var tooltipEl = null;

  function escapeHtml(str) {
    return str
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#39;');
  }

  function positionTooltip(el, triggerRect) {
    var vw = window.innerWidth;
    var vh = window.innerHeight;
    var tw = el.offsetWidth;
    var th = el.offsetHeight;
    var GAP = 6;

    // Prefer below, fall back to above.
    var top = triggerRect.bottom + GAP;
    if (top + th > vh - 4) {
      top = triggerRect.top - th - GAP;
    }
    if (top < 4) top = 4;

    // Center horizontally over trigger, clamp to viewport.
    var left = triggerRect.left + triggerRect.width / 2 - tw / 2;
    if (left + tw > vw - 4) left = vw - tw - 4;
    if (left < 4) left = 4;

    el.style.top = top + 'px';
    el.style.left = left + 'px';
  }

  function initTooltip() {
    tooltipEl = document.createElement('div');
    tooltipEl.className = 'cclog-tooltip';
    tooltipEl.setAttribute('aria-hidden', 'true');
    tooltipEl.setAttribute('hidden', '');
    document.body.appendChild(tooltipEl);

    document.addEventListener('mouseover', function (e) {
      var target = e.target.closest('[data-tooltip]');
      if (!target) return;
      var text = target.getAttribute('data-tooltip');
      if (!text) return;

      tooltipEl.innerHTML = escapeHtml(text);
      tooltipEl.removeAttribute('hidden');

      // Position after making visible so offsetWidth/Height are correct.
      var rect = target.getBoundingClientRect();
      positionTooltip(tooltipEl, rect);
    });

    document.addEventListener('mouseout', function (e) {
      var target = e.target.closest('[data-tooltip]');
      if (!target) return;
      // Hide only when leaving the trigger element itself.
      if (!target.contains(e.relatedTarget)) {
        tooltipEl.setAttribute('hidden', '');
      }
    });
  }

  // -----------------------------------------------------------------------
  // Boot
  // -----------------------------------------------------------------------

  function init() {
    initFilterChips();
    initSearch();
    initThemeToggle();
    initDetailsToggle();
    initModal();
    initTooltip();
    initClamp();
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
})();
