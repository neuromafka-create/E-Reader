/**
 * Reading position encoding.
 *
 * Formats (newest first):
 * - sel:v1:start=<n>&end=<n>&q=<base64url quote>&doc=<0-1>  (text selection bookmark)
 * - v1:anchor=<id>&rel=<0-1>&doc=<0-1>                       (scroll progress)
 * - scroll:<pixels>                                           (legacy)
 *
 * `doc` is the primary restore key for progress (fraction of scrollable height).
 * Selection bookmarks jump to a character range inside the reader container.
 */

export interface ReadingPosition {
  /** Element id near the top of the viewport, if any */
  anchor: string | null;
  /** 0–1 progress within the anchor element */
  rel: number;
  /** 0–1 progress through the whole document */
  doc: number;
  /** Only for legacy scroll:px restore */
  legacyPixels?: number;
}

/** Bookmark tied to a selected text span inside the reader container. */
export interface SelectionBookmark {
  /** UTF-16-ish JS string offsets within container textContent */
  start: number;
  end: number;
  /** Exact selected text (normalized whitespace kept as in DOM text) */
  quote: string;
  /** Approximate doc fraction as fallback */
  doc: number;
  /**
   * Virtual page number (1-based), derived from selection Y / viewport height.
   * Continuous-scroll reader has no fixed pages; this is a stable-enough screen index.
   */
  page: number;
  /** Total virtual pages at capture time (optional, for display like "3 / 40") */
  pageCount: number;
}

const V1_PREFIX = "v1:";
const SEL_PREFIX = "sel:v1:";

export function encodePosition(pos: ReadingPosition): string {
  const params = new URLSearchParams();
  if (pos.anchor) params.set("anchor", pos.anchor);
  params.set("rel", clamp01(pos.rel).toFixed(4));
  params.set("doc", clamp01(pos.doc).toFixed(4));
  return `${V1_PREFIX}${params.toString()}`;
}

export function encodeSelectionBookmark(sel: SelectionBookmark): string {
  const params = new URLSearchParams();
  params.set("start", String(Math.max(0, Math.floor(sel.start))));
  params.set("end", String(Math.max(0, Math.floor(sel.end))));
  params.set("q", toBase64Url(sel.quote));
  params.set("doc", clamp01(sel.doc).toFixed(4));
  params.set("page", String(Math.max(1, Math.floor(sel.page || 1))));
  params.set("pages", String(Math.max(1, Math.floor(sel.pageCount || 1))));
  return `${SEL_PREFIX}${params.toString()}`;
}

export function parseSelectionBookmark(
  raw: string | null | undefined,
): SelectionBookmark | null {
  if (!raw?.startsWith(SEL_PREFIX)) return null;
  const q = new URLSearchParams(raw.slice(SEL_PREFIX.length));
  const start = Math.floor(num(q.get("start"), 0));
  const end = Math.floor(num(q.get("end"), start));
  const quote = fromBase64Url(q.get("q") ?? "") ?? "";
  if (!quote && end <= start) return null;
  const hasPage = q.has("page");
  const page = hasPage ? Math.max(1, Math.floor(num(q.get("page"), 1))) : 0;
  const pageCount = q.has("pages")
    ? Math.max(1, Math.floor(num(q.get("pages"), 1)))
    : 0;
  return {
    start,
    end: Math.max(start, end),
    quote,
    doc: num(q.get("doc"), 0),
    // 0 = unknown (legacy bookmarks without page)
    page,
    pageCount,
  };
}

export function parsePosition(raw: string | null | undefined): ReadingPosition | null {
  if (!raw) return null;

  // Selection bookmarks are handled separately for jump, but also map to doc.
  const sel = parseSelectionBookmark(raw);
  if (sel) {
    return { anchor: null, rel: 0, doc: sel.doc };
  }

  if (raw.startsWith(V1_PREFIX)) {
    const q = new URLSearchParams(raw.slice(V1_PREFIX.length));
    return {
      anchor: q.get("anchor"),
      rel: num(q.get("rel"), 0),
      doc: num(q.get("doc"), 0),
    };
  }

  const scrollMatch = raw.match(/^scroll:(\d+(?:\.\d+)?)$/);
  if (scrollMatch) {
    return {
      anchor: null,
      rel: 0,
      doc: 0,
      legacyPixels: Number(scrollMatch[1]),
    };
  }

  // Bare element id
  if (/^[\w.-]+$/.test(raw)) {
    return { anchor: raw, rel: 0, doc: 0 };
  }

  return null;
}

