import { describe, expect, test } from "vitest";
import { rank, canAccess, parseTier, ALL_TIERS } from "../tier";

describe("tier (TP)", () => {
  test("TP-R1 ranks ascend Silver < Gold < Platinum", () => {
    expect(rank("Silver")).toBe(1);
    expect(rank("Gold")).toBe(2);
    expect(rank("Platinum")).toBe(3);
  });

  test.each([
    ["Silver", "Silver", true],
    ["Silver", "Gold", false],
    ["Silver", "Platinum", false],
    ["Gold", "Silver", true],
    ["Gold", "Gold", true],
    ["Gold", "Platinum", false],
    ["Platinum", "Silver", true],
    ["Platinum", "Gold", true],
    ["Platinum", "Platinum", true],
  ] as const)("TP-R2 %s on min %s = %s", (p, min, expected) => {
    expect(canAccess(p, min)).toBe(expected);
  });

  test("TP-E1 parseTier accepts canonical names", () => {
    for (const t of ALL_TIERS) {
      expect(parseTier(t)).toBe(t);
    }
  });

  test("TP-E1 parseTier rejects unknown / wrong case", () => {
    expect(parseTier("Bronze")).toBeNull();
    expect(parseTier("silver")).toBeNull();
    expect(parseTier("")).toBeNull();
  });
});
