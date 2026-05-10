import { describe, it, expect } from "vitest";
import { detectLocale, formatMessage } from "./i18n.js";

describe("detectLocale", () => {
  it("uses URL ?lang=", () => {
    const url = new URL("https://x/?lang=en");
    expect(detectLocale({ url })).toBe("en");
  });
  it("falls back to cookie", () => {
    expect(detectLocale({ cookies: "locale=en; foo=1" })).toBe("en");
  });
  it("falls back to Accept-Language", () => {
    expect(detectLocale({ acceptLanguage: "en-GB,en;q=0.9,fr;q=0.8" })).toBe("en");
  });
  it("falls back to default", () => {
    expect(detectLocale({})).toBe("fr");
  });
  it("ignores unsupported tags", () => {
    expect(detectLocale({ acceptLanguage: "es-ES;q=1" })).toBe("fr");
  });
});

describe("formatMessage", () => {
  it("interpolates", () => {
    expect(formatMessage("Hello {name}", { name: "Anne" })).toBe("Hello Anne");
  });
  it("keeps unknown placeholders untouched", () => {
    expect(formatMessage("Hello {name} {age}", { name: "Anne" })).toBe("Hello Anne {age}");
  });
});