/**
 * Capture current text selection relative to the reader container.
 * Returns null if there is no non-empty selection inside the container.
 */
export function captureTextSelection(
  container: HTMLElement,
): SelectionBookmark | null {
  const selection = window.getSelection();
  if (!selection || selection.rangeCount === 0 || selection.isCollapsed) {
    return null;
  }

  const range = selection.getRangeAt(0);
  if (!container.contains(range.commonAncestorContainer)) {
    return null;
  }

  const quote = range.toString().replace(/\s+/g, " ").trim();
  if (!quote) return null;

  const pre = range.cloneRange();
  pre.selectNodeContents(container);
  pre.setEnd(range.startContainer, range.startOffset);
  const start = pre.toString().length;
  const end = start + range.toString().length;

  const { page, pageCount } = pageFromRange(container, range);

  return {
    start,
    end,
    quote,
    doc: docFraction(container),
    page,
    pageCount,
  };
}

/**
 * Virtual page index from a point/range inside the scroll container.
 * Page height ≈ one screen (clientHeight) of the reader.
 */
export function pageFromRange(
  container: HTMLElement,
  range: Range,
): { page: number; pageCount: number } {
  const pageHeight = Math.max(1, container.clientHeight);
  const pageCount = Math.max(1, Math.ceil(container.scrollHeight / pageHeight));

  const cRect = container.getBoundingClientRect();
  const rRect = range.getBoundingClientRect();
  // If range has zero size (rare), fall back to scroll position
  const y =
    rRect.height > 0 || rRect.width > 0
      ? rRect.top - cRect.top + container.scrollTop
      : container.scrollTop;

  const page = Math.min(pageCount, Math.max(1, Math.floor(y / pageHeight) + 1));
  return { page, pageCount };
}

/** Short quote for the bookmark list (ellipsis if long). */
export function selectionQuoteLabel(quote: string, maxLen = 72): string {
  const clean = quote.replace(/\s+/g, " ").trim();
  if (clean.length <= maxLen) return clean;
  return `${clean.slice(0, maxLen - 1)}…`;
}

/**
 * Full bookmark list label: quote + page number.
 * @param pageLabel e.g. localized "стр. {n}" or "p. {n}" — pass already formatted page part
 */
export function selectionLabel(
  quote: string,
  page?: number | null,
  pagePrefix = "стр.",
  maxLen = 72,
): string {
  const q = selectionQuoteLabel(quote, maxLen);
  if (page != null && page > 0) {
    return `${q} · ${pagePrefix} ${page}`;
  }
  return q;
}

/**
 * Scroll/jump to a selection bookmark. Prefer exact offsets; fall back to quote search.
 * Temporarily highlights the range when found.
 */
export function jumpToSelection(
  container: HTMLElement,
  raw: string,
  smooth = true,
): boolean {
  const sel = parseSelectionBookmark(raw);
  if (!sel) {
    restorePosition(container, raw);
    return false;
  }

  let range =
    rangeFromOffsets(container, sel.start, sel.end) ??
    rangeFromQuoteSearch(container, sel.quote);

  if (!range) {
    // Last resort: approximate scroll
    const max = Math.max(0, container.scrollHeight - container.clientHeight);
    container.scrollTop = sel.doc * max;
    return false;
  }

  const marker = document.createElement("span");
  marker.className = "bookmark-highlight";
  try {
    range.surroundContents(marker);
  } catch {
    // surroundContents fails on partial non-text nodes — use extract/insert
    try {
      const frag = range.extractContents();
      marker.appendChild(frag);
      range.insertNode(marker);
    } catch {
      // Final fallback: scroll to range start container
      const node = range.startContainer;
      const el =
        node.nodeType === Node.ELEMENT_NODE
          ? (node as HTMLElement)
          : node.parentElement;
      el?.scrollIntoView({ behavior: smooth ? "smooth" : "auto", block: "center" });
      return true;
    }
  }

  marker.scrollIntoView({ behavior: smooth ? "smooth" : "auto", block: "center" });

  window.setTimeout(() => {
    // Unwrap highlight, keep text
    const parent = marker.parentNode;
    if (!parent) return;
    while (marker.firstChild) {
      parent.insertBefore(marker.firstChild, marker);
    }
    parent.removeChild(marker);
    parent.normalize();
  }, 1800);

  return true;
}

