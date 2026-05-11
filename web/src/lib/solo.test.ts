import { describe, expect, it } from "vitest";

import { buildSeedUrl, parseSeed } from "./solo.js";

describe("parseSeed", () => {
  it("accepts positive integers", () => {
    expect(parseSeed("0")).toBe(0);
    expect(parseSeed("42")).toBe(42);
    expect(parseSeed("4294967295")).toBe(0xffff_ffff);
  });

  it("rejects negatives, decimals, NaN, oversized", () => {
    expect(parseSeed("-1")).toBeUndefined();
    expect(parseSeed("1.5")).toBeUndefined();
    expect(parseSeed("abc")).toBeUndefined();
    expect(parseSeed("4294967296")).toBeUndefined(); // 2^32
    expect(parseSeed("")).toBeUndefined();
  });

  it("trims whitespace", () => {
    expect(parseSeed("  314159  ")).toBe(314_159);
  });
});

describe("buildSeedUrl", () => {
  it("uses the provided base when given", () => {
    expect(buildSeedUrl(42, "https://example.com")).toBe("https://example.com/play?seed=42");
  });

  it("falls back to the relative form without base or window", () => {
    // jsdom exposes window in test env, so we force the explicit-base branch
    // for predictability instead.
    expect(buildSeedUrl(42, "")).toBe("/play?seed=42");
  });
});
