import { describe, it, expect } from "vitest";
import { buildShareString } from "./share.js";

describe("buildShareString", () => {
  it("renders a 3x3 emoji grid with correct tiles", () => {
    const out = buildShareString({
      date: "2026-05-10",
      score: 12,
      maxScore: 18,
      mistakes: 1,
      mistakesAllowed: 3,
      answers: [
        { row: 0, col: 0, entityName: "a", filledAt: "" },
        { row: 1, col: 1, entityName: "b", filledAt: "" },
        { row: 2, col: 2, entityName: "c", filledAt: "" },
      ],
    });
    expect(out.split("\n")[0]).toBe("kalidoku 2026-05-10 — 12/18");
    const lines = out.split("\n").slice(1);
    expect(lines[0]?.startsWith("🟩")).toBe(true);
    expect(lines[1]?.includes("🟩")).toBe(true);
    expect(lines[2]?.endsWith("🟩")).toBe(true);
  });
  it("omits maxScore when undefined", () => {
    const out = buildShareString({
      date: "2026-05-10",
      score: 5,
      maxScore: undefined,
      mistakes: 2,
      mistakesAllowed: 3,
      answers: [],
    });
    expect(out.split("\n")[0]).toBe("kalidoku 2026-05-10 — 5");
  });
});