/**
 * Capture position from a scrollable reader container that holds the book HTML.
 */
export function capturePosition(container: HTMLElement): ReadingPosition {
  const max = Math.max(0, container.scrollHeight - container.clientHeight);
  const top = container.scrollTop;
  const doc = max > 0 ? top / max : 0;

  const anchorEl = findAnchorNearTop(container, top);
  let rel = 0;
  let anchor: string | null = null;

  if (anchorEl?.id) {
    anchor = anchorEl.id;
    const elTop = offsetTopWithin(anchorEl, container);
    const elHeight = Math.max(1, anchorEl.offsetHeight);
    rel = clamp01((top - elTop) / elHeight);
  }

  return { anchor, rel, doc: clamp01(doc) };
}

/**
 * Restore a saved position into the container. Returns achieved doc fraction.
 * For selection bookmarks, jumps to the text range.
 */
export function restorePosition(
  container: HTMLElement,
  raw: string | null | undefined,
): number {
  if (raw && parseSelectionBookmark(raw)) {
    jumpToSelection(container, raw, false);
    return docFraction(container);
  }

  const pos = parsePosition(raw);
  if (!pos) return 0;

  if (pos.legacyPixels != null) {
    container.scrollTop = pos.legacyPixels;
    return docFraction(container);
  }

  const max = Math.max(0, container.scrollHeight - container.clientHeight);

  // Prefer document fraction — stable across moderate reflow.
  container.scrollTop = pos.doc * max;

  // Refine with anchor when available (helps after font-size change).
  if (pos.anchor) {
    const el = container.querySelector(`#${cssEscape(pos.anchor)}`);
    if (el instanceof HTMLElement) {
      const elTop = offsetTopWithin(el, container);
      const elHeight = Math.max(1, el.offsetHeight);
      const target = elTop + pos.rel * elHeight;
      const fromDoc = pos.doc * max;
      if (Math.abs(fromDoc - target) > container.clientHeight * 0.35) {
        container.scrollTop = clamp(target, 0, max);
      }
    }
  }

  return docFraction(container);
}

export function jumpToAnchor(container: HTMLElement, id: string, smooth = true) {
  const el = container.querySelector(`#${cssEscape(id)}`);
  if (el instanceof HTMLElement) {
    el.scrollIntoView({ behavior: smooth ? "smooth" : "auto", block: "start" });
  }
}

export function activeAnchorId(container: HTMLElement, knownIds: string[]): string | null {
  if (!knownIds.length) return null;
  const top = container.scrollTop + 8;
  let current: string | null = null;
  for (const id of knownIds) {
    const el = container.querySelector(`#${cssEscape(id)}`);
    if (!(el instanceof HTMLElement)) continue;
    const elTop = offsetTopWithin(el, container);
    if (elTop <= top) current = id;
    else break;
  }
  return current;
}

export function formatProgressLabel(percentage: number, anchorTitle?: string | null): string {
  const pct = Math.round(clamp01(percentage / 100) * 100);
  if (anchorTitle) return `${pct}% · ${anchorTitle}`;
  return `${pct}%`;
}

function findAnchorNearTop(container: HTMLElement, scrollTop: number): HTMLElement | null {
  const nodes = container.querySelectorAll<HTMLElement>(
    ".epub-chapter[id], .fb2-section[id], h1[id], h2[id], h3[id], section[id]",
  );
  let best: HTMLElement | null = null;
  let bestTop = -Infinity;
  for (const el of nodes) {
    if (!el.id) continue;
    const t = offsetTopWithin(el, container);
    if (t <= scrollTop + 4 && t >= bestTop) {
      best = el;
      bestTop = t;
    }
  }
  return best;
}

