import { describe, expect, it } from "vitest";
import { formatBytes, formatDuration, formatRelativeDate } from "./format";

describe("format helpers", () => {
  it("formats tracked playtime without false precision", () => {
    expect(formatDuration(0)).toBe("Not played");
    expect(formatDuration(3_600)).toBe("1h");
    expect(formatDuration(3_900)).toBe("1h 5m");
  });

  it("formats portable file sizes", () => {
    expect(formatBytes(null)).toBe("Size unavailable");
    expect(formatBytes(1024 * 1024 * 1024)).toBe("1.0 GB");
  });

  it("handles missing relative dates", () => {
    expect(formatRelativeDate(null)).toBe("Never");
  });
});
