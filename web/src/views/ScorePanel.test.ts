import { describe, it, expect } from "vitest";
import { ScorePanel } from "./ScorePanel";

function mockEl(): HTMLElement {
  return { textContent: "", innerHTML: "" } as unknown as HTMLElement;
}

describe("ScorePanel", () => {
  it("update sets scoreBig.textContent to percentage string", () => {
    const scoreBig = mockEl();
    const spProfile = mockEl();
    const spRanker = mockEl();
    const spRows = mockEl();
    const spHint = mockEl();
    const panel = new ScorePanel(scoreBig, spProfile, spRanker, spRows, spHint);

    panel.update({
      score: 0.85,
      costs: [0.1, 0.2, 0.3, 0.4, 0.5, 0.6],
      weights: [0.3, 0.2, 0.2, 0.15, 0.1, 0.05],
      profileLabel: "Resin Printing",
      rankerLabel: "Consensus",
      hint: "Minimax explanation",
    });

    expect(scoreBig.textContent).toBe("85%");
  });

  it("update sets spRows.innerHTML to 6 metric bar rows", () => {
    const scoreBig = mockEl();
    const spProfile = mockEl();
    const spRanker = mockEl();
    const spRows = mockEl();
    const spHint = mockEl();
    const panel = new ScorePanel(scoreBig, spProfile, spRanker, spRows, spHint);

    panel.update({
      score: 0.7,
      costs: [0.1, 0.2, 0.3, 0.4, 0.5, 0.6],
      weights: [0.3, 0.2, 0.2, 0.15, 0.1, 0.05],
      profileLabel: "Balanced",
      rankerLabel: "TOPSIS",
      hint: "TOPSIS explanation",
    });

    expect(spProfile.textContent).toBe("Balanced");
    expect(spRanker.textContent).toBe("TOPSIS");
    expect(spHint.textContent).toBe("TOPSIS explanation");
    // Should render 6 metric bar rows (Supports, Bed Space, Layer Width, Finish, Height Risk, Hard-to-Reach)
    const html = spRows.innerHTML as string;
    expect(html).toContain("Supports");
    expect(html).toContain("Bed Space");
    expect(html).toContain("Layer Width");
    expect(html).toContain("Finish");
    expect(html).toContain("Height");
    expect(html).toContain("Hard-to-Reach");
  });
});
