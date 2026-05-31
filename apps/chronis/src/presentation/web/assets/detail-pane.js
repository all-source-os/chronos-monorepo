// Shared detail-pane behaviour for all four views (Dashboard, Kanban, Graph,
// Tree). Loaded by every page after htmx.min.js. Handles the expand-in-place
// full-screen toggle for #detail-pane.
//
// The detail fragment (and its expand button) is swapped in by HTMX on every
// task click, so we never bind directly to the button — instead we delegate
// from <body>, which is stable across swaps. The expanded state lives as a
// class on #detail-pane (.detail-pane--expanded), so an HTMX innerHTML swap of
// the pane's CONTENTS keeps the pane expanded while you pivot between tasks.
(function () {
  'use strict';

  var EXPANDED = 'detail-pane--expanded';
  // Where focus was before expanding, so Esc / collapse can restore it.
  var lastFocus = null;

  function pane() {
    return document.getElementById('detail-pane');
  }

  function expandBtn() {
    var p = pane();
    return p ? p.querySelector('[data-detail-expand]') : null;
  }

  function isExpanded() {
    var p = pane();
    return !!p && p.classList.contains(EXPANDED);
  }

  function setExpanded(on) {
    var p = pane();
    if (!p) return;
    p.classList.toggle(EXPANDED, on);
    var btn = expandBtn();
    if (btn) {
      btn.setAttribute('aria-expanded', on ? 'true' : 'false');
      btn.setAttribute(
        'aria-label',
        on ? 'Collapse detail' : 'Expand detail to full screen'
      );
      btn.setAttribute('title', on ? 'Collapse (Esc)' : 'Expand (full screen)');
      // ⛶ expand glyph / ✕ close glyph.
      btn.textContent = on ? '✕' : '⛶';
    }
  }

  function expand() {
    if (isExpanded()) return;
    lastFocus = document.activeElement;
    setExpanded(true);
    var p = pane();
    if (p) p.focus({ preventScroll: true });
  }

  function collapse() {
    if (!isExpanded()) return;
    setExpanded(false);
    // Restore focus to wherever it was (the originating row/card), else the
    // expand button.
    if (lastFocus && document.contains(lastFocus)) {
      lastFocus.focus();
    } else {
      var btn = expandBtn();
      if (btn) btn.focus();
    }
    lastFocus = null;
  }

  // Delegated click: the expand button is re-created on every HTMX swap.
  document.body.addEventListener('click', function (e) {
    var btn = e.target.closest && e.target.closest('[data-detail-expand]');
    if (!btn) return;
    e.preventDefault();
    if (isExpanded()) collapse();
    else expand();
  });

  // Esc collapses the expanded pane. Capture phase so it wins before any
  // per-view Escape handler (e.g. the dashboard's search-clear) — but only
  // when we are actually expanded, otherwise we leave the event alone.
  document.addEventListener(
    'keydown',
    function (e) {
      if (e.key === 'Escape' && isExpanded()) {
        e.preventDefault();
        e.stopPropagation();
        collapse();
      }
    },
    true
  );

  // If the pane's contents are replaced while expanded (pivoting to a
  // dependency), re-sync the freshly-rendered button's aria/label to the
  // current expanded state.
  document.body.addEventListener('htmx:afterSwap', function (e) {
    if (e.detail && e.detail.target && e.detail.target.id === 'detail-pane') {
      setExpanded(isExpanded());
    }
  });
})();
