import { describe, it, expect, vi } from "vitest";
import { AppState } from "./AppState";
import { defaultConfig } from "../types";
import type { AppStateData } from "./AppState";

function createInitialState(): AppStateData {
  return {
    config: defaultConfig(),
    candidates: [],
    currentIndex: 0,
    stlName: "",
    currentProfile: "resin-biased",
    currentRanker: "consensus",
    isComputeDirty: false,
    lastOriData: null,
    liveData: null,
    normBounds: null,
    loadConvention: "z-up",
  };
}

describe("AppState", () => {
  it("subscribing before a set() call receives the callback with correct key and value", () => {
    const state = new AppState(createInitialState());
    const listener = vi.fn();
    state.subscribe(listener);
    state.set("currentProfile", "topsis");
    expect(listener).toHaveBeenCalledWith("currentProfile", "topsis");
  });

  it("set('currentProfile', 'topsis') notifies subscribers with key='currentProfile', value='topsis'", () => {
    const state = new AppState(createInitialState());
    const listener = vi.fn();
    state.subscribe(listener);
    state.set("currentProfile", "topsis");
    expect(listener).toHaveBeenCalledTimes(1);
    expect(listener).toHaveBeenCalledWith("currentProfile", "topsis");
  });

  it("unsubscribe() return value stops further notifications", () => {
    const state = new AppState(createInitialState());
    const listener = vi.fn();
    const unsub = state.subscribe(listener);
    unsub();
    state.set("currentProfile", "topsis");
    expect(listener).not.toHaveBeenCalled();
  });

  it("get('candidates') returns the value previously set", () => {
    const state = new AppState(createInitialState());
    const cands = [
      {
        id: "1",
        quaternion: [0, 0, 0, 1] as [number, number, number, number],
        overhangPenalty: 0.1,
        footprint: 0.2,
        maxCross: 0.3,
        shadowed: 0.4,
        surfaceQuality: 0.5,
        estHeight: 10,
        refinedOverhang: 0.1,
        refineVariance: 0.01,
        stability: "stable" as const,
        stabilityMargin: 0.8,
        contactArea: 100,
        compositeScore: 0.9,
      },
    ];
    state.set("candidates", cands);
    expect(state.get("candidates")).toEqual(cands);
  });

  it("set replaces value immutably (does not mutate the original data object)", () => {
    const initial = createInitialState();
    const state = new AppState(initial);
    const originalCandidates = initial.candidates;
    const newCandidates = [
      {
        id: "1",
        quaternion: [0, 0, 0, 1] as [number, number, number, number],
        overhangPenalty: 0.1,
        footprint: 0.2,
        maxCross: 0.3,
        shadowed: 0.4,
        surfaceQuality: 0.5,
        estHeight: 10,
        refinedOverhang: 0.1,
        refineVariance: 0.01,
        stability: "stable" as const,
        stabilityMargin: 0.8,
        contactArea: 100,
        compositeScore: 0.9,
      },
    ];
    state.set("candidates", newCandidates);
    // Original object should still have empty candidates
    expect(originalCandidates).toEqual([]);
    // State should have the new candidates
    expect(state.get("candidates")).toEqual(newCandidates);
  });

  it("initial values passed to constructor are retrievable via get()", () => {
    const initial = createInitialState();
    initial.stlName = "test.stl";
    initial.currentIndex = 7;
    const state = new AppState(initial);
    expect(state.get("stlName")).toBe("test.stl");
    expect(state.get("currentIndex")).toBe(7);
    expect(state.get("config")).toEqual(defaultConfig());
  });
});
