// ─── Decimation ───
export const DECIMATE_TARGET = 12000;

// ─── Persistence ───
export const STORAGE_KEY = 'orient-stl-config';
export const SCHEMA_VERSION = 1;

// ─── Worker defaults ───
export const MIN_ANGLE_DEG = 15;

// ─── Scoring defaults ───
export const DEFAULT_REFINE_SEED = 42;
export const DEFAULT_PROFILE = 'resin-biased';
export const DEFAULT_RANKER = 'consensus';

// ─── Worker metrics layout ───
export const METRIC_STRIDE = 13;

// ─── File limits ───
export const MAX_FILE_BYTES = 104857600; // 100 MB

// ─── Camera / viewport ───
export const CAMERA_FOV = 45;
export const CAMERA_NEAR = 0.1;
export const CAMERA_FAR = 1000;
export const INITIAL_CAMERA_POS = [30, 20, 30] as const;
export const PLATE_SIZE = 60;
export const CAMERA_DIST_MULT = 2.5;
export const RING_SCALE = 1.3;
export const CAMERA_RING_SCALE = 1.6;
export const TUBE_MIN_RATIO = 0.006;
export const ANGLE_EPSILON = 0.0001;
