import { describe, it, expect, vi } from "vitest";
import { CandidateList } from "./CandidateList";
import type { Candidate } from "../types";

function makeCandidates(n: number): Candidate[] {
  const out: Candidate[] = [];
  for (let i = 0; i < n; i++) {
    out.push({
      id: `c-${i}`,
      quaternion: [0, 0, 0, 1],
      overhangPenalty: 0.1 * i,
      footprint: 0.2 * i,
      maxCross: 0.3 * i,
      shadowed: 0.4 * i,
      surfaceQuality: 0.5 * i,
      estHeight: 10 + i,
      refinedOverhang: 0.1 * i,
      refineVariance: 0.01,
      stability: "stable" as const,
      stabilityMargin: 0.8,
      contactArea: 100,
      compositeScore: 0.9 - i * 0.1,
    });
  }
  return out;
}

function mockListEl(): HTMLOListElement {
  return { innerHTML: "", addEventListener: vi.fn() } as unknown as HTMLOListElement;
}

function mockSectionEl(): HTMLElement {
  return { style: {} } as unknown as HTMLElement;
}

describe("CandidateList", () => {
  it("render sets innerHTML with data-index attributes for each candidate", () => {
    const listEl = mockListEl();
    const cl = new CandidateList(listEl, mockSectionEl());
    const candidates = makeCandidates(3);

    cl.render(candidates, 0);

    expect(listEl.innerHTML).toContain('data-index="0"');
    expect(listEl.innerHTML).toContain('data-index="1"');
    expect(listEl.innerHTML).toContain('data-index="2"');
  });

  it("click on a list item emits index via onSelect callback", () => {
    const listEl = mockListEl();
    // Capture addEventListener calls
    const handlers: Record<string, Function> = {};
    listEl.addEventListener = vi.fn((event: string, handler: any) => {
      handlers[event] = handler;
    }) as any;

    const cl = new CandidateList(listEl, mockSectionEl());
    const candidates = makeCandidates(3);
    cl.render(candidates, 0);

    const onSelect = vi.fn();
    cl.onSelect(onSelect);

    // Simulate click on the second item
    const mockTarget = {
      closest: (_sel: string) => mockTarget,
      dataset: { index: "1" },
    } as unknown as HTMLElement;
    handlers.click?.({ target: mockTarget } as unknown as MouseEvent);

    expect(onSelect).toHaveBeenCalledWith(1);
  });
});
