import { WEIGHT_PRESETS } from '../profiles';
import type { LoadConvention } from '../convention';

const PROFILE_LABELS: Record<string, string> = {
  'overhang-only': 'Minimize Supports',
  'footprint-only': 'Minimize Footprint',
  'cross-only': 'Structural Strength',
  'surface-only': 'Best Surface Quality',
  'height-only': 'Fast Print',
  equal: 'Balanced',
  'resin-biased': 'Resin Printing',
  'overhang-footprint': 'Support + Footprint',
};
const RANKER_LABELS: Record<string, string> = { consensus: 'Consensus', weights: 'Weighted Sum', topsis: 'TOPSIS' };
const RANKER_HINTS: Record<string, string> = {
  consensus: 'Minimax: score = 1 − worst normalized cost across all metrics. Bars show per-metric quality vs candidate range.',
  weights: 'Weighted average of normalized costs using profile weights. Each metric contributes proportionally to its weight.',
  topsis: 'TOPSIS: ranks by distance to ideal vs anti-ideal solution. Falls back to consensus for live display.',
};
export { PROFILE_LABELS, RANKER_LABELS, RANKER_HINTS };

export class ConfigPanel {
  private _profile: string;
  private _ranker: string;
  onChangeCb: (() => void) | null = null;
  onRecalcCb: (() => void) | null = null;
  onFindCb: (() => void) | null = null;
  onExportCb: (() => void) | null = null;
  onAutoRepairCb: ((v: boolean) => void) | null = null;

  constructor(
    private angleSlider: HTMLInputElement,
    private angleValue: HTMLElement,
    private hullSphereToggle: HTMLInputElement,
    private autoRepairToggle: HTMLInputElement,
    private conventionSelect: HTMLSelectElement,
    private profileSelect: HTMLSelectElement,
    private rankerSelect: HTMLSelectElement,
    private findBtn: HTMLButtonElement,
    private exportBtn: HTMLButtonElement,
    private recalcBtn: HTMLButtonElement,
  ) {
    this._profile = profileSelect.value;
    this._ranker = rankerSelect.value;
    profileSelect.innerHTML = Object.keys(WEIGHT_PRESETS).map(n =>
      `<option value="${n}" ${n === this._profile ? 'selected' : ''}>${PROFILE_LABELS[n] ?? n}</option>`,
    ).join('');

    angleSlider.addEventListener('input', () => { this.angleValue.textContent = angleSlider.value; this.onChangeCb?.(); });
    hullSphereToggle.addEventListener('change', () => this.onChangeCb?.());
    autoRepairToggle.addEventListener('change', () => this.onAutoRepairCb?.(autoRepairToggle.checked));
    conventionSelect.addEventListener('change', () => this.onChangeCb?.());
    profileSelect.addEventListener('change', () => { this._profile = profileSelect.value; this.onChangeCb?.(); });
    rankerSelect.addEventListener('change', () => { this._ranker = rankerSelect.value; this.onChangeCb?.(); });
    recalcBtn.addEventListener('click', () => this.onRecalcCb?.());
    findBtn.addEventListener('click', () => this.onFindCb?.());
    exportBtn.addEventListener('click', () => this.onExportCb?.());
  }

  onChange(cb: () => void): void { this.onChangeCb = cb; }
  onRecalc(cb: () => void): void { this.onRecalcCb = cb; }
  onFind(cb: () => void): void { this.onFindCb = cb; }
  onExport(cb: () => void): void { this.onExportCb = cb; }
  onAutoRepair(cb: (v: boolean) => void): void { this.onAutoRepairCb = cb; }

  getProfile(): string { return this._profile; }
  setProfile(v: string): void { this._profile = v; this.profileSelect.value = v; }
  getRanker(): string { return this._ranker; }
  setRanker(v: string): void { this._ranker = v; this.rankerSelect.value = v; }
  getAngle(): number { return parseFloat(this.angleSlider.value); }
  setAngle(deg: number): void { this.angleSlider.value = String(deg); this.angleValue.textContent = String(deg); }
  getConvention(): LoadConvention { return this.conventionSelect.value as LoadConvention; }
  setConvention(v: LoadConvention): void { this.conventionSelect.value = v; }
  getHullSphere(): boolean { return this.hullSphereToggle.checked; }
  setHullSphere(v: boolean): void { this.hullSphereToggle.checked = v; }
  getAutoRepair(): boolean { return this.autoRepairToggle.checked; }
  setAutoRepair(v: boolean): void { this.autoRepairToggle.checked = v; }

  enableFind(v: boolean): void { this.findBtn.disabled = !v; }
  enableExport(v: boolean): void { this.exportBtn.disabled = !v; }
  enableRecalc(v: boolean): void { this.recalcBtn.disabled = !v; }
}
