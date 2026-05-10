import { describe, it, expect } from "vitest";
import { normalize, highlight } from "./normalize.js";

describe("normalize", () => {
  it("lowercases", () => {
    expect(normalize("Châtelet")).toBe("chatelet");
  });
  it("strips combining marks", () => {
    expect(normalize("Étoile")).toBe("etoile");
  });
  it("collapses whitespace", () => {
    expect(normalize("  La   Défense  ")).toBe("la defense");
  });
  it("strips punctuation but keeps apostrophes and hyphens", () => {
    expect(normalize("Charles-de-Gaulle, Étoile!")).toBe("charles-de-gaulle etoile");
  });
});

describe("highlight", () => {
  it("returns single non-match for empty needle", () => {
    expect(highlight("Étoile", "")).toEqual([{ text: "Étoile", match: false }]);
  });
  it("returns single non-match if no hit", () => {
    expect(highlight("Étoile", "xyz")).toEqual([{ text: "Étoile", match: false }]);
  });
  it("highlights a prefix", () => {
    const parts = highlight("Étoile", "éto");
    expect(parts.length).toBeGreaterThanOrEqual(2);
    expect(parts[0]?.match).toBe(true);
  });
});