function offsetTopWithin(el: HTMLElement, container: HTMLElement): number {
  const cRect = container.getBoundingClientRect();
  const eRect = el.getBoundingClientRect();
  return eRect.top - cRect.top + container.scrollTop;
}

function docFraction(container: HTMLElement): number {
  const max = Math.max(0, container.scrollHeight - container.clientHeight);
  return max > 0 ? clamp01(container.scrollTop / max) : 0;
}

function clamp01(n: number): number {
  if (Number.isNaN(n)) return 0;
  return Math.min(1, Math.max(0, n));
}

function clamp(n: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, n));
}

function num(v: string | null, fallback: number): number {
  if (v == null || v === "") return fallback;
  const n = Number(v);
  return Number.isFinite(n) ? n : fallback;
}

function cssEscape(id: string): string {
  if (typeof CSS !== "undefined" && typeof CSS.escape === "function") {
    return CSS.escape(id);
  }
  return id.replace(/([^a-zA-Z0-9_-])/g, "\\$1");
}

function toBase64Url(text: string): string {
  const bytes = new TextEncoder().encode(text);
  let bin = "";
  for (const b of bytes) bin += String.fromCharCode(b);
  return btoa(bin).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

function fromBase64Url(encoded: string): string | null {
  if (!encoded) return null;
  try {
    const b64 = encoded.replace(/-/g, "+").replace(/_/g, "/");
    const pad = b64.length % 4 === 0 ? "" : "=".repeat(4 - (b64.length % 4));
    const bin = atob(b64 + pad);
    const bytes = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
    return new TextDecoder().decode(bytes);
  } catch {
    return null;
  }
}

/** Build a Range covering [start, end) character offsets in container's text. */
function rangeFromOffsets(
  container: HTMLElement,
  start: number,
  end: number,
): Range | null {
  const points = { start: null as { node: Text; offset: number } | null, end: null as { node: Text; offset: number } | null };
  let walked = 0;

  const walker = document.createTreeWalker(container, NodeFilter.SHOW_TEXT);
  let node = walker.nextNode() as Text | null;
  while (node) {
    const len = node.data.length;
    if (!points.start && walked + len >= start) {
      points.start = { node, offset: Math.min(len, Math.max(0, start - walked)) };
    }
    if (!points.end && walked + len >= end) {
      points.end = { node, offset: Math.min(len, Math.max(0, end - walked)) };
      break;
    }
    walked += len;
    node = walker.nextNode() as Text | null;
  }

  if (!points.start) return null;
  if (!points.end) {
    points.end = points.start;
  }

  try {
    const range = document.createRange();
    range.setStart(points.start.node, points.start.offset);
    range.setEnd(points.end.node, points.end.offset);
    return range;
  } catch {
    return null;
  }
}

/** Find first occurrence of quote in container text and return a Range. */
function rangeFromQuoteSearch(container: HTMLElement, quote: string): Range | null {
  const needle = quote.replace(/\s+/g, " ").trim();
  if (!needle) return null;

  // Build full text and map offsets → nodes (whitespace-collapsed search is harder;
  // use exact substring on concatenated textContent first).
  const full = container.textContent ?? "";
  let idx = full.indexOf(quote);
  if (idx < 0) {
    // Try collapsed whitespace match: find via normalize
    const collapsedFull = full.replace(/\s+/g, " ");
    const cIdx = collapsedFull.indexOf(needle);
    if (cIdx < 0) return null;
    // Map collapsed index approximately by walking original
    idx = mapCollapsedIndex(full, cIdx);
  }
  if (idx < 0) return null;
  return rangeFromOffsets(container, idx, idx + quote.length);
}

function mapCollapsedIndex(original: string, collapsedIndex: number): number {
  let ci = 0;
  let inSpace = false;
  for (let i = 0; i < original.length; i++) {
    const ch = original[i];
    const space = /\s/.test(ch);
    if (space) {
      if (!inSpace) {
        if (ci === collapsedIndex) return i;
        ci += 1;
        inSpace = true;
      }
    } else {
      if (ci === collapsedIndex) return i;
      ci += 1;
      inSpace = false;
    }
  }
  return -1;
}
