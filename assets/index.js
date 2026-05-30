/**
 * cclog index page interactivity — self-contained, no external dependencies.
 *
 * Features:
 *   1. View-mode toggle (cards / list) with localStorage persistence
 *   2. Search bar (case-insensitive substring, 150ms debounce)
 *   3. Date filter (preset chips + custom range picker)
 *
 * All filters combine via AND.  Inline theme + view-mode scripts in
 * <head> prevent layout flash before the full script boots.
 */
(function () {
  'use strict';

  var VIEW_KEY = 'cclog:index:viewMode';

  // -----------------------------------------------------------------------
  // 1. View-mode toggle
  // -----------------------------------------------------------------------

  function getViewMode() {
    try {
      return localStorage.getItem(VIEW_KEY) || 'cards';
    } catch (_) {
      return 'cards';
    }
  }

  function setViewMode(mode) {
    try {
      localStorage.setItem(VIEW_KEY, mode);
    } catch (_) { /* quota exceeded — degrade gracefully */ }
  }

  function applyViewMode(mode) {
    var grid = document.querySelector('.project-grid');
    if (!grid) return;
    grid.setAttribute('data-view', mode);
    var toggle = document.querySelector('[data-view-toggle]');
    if (toggle) {
      toggle.setAttribute('aria-pressed', mode === 'list' ? 'true' : 'false');
    }
  }

  // ---- List-view column sorting ----

  var currentSort = { col: null, asc: true };

  function getSortValue(card, col) {
    switch (col) {
      case 'name':
        return (card.getAttribute('data-short-name') || '').toLowerCase();
      case 'activity':
        return card.getAttribute('data-last-activity') || '';
      case 'sessions':
        return parseInt(card.getAttribute('data-sessions') || '0', 10);
      case 'messages':
        return parseInt(card.getAttribute('data-messages') || '0', 10);
      case 'tokens':
        return parseInt(card.getAttribute('data-tokens-raw') || '0', 10);
      default:
        return '';
    }
  }

  function sortCards(col, asc) {
    var grid = document.querySelector('.project-grid');
    if (!grid) return;
    var cards = Array.from(grid.querySelectorAll('.project-card'));
    cards.sort(function (a, b) {
      var va = getSortValue(a, col);
      var vb = getSortValue(b, col);
      if (typeof va === 'string') {
        return asc ? va.localeCompare(vb) : vb.localeCompare(va);
      }
      return asc ? va - vb : vb - va;
    });
    cards.forEach(function (c) { grid.appendChild(c); });
  }

  function updateSortHeaders() {
    var headers = document.querySelectorAll('[data-sort-col]');
    headers.forEach(function (h) {
      var col = h.getAttribute('data-sort-col');
      h.classList.remove('sort-asc', 'sort-desc');
      if (col === currentSort.col) {
        h.classList.add(currentSort.asc ? 'sort-asc' : 'sort-desc');
      }
    });
  }

  function initViewToggle() {
    var mode = getViewMode();
    applyViewMode(mode);

    var toggle = document.querySelector('[data-view-toggle]');
    if (!toggle) return;

    // Sync UI on first paint — inline script in <head> already set the
    // data-view attribute, so we only need to update the button state.
    toggle.setAttribute('aria-pressed', mode === 'list' ? 'true' : 'false');

    toggle.addEventListener('click', function () {
      var grid = document.querySelector('.project-grid');
      if (!grid) return;
      var current = grid.getAttribute('data-view') || 'cards';
      var next = current === 'cards' ? 'list' : 'cards';
      setViewMode(next);
      applyViewMode(next);
      // Re-apply sort if switching to list view.
      if (next === 'list' && currentSort.col) {
        sortCards(currentSort.col, currentSort.asc);
        updateSortHeaders();
      }
    });
  }

  function initSortableHeaders() {
    var headers = document.querySelectorAll('[data-sort-col]');
    headers.forEach(function (h) {
      h.addEventListener('click', function () {
        var col = h.getAttribute('data-sort-col');
        var asc = col === currentSort.col ? !currentSort.asc : true;
        currentSort = { col: col, asc: asc };
        sortCards(col, asc);
        updateSortHeaders();
      });
    });
  }

  // -----------------------------------------------------------------------
  // 2. Search bar
  // -----------------------------------------------------------------------

  var searchDebounce = null;
  var currentSearch = '';

  function applySearch(term) {
    currentSearch = term.toLowerCase();
    var cards = document.querySelectorAll('.project-card');
    cards.forEach(function (c) {
      var shortName = (c.getAttribute('data-short-name') || '').toLowerCase();
      var name = (c.getAttribute('data-name') || '').toLowerCase();
      if (currentSearch && shortName.indexOf(currentSearch) === -1 && name.indexOf(currentSearch) === -1) {
        c.setAttribute('data-search-hidden', '');
      } else {
        c.removeAttribute('data-search-hidden');
      }
    });
    syncVisibility();
  }

  function initSearch() {
    var input = document.querySelector('.index-search-input');
    if (!input) return;

    input.addEventListener('input', function () {
      var term = this.value;
      clearTimeout(searchDebounce);
      searchDebounce = setTimeout(function () {
        applySearch(term);
      }, 150);
    });
  }

  // -----------------------------------------------------------------------
  // 3. Date filter
  // -----------------------------------------------------------------------

  var currentDatePreset = 'all';
  var currentDateFrom = null;
  var currentDateTo = null;

  function toLocalDate(iso) {
    if (!iso) return null;
    // Parse ISO-8601 and convert to local date string YYYY-MM-DD.
    try {
      var d = new Date(iso);
      if (isNaN(d.getTime())) return null;
      return d.getFullYear() + '-' +
        String(d.getMonth() + 1).padStart(2, '0') + '-' +
        String(d.getDate()).padStart(2, '0');
    } catch (_) {
      return null;
    }
  }

  function todayStr() {
    var d = new Date();
    return d.getFullYear() + '-' +
      String(d.getMonth() + 1).padStart(2, '0') + '-' +
      String(d.getDate()).padStart(2, '0');
  }

  function daysAgoStr(n) {
    var d = new Date();
    d.setDate(d.getDate() - n);
    return d.getFullYear() + '-' +
      String(d.getMonth() + 1).padStart(2, '0') + '-' +
      String(d.getDate()).padStart(2, '0');
  }

  function applyDateFilter() {
    var cards = document.querySelectorAll('.project-card');
    cards.forEach(function (c) {
      // No filter → visible.
      if (currentDatePreset === 'all' && !currentDateFrom && !currentDateTo) {
        c.removeAttribute('data-date-hidden');
        return;
      }
      var activityIso = c.getAttribute('data-last-activity');
      var activityDate = toLocalDate(activityIso);
      if (!activityDate) {
        // No timestamp → show if "All time", hide otherwise.
        if (currentDatePreset === 'all' && !currentDateFrom && !currentDateTo) {
          c.removeAttribute('data-date-hidden');
        } else {
          c.setAttribute('data-date-hidden', '');
        }
        return;
      }

      var from = currentDateFrom;
      var to = currentDateTo;
      if (currentDatePreset === 'today') {
        from = todayStr();
        to = todayStr();
      } else if (currentDatePreset === '7days') {
        from = daysAgoStr(7);
        to = todayStr();
      } else if (currentDatePreset === '30days') {
        from = daysAgoStr(30);
        to = todayStr();
      }

      var visible = true;
      if (from && activityDate < from) visible = false;
      if (to && activityDate > to) visible = false;
      if (visible) {
        c.removeAttribute('data-date-hidden');
      } else {
        c.setAttribute('data-date-hidden', '');
      }
    });
    syncVisibility();
  }

  function syncDateInputs() {
    var fromInput = document.querySelector('.date-picker-from');
    var toInput = document.querySelector('.date-picker-to');
    var now = new Date();
    if (currentDatePreset === 'today') {
      if (fromInput) fromInput.value = todayStr();
      if (toInput) toInput.value = todayStr();
    } else if (currentDatePreset === '7days') {
      if (fromInput) fromInput.value = daysAgoStr(7);
      if (toInput) toInput.value = todayStr();
    } else if (currentDatePreset === '30days') {
      if (fromInput) fromInput.value = daysAgoStr(30);
      if (toInput) toInput.value = todayStr();
    } else if (currentDatePreset === 'all') {
      if (fromInput) fromInput.value = '';
      if (toInput) toInput.value = '';
    }
  }

  function updateDateChips(active) {
    var chips = document.querySelectorAll('[data-date-preset]');
    chips.forEach(function (chip) {
      var preset = chip.getAttribute('data-date-preset');
      if (preset === active) {
        chip.classList.add('date-chip--active');
        chip.setAttribute('aria-pressed', 'true');
      } else {
        chip.classList.remove('date-chip--active');
        chip.setAttribute('aria-pressed', 'false');
      }
    });
  }

  function initDateFilter() {
    // Preset chips.
    var chips = document.querySelectorAll('[data-date-preset]');
    chips.forEach(function (chip) {
      chip.addEventListener('click', function () {
        currentDatePreset = this.getAttribute('data-date-preset');
        currentDateFrom = null;
        currentDateTo = null;
        syncDateInputs();
        updateDateChips(currentDatePreset);
        applyDateFilter();
      });
    });

    // Custom range inputs.
    var fromInput = document.querySelector('.date-picker-from');
    var toInput = document.querySelector('.date-picker-to');
    var applyBtn = document.querySelector('.date-picker-apply');

    function onCustomRange() {
      currentDatePreset = 'custom';
      currentDateFrom = fromInput ? fromInput.value || null : null;
      currentDateTo = toInput ? toInput.value || null : null;
      updateDateChips('custom');
      applyDateFilter();
    }

    if (fromInput) fromInput.addEventListener('change', onCustomRange);
    if (toInput) toInput.addEventListener('change', onCustomRange);
    if (applyBtn) {
      applyBtn.addEventListener('click', function () {
        currentDatePreset = 'custom';
        currentDateFrom = fromInput ? fromInput.value || null : null;
        currentDateTo = toInput ? toInput.value || null : null;
        updateDateChips('custom');
        applyDateFilter();
      });
    }

    // Initial state: "All time" active.
    updateDateChips('all');
  }

  // -----------------------------------------------------------------------
  // Combined visibility
  // -----------------------------------------------------------------------

  function syncVisibility() {
    var cards = document.querySelectorAll('.project-card');
    cards.forEach(function (c) {
      var searchHidden = c.hasAttribute('data-search-hidden');
      var dateHidden = c.hasAttribute('data-date-hidden');
      c.hidden = searchHidden || dateHidden;
    });
  }

  // -----------------------------------------------------------------------
  // Boot
  // -----------------------------------------------------------------------

  function init() {
    initViewToggle();
    initSortableHeaders();
    initSearch();
    initDateFilter();
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
})();
