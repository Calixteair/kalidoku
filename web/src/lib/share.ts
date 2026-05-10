import type { components } from "./api/types.js";
import type { CellAnswer } from "./stores/gameStore.svelte.js";

type GameSummary = components["schemas"]["GameSummary"];
type EndGameView = components["schemas"]["EndGameView"];

/**
 * Build a Wordle-style emoji share string from local answers when the server's
 * `shareString` isn't available yet (offline / fallback). Backend remains the
 * source of truth and provides `summary.shareString` once the game ends.
 */
export const buildShareString = (input: {
  date: string;
  score: number;
  maxScore: number | undefined;
  mistakes: number;
  mistakesAllowed: number;
  answers: CellAnswer[];
}): string => {
  const grid: string[][] = Array.from({ length: 3 }, () => ["⬛", "⬛", "⬛"]);
  for (const a of input.answers) {
    if (a.row >= 0 && a.row < 3 && a.col >= 0 && a.col < 3) {
      const row = grid[a.row];
      if (row !== undefined) {
        row[a.col] = "🟩";
      }
    }
  }
  const maxStr = input.maxScore !== undefined ? `/${input.maxScore}` : "";
  const head = `kalidoku ${input.date} — ${input.score}${maxStr}`;
  const body = grid.map((row) => row.join("")).join("\n");
  return `${head}\n${body}`;
};

export const pickShareString = (
  summary: GameSummary | null,
  endGameView: EndGameView | null,
  fallback: () => string,
): string => {
  if (summary?.shareString) return summary.shareString;
  if (endGameView?.summary.shareString) return endGameView.summary.shareString;
  return fallback();
};

export const copyToClipboard = async (text: string): Promise<boolean> => {
  if (typeof navigator === "undefined") return false;
  if (!navigator.clipboard || typeof navigator.clipboard.writeText !== "function") {
    return false;
  }
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    return false;
  }
};

export const shareNative = async (text: string, title: string): Promise<boolean> => {
  if (typeof navigator === "undefined" || !("share" in navigator)) return false;
  try {
    await (navigator as Navigator & { share: (data: ShareData) => Promise<void> }).share({
      title,
      text,
    });
    return true;
  } catch {
    return false;
  }
};
