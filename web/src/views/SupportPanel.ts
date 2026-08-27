import type { SupportConfig } from '../types';
import styles from './SupportPanel.module.css';

export function defaultSupportConfig(): SupportConfig {
  return {
    layerHeight: 0.05,
    lightThreshold: 50,
    mediumThreshold: 500,
    lightTipDiameter: 0.25,
    mediumTipDiameter: 0.40,
    heavyTipDiameter: 0.80,
    lightPenetration: 0.3,
    mediumPenetration: 0.5,
    heavyPenetration: 0.8,
    lightSpacing: [2, 4],
    mediumSpacing: [3, 6],
    heavySpacing: [5, 10],
    raftThickness: 0.5,
    raftLineWidth: 0.3,
    cellSize: 0.5,
  };
}

export interface SupportPanelState {
  enabled: boolean;
  config: SupportConfig;
}

export class SupportPanel {
  private root: HTMLElement;
  private enabled = false;
  private config: SupportConfig = defaultSupportConfig();
  private changeCb: ((state: SupportPanelState) => void) | null = null;

  constructor(container: HTMLElement) {
    this.root = document.createElement('section');
    this.root.className = styles.panel;
    this.root.innerHTML = `
      <h3>Support Generation</h3>
      <label class="${styles.toggle}">
        <input type="checkbox" id="support-toggle" />
        <span>Enable supports</span>
      </label>
      <div class="${styles.config}" id="support-config" hidden>
        <label>Layer height: <input type="number" id="support-layer" value="0.05" step="0.01" min="0.01" max="0.1" /> mm</label>
        <label>Light threshold: <input type="number" id="support-light" value="50" step="10" min="0" /> mm³</label>
        <label>Medium threshold: <input type="number" id="support-medium" value="500" step="50" min="0" /> mm³</label>
        <label>Light tip: <input type="number" id="support-light-tip" value="0.25" step="0.05" min="0.1" /> mm</label>
        <label>Medium tip: <input type="number" id="support-medium-tip" value="0.40" step="0.05" min="0.1" /> mm</label>
        <label>Heavy tip: <input type="number" id="support-heavy-tip" value="0.80" step="0.05" min="0.1" /> mm</label>
      </div>
    `;
    container.appendChild(this.root);
    this.bindEvents();
  }

  private bindEvents(): void {
    const toggle = this.root.querySelector('#support-toggle') as HTMLInputElement;
    const configDiv = this.root.querySelector('#support-config') as HTMLDivElement;

    toggle.addEventListener('change', () => {
      this.enabled = toggle.checked;
      configDiv.hidden = !this.enabled;
      this.emit();
    });

    this.root.querySelectorAll('.support-config input').forEach(input => {
      input.addEventListener('change', () => this.emit());
    });
  }

  private emit(): void {
    this.config = this.readConfig();
    this.changeCb?.({ enabled: this.enabled, config: this.config });
  }

  private readConfig(): SupportConfig {
    const q = (id: string) => parseFloat((this.root.querySelector(id) as HTMLInputElement).value);
    return {
      layerHeight: q('#support-layer'),
      lightThreshold: q('#support-light'),
      mediumThreshold: q('#support-medium'),
      lightTipDiameter: q('#support-light-tip'),
      mediumTipDiameter: q('#support-medium-tip'),
      heavyTipDiameter: q('#support-heavy-tip'),
      lightPenetration: this.config.lightPenetration,
      mediumPenetration: this.config.mediumPenetration,
      heavyPenetration: this.config.heavyPenetration,
      lightSpacing: this.config.lightSpacing,
      mediumSpacing: this.config.mediumSpacing,
      heavySpacing: this.config.heavySpacing,
      raftThickness: this.config.raftThickness,
      raftLineWidth: this.config.raftLineWidth,
      cellSize: this.config.cellSize,
    };
  }

  onChange(cb: (state: SupportPanelState) => void): void {
    this.changeCb = cb;
  }

  getState(): SupportPanelState {
    return { enabled: this.enabled, config: this.config };
  }

  setEnabled(v: boolean): void {
    this.enabled = v;
    const toggle = this.root.querySelector('#support-toggle') as HTMLInputElement;
    const configDiv = this.root.querySelector('#support-config') as HTMLDivElement;
    toggle.checked = v;
    configDiv.hidden = !v;
  }
}
