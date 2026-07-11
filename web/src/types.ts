export interface OrientConfig {
  mode: "hull" | "hull_plus_sphere";
  criticalAngleDeg: number;
  dedupeAngleDeg: number;
  refineIterations: number;
  excludeUnstable: boolean;
  maxCandidates: number;
}

export function defaultConfig(): OrientConfig {
  return {
    mode: "hull",
    criticalAngleDeg: 30,
    dedupeAngleDeg: 3,
    refineIterations: 0,
    excludeUnstable: true,
    maxCandidates: 10,
  };
}
