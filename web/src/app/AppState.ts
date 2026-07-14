import type { OrientConfig, Candidate, OriData } from "../types";
import type { LoadConvention } from "../convention";

export interface AppStateData {
  config: OrientConfig;
  candidates: Candidate[];
  currentIndex: number;
  stlName: string;
  currentProfile: string;
  currentRanker: string;
  isComputeDirty: boolean;
  lastOriData: OriData | null;
  liveData: {
    positions: Float32Array;
    normals: Float32Array;
    areas: Float32Array;
  } | null;
  normBounds: { lo: number[]; hi: number[] } | null;
  bboxDiagonal: number;
  loadConvention: LoadConvention;
}

export class AppState extends EventTarget {
  private data: AppStateData;

  constructor(initial: AppStateData) {
    super();
    this.data = initial;
  }

  get<K extends keyof AppStateData>(key: K): AppStateData[K] {
    return this.data[key];
  }

  set<K extends keyof AppStateData>(key: K, value: AppStateData[K]): void {
    this.data = { ...this.data, [key]: value };
    this.dispatchEvent(
      new CustomEvent("change", { detail: { key, value } }),
    );
  }

  subscribe(
    listener: (key: keyof AppStateData, value: unknown) => void,
  ): () => void {
    const handler = (e: Event) => {
      const { key, value } = (e as CustomEvent).detail;
      listener(key, value);
    };
    this.addEventListener("change", handler);
    return () => this.removeEventListener("change", handler);
  }
}
