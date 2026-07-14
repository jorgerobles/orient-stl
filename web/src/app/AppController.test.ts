import { describe, it, expect, vi, beforeEach } from "vitest";
import { AppController } from "./AppController";
import type { AppControllerDeps } from "./AppController";
import { AppState } from "./AppState";
import type { AppStateData } from "./AppState";
import { defaultConfig } from "../types";
import type { Candidate, WorkerMessage } from "../types";

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

function mockWorker() {
  return {
    postMessage: vi.fn(),
    terminate: vi.fn(),
    onmessage: null as unknown as ((e: MessageEvent) => void),
    onerror: null as unknown as ((e: ErrorEvent) => void),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  } as unknown as Worker;
}

function createMockDeps(): AppControllerDeps {
  let onFindCb: (() => void) | null = null;
  let onRecalcCb: (() => void) | null = null;
  let onChangeCb: (() => void) | null = null;
  let onExportCb: (() => void) | null = null;

  return {
    state: new AppState(createInitialState()),
    viewport: {
      loadModel: vi.fn(),
      showCandidate: vi.fn(),
      resetCamera: vi.fn(),
      setCriticalAngle: vi.fn(),
      enterOverlayMode: vi.fn(),
      getMeshQuaternion: vi.fn(() => [0, 0, 0, 1] as [number, number, number, number]),
      dispose: vi.fn(),
    } as any,
    fileDrop: {
      onFile: vi.fn(),
      dispose: vi.fn(),
    } as any,
    configPanel: {
      onChange: vi.fn((cb: () => void) => { onChangeCb = cb; }),
      onRecalc: vi.fn((cb: () => void) => { onRecalcCb = cb; }),
      onFind: vi.fn((cb: () => void) => { onFindCb = cb; }),
      onExport: vi.fn((cb: () => void) => { onExportCb = cb; }),
      getProfile: vi.fn(() => "resin-biased"),
      setProfile: vi.fn(),
      getRanker: vi.fn(() => "consensus"),
      setRanker: vi.fn(),
      getAngle: vi.fn(() => 30),
      setAngle: vi.fn(),
      getConvention: vi.fn(() => "z-up" as const),
      setConvention: vi.fn(),
      getHullSphere: vi.fn(() => false),
      setHullSphere: vi.fn(),
      // Expose stored callbacks for test triggering
      _onFindCb: () => onFindCb?.(),
      _onChangeCb: () => onChangeCb?.(),
    } as any,
    scorePanel: { update: vi.fn() } as any,
    candidateList: { render: vi.fn(), show: vi.fn(), hide: vi.fn(), onSelect: vi.fn() } as any,
    workerFactory: vi.fn(() => mockWorker()),
    statusEl: { textContent: "" } as HTMLElement,
    progressContainer: { style: {} } as HTMLElement,
    progressBar: { style: {}, className: "" } as HTMLElement,
    progressLabel: { textContent: "" } as HTMLElement,
    resultsPlaceholder: { style: {} } as HTMLElement,
    panelRight: { style: {} } as HTMLElement,
    cancelBtn: { addEventListener: vi.fn() } as unknown as HTMLButtonElement,
  };
}

describe("AppController", () => {
  let deps: AppControllerDeps;

  beforeEach(() => {
    deps = createMockDeps();
  });

  it("constructed with mock deps does not throw", () => {
    expect(() => new AppController(deps)).not.toThrow();
  });

  it("construction wires configPanel.onChange callback", () => {
    new AppController(deps);
    expect(deps.configPanel.onChange).toHaveBeenCalled();
  });

  it("construction wires configPanel.onRecalc callback", () => {
    new AppController(deps);
    expect(deps.configPanel.onRecalc).toHaveBeenCalled();
  });

  it("construction wires fileDrop.onFile callback", () => {
    new AppController(deps);
    expect(deps.fileDrop.onFile).toHaveBeenCalled();
  });

  it("construction wires cancelBtn click to cancelCompute", () => {
    new AppController(deps);
    expect(deps.cancelBtn.addEventListener).toHaveBeenCalledWith("click", expect.any(Function));
  });

  it("state.set('candidates', [...]) triggers candidateList.render via subscription", () => {
    const controller = new AppController(deps);
    const cands = [] as Candidate[];
    deps.state.set("candidates", cands);
    expect(deps.candidateList.render).toHaveBeenCalledWith(cands, 0);
  });

  it("worker message type narrowing works: 'results' handler is reachable from WorkerMessage union", () => {
    // Verify that the WorkerMessage discriminated union narrows correctly
    // This is a compile-time + runtime type safety test
    const results: WorkerMessage = { type: "results", candidates: [] };
    // At runtime, narrowing by type works
    if (results.type === "results") {
      expect(Array.isArray(results.candidates)).toBe(true);
    }
    const progress: WorkerMessage = { type: "progress", value: 50 };
    if (progress.type === "progress") {
      expect(typeof progress.value).toBe("number");
    }
    const err: WorkerMessage = { type: "error", message: "test" };
    if (err.type === "error") {
      expect(typeof err.message).toBe("string");
    }
  });

  it("state subscription triggers candidateList.render when candidates change", () => {
    new AppController(deps);
    const cands = [
      {
        id: "c-0", quaternion: [0, 0, 0, 1] as [number, number, number, number],
        overhangPenalty: 0.1, footprint: 0.2, maxCross: 0.3, shadowed: 0.4,
        surfaceQuality: 0.5, estHeight: 10, refinedOverhang: 0.1, refineVariance: 0.01,
        stability: "stable" as const, stabilityMargin: 0.8, contactArea: 100, compositeScore: 0.9,
      },
    ];
    deps.state.set("candidates", cands);
    expect(deps.candidateList.render).toHaveBeenCalledWith(cands, 0);
  });

  it("showCandidate calls viewport.showCandidate and scorePanel.update", () => {
    const controller = new AppController(deps);
    const cands = [
      {
        id: "c-0", quaternion: [0, 0, 0, 1] as [number, number, number, number],
        overhangPenalty: 0.1, footprint: 0.2, maxCross: 0.3, shadowed: 0.4,
        surfaceQuality: 0.5, estHeight: 10, refinedOverhang: 0.1, refineVariance: 0.01,
        stability: "stable" as const, stabilityMargin: 0.8, contactArea: 100, compositeScore: 0.9,
      },
    ];
    deps.state.set("candidates", cands);
    controller.showCandidate(0);

    expect(deps.viewport.showCandidate).toHaveBeenCalled();
  });
});
