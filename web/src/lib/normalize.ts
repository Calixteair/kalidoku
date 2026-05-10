/**
 * Client-side text normalisation aligned with `core::normalize` (Rust).
 *
 * - lowercase
 * - strip combining marks (NFKD + filter)
 * - collapse whitespace
 * - drop punctuation (keep word chars + spaces)
 *
 * This is used to highlight matches and for fast local filtering. Server-side
 * remains the source of truth for actual match validation.
 */
export const normalize = (input: string): string => {
  return input
    .normalize("NFKD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/[^\p{L}\p{N}\s'-]/gu, " ")
    .replace(/\s+/g, " ")
    .trim();
};

/**
 * Split into segments where the matched substring is highlighted.
 * Returns an array of `{ text, match }` parts ready for rendering.
 */
export const highlight = (
  haystack: string,
  needle: string,
): Array<{ text: string; match: boolean }> => {
  const n = normalize(needle);
  if (n.length === 0) return [{ text: haystack, match: false }];
  const hNorm = normalize(haystack);
  const idx = hNorm.indexOf(n);
  if (idx < 0) return [{ text: haystack, match: false }];
  // Best-effort: align indexes back into original `haystack`. Because normalize
  // can drop chars, we scan original positions char-by-char until we've consumed
  // `idx` normalised chars, then `n.length` more.
  const positions: number[] = [];
  for (let i = 0; i < haystack.length; i += 1) {
    const ch = haystack.charAt(i);
    if (normalize(ch).length > 0) {
      positions.push(i);
    }
  }
  const start = positions[idx];
  const end = positions[idx + n.length - 1];
  if (start === undefined || end === undefined) {
    return [{ text: haystack, match: false }];
  }
  return [
    { text: haystack.slice(0, start), match: false },
    { text: haystack.slice(start, end + 1), match: true },
    { text: haystack.slice(end + 1), match: false },
  ].filter((p) => p.text.length > 0);
};
