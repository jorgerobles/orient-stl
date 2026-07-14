import styles from '../styles/ScorePanel.module.css';

export interface ScorePanelData {
  score: number;
  costs: number[];
  weights: number[];
  profileLabel: string;
  rankerLabel: string;
  hint: string;
}

const METRIC_NAMES = ['Overhang', 'Footprint', 'Cross-sect', 'Surface', 'Height'];
const METRIC_DESCS = [
  'Faces needing supports (lower=better)',
  'Base contact area (lower=better)',
  'Max layer material — peel force (lower=better)',
  'Axis misalignment — resin finish (higher=better)',
  'Print height — layers & time (lower=better)',
];

function qColor(q: number): string {
  return q > 0.6 ? '#27ae60' : q > 0.3 ? '#f0ad4e' : '#e74c3c';
}

export class ScorePanel {
  constructor(
    private scoreBig: HTMLElement,
    private spProfile: HTMLElement,
    private spRanker: HTMLElement,
    private spRows: HTMLElement,
    private spHint: HTMLElement,
  ) {}

  update(data: ScorePanelData): void {
    this.scoreBig.textContent = `${(data.score * 100).toFixed(0)}%`;
    this.spProfile.textContent = data.profileLabel;
    this.spRanker.textContent = data.rankerLabel;
    this.spHint.textContent = data.hint;

    this.spRows.innerHTML = data.costs
      .map((cost, i) => {
        const quality = (1 - cost) * 100;
        const wt = data.weights[i];
        return `<div class="${styles.spRow}" title="${METRIC_DESCS[i]}">
      <span class="${styles.spName}">${METRIC_NAMES[i]}</span>
      <div class="${styles.spBar}"><div class="${styles.spBarFill}" style="width:${quality.toFixed(0)}%;background:${qColor(1 - cost)}"></div></div>
      <span class="${styles.spPct}">${quality.toFixed(0)}%</span>
      <span class="${styles.spWeight}">${wt > 0 ? '×' + wt.toFixed(1) : '—'}</span>
    </div>`;
      })
      .join('');
  }
}
