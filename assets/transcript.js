/**
 * cclog transcript interactivity — self-contained, no external dependencies.
 *
 * Features:
 *   1. Message-type filter chips (Task 7.1)
 *   2. In-page session search (Task 7.4)
 *   3. Sidebar scroll-spy via IntersectionObserver (Task 7.3)
 *   4. Light/dark theme toggle (Task 7.5)
 */
(function () {
  'use strict';

  // -----------------------------------------------------------------------
  // 1. Message-type filter chips
  // -----------------------------------------------------------------------

  var FILTER_TYPES = [
    { key: 'user', label: 'User' },
    { key: 'assistant', label: 'Assistant' },
    { key: 'tool-Bash', label: 'Bash' },
    { key: 'tool-Read', label: 'Read' },
    { key: 'tool-Write', label: 'Write' },
    { key: 'tool-Edit', label: 'Edit' },
    { key: 'thinking', label: 'Thinking' }
  ];

  /** Read active filter set from URL hash, e.g. #filter=user,assistant */
  function readFilterFromHash() {
    var m = window.location.hash.match(/filter=([^&]*)/);
    if (m && m[1]) {
      return m[1].split(',').reduce(function (acc, k) { acc[k] = true; return acc; }, {});
    }
    // Default: all visible.
    var all = {};
    FILTER_TYPES.forEach(function (f) { all[f.key] = true; });
    return all;
  }

  function writeFilterToHash(active) {
    var keys = Object.keys(active).filter(function (k) { return active[k]; });
    var h = window.location.hash.replace(/filter=[^&]*&?/, '').replace(/&$/, '').replace(/^#$/, '');
    var params = h ? h.replace(/^#/, '') : '';
    var parts = params.split('&').filter(Boolean);
    parts = parts.filter(function (p) { return !p.startsWith('filter='); });
    if (keys.length < FILTER_TYPES.length) {
      parts.push('filter=' + keys.join(','));
    }
    var searchQ = window.location.hash.match(/q=([^&]*)/);
    if (searchQ && searchQ[1]) {
      parts.push('q=' + searchQ[1]);
    }
    window.location.hash = parts.length ? '#' + parts.join('&') : '';
  }

  function applyFilter(active) {
    var wrappers = document.querySelectorAll('.message-card-wrapper');
    wrappers.forEach(function (w) {
      var visible = false;
      for (var i = 0; i < FILTER_TYPES.length; i++) {
        var cls = 'message-' + FILTER_TYPES[i].key;
        if (active[FILTER_TYPES[i].key] && w.classList.contains(cls)) {
          visible = true;
          break;
        }
      }
      w.hidden = !visible;
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

    // Convert existing .filter-chip spans to buttons in the main filter bar.
    var chipContainers = document.querySelectorAll('.filter-chips, .sidebar-filter-chips');
    chipContainers.forEach(function (container) {
      // Clear existing content.
      var existingChips = container.querySelectorAll('.filter-chip, .sidebar-chip');
      existingChips.forEach(function (c) { c.remove(); });

      FILTER_TYPES.forEach(function (ft) {
        var btn = document.createElement('button');
        btn.type = 'button';
        btn.className = container.classList.contains('sidebar-filter-chips') ? 'sidebar-chip' : 'filter-chip';
        btn.setAttribute('data-filter-chip', ft.key);
        btn.setAttribute('aria-pressed', active[ft.key] ? 'true' : 'false');
        if (active[ft.key]) btn.classList.add('filter-chip--active');
        if (container.classList.contains('sidebar-filter-chips') && active[ft.key]) btn.classList.add('sidebar-chip--active');
        btn.textContent = ft.label;
        btn.addEventListener('click', function () {
          var k = this.getAttribute('data-filter-chip');
          active[k] = !active[k];
          // At least one must stay active.
          var anyActive = Object.keys(active).some(function (a) { return active[a]; });
          if (!anyActive) { active[k] = true; return; }
          applyFilter(active);
          updateChipUI(active);
          writeFilterToHash(active);
        });
        container.appendChild(btn);
      });
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

  function applySearch(term) {
    currentSearchTerm = term;
    var wrappers = document.querySelectorAll('.message-card-wrapper');
    var lowerTerm = term.toLowerCase();
    wrappers.forEach(function (w) {
      if (w.hasAttribute('data-search-hidden')) {
        w.removeAttribute('data-search-hidden');
      }
      if (lowerTerm && w.textContent.toLowerCase().indexOf(lowerTerm) === -1) {
        w.setAttribute('data-search-hidden', '');
      }
    });
    syncVisibility();
  }

  /** Combine search + filter: a card is visible only if it passes both. */
  function syncVisibility() {
    var active = readFilterFromHash();
    var wrappers = document.querySelectorAll('.message-card-wrapper');
    wrappers.forEach(function (w) {
      var filterVisible = false;
      for (var i = 0; i < FILTER_TYPES.length; i++) {
        if (active[FILTER_TYPES[i].key] && w.classList.contains('message-' + FILTER_TYPES[i].key)) {
          filterVisible = true;
          break;
        }
      }
      var searchVisible = !w.hasAttribute('data-search-hidden');
      w.hidden = !(filterVisible && searchVisible);
    });
  }

  function initSearch() {
    var inputs = document.querySelectorAll('.filter-search-input, .sidebar-search-input');
    var initialTerm = readSearchFromHash();
    if (initialTerm) {
      inputs.forEach(function (inp) { inp.value = initialTerm; });
      applySearch(initialTerm);
    }

    inputs.forEach(function (input) {
      input.addEventListener('input', function () {
        var term = this.value;
        // Sync all search inputs.
        document.querySelectorAll('.filter-search-input, .sidebar-search-input').forEach(function (inp) {
          if (inp !== input) inp.value = term;
        });
        clearTimeout(searchDebounceTimer);
        searchDebounceTimer = setTimeout(function () {
          applySearch(term);
          writeSearchToHash(term);
        }, 150);
      });
    });
  }

  // -----------------------------------------------------------------------
  // 3. Sidebar scroll-spy (IntersectionObserver)
  // -----------------------------------------------------------------------

  function initScrollSpy() {
    var sidebarItems = document.querySelectorAll('.sidebar-nav-item[href]');
    if (sidebarItems.length === 0) return;

    var targets = [];
    sidebarItems.forEach(function (item) {
      var href = item.getAttribute('href');
      if (href && href.startsWith('#msg-')) {
        var el = document.getElementById(href.slice(1));
        if (el) targets.push({ nav: item, card: el });
      }
    });
    if (targets.length === 0) return;

    var observer = new IntersectionObserver(function (entries) {
      var visible = {};
      entries.forEach(function (e) {
        if (e.isIntersecting) visible[e.target.id] = true;
      });

      var firstVisible = null;
      var minIdx = Infinity;
      targets.forEach(function (t, i) {
        if (visible[t.card.id] && i < minIdx) {
          minIdx = i;
          firstVisible = t;
        }
      });

      sidebarItems.forEach(function (item) { item.classList.remove('sidebar-nav-item--active'); });
      if (firstVisible) firstVisible.nav.classList.add('sidebar-nav-item--active');
    }, {
      root: document.getElementById('main-content'),
      rootMargin: '-80px 0px -60% 0px',
      threshold: 0
    });

    targets.forEach(function (t) { observer.observe(t.card); });
  }

  // -----------------------------------------------------------------------
  // 4. Theme toggle
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
  // Boot
  // -----------------------------------------------------------------------

  function init() {
    initFilterChips();
    initSearch();
    initScrollSpy();
    initThemeToggle();
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
})();
