import { AppState } from './app/AppState';
import { AppController } from './app/AppController';
import { Viewport } from './viewport';
import { FileDrop } from './views/FileDrop';
import { ConfigPanel } from './views/ConfigPanel';
import { ScorePanel } from './views/ScorePanel';
import { CandidateList } from './views/CandidateList';
import { defaultConfig } from './types';
import { DEFAULT_PROFILE, DEFAULT_RANKER } from './constants';
import './styles/theme.css';
import './styles/main.css';

function boot(): void {
  const state = new AppState({
    config: defaultConfig(),
    candidates: [],
    currentIndex: 0,
    stlName: '',
    currentProfile: DEFAULT_PROFILE,
    currentRanker: DEFAULT_RANKER,
    isComputeDirty: false,
    lastOriData: null,
    liveData: null,
    normBounds: null,
    loadConvention: 'z-up',
  });

  const viewportContainer = document.getElementById('viewport')!;
  const viewport = new Viewport(viewportContainer);

  const fileDrop = new FileDrop(
    document.getElementById('drop-zone')!,
    document.getElementById('file-input') as HTMLInputElement,
  );

  const configPanel = new ConfigPanel(
    document.getElementById('angle-slider') as HTMLInputElement,
    document.getElementById('angle-value')!,
    document.getElementById('hull-sphere-toggle') as HTMLInputElement,
    document.getElementById('convention-select') as HTMLSelectElement,
    document.getElementById('profile-select') as HTMLSelectElement,
    document.getElementById('ranker-select') as HTMLSelectElement,
    document.getElementById('find-btn') as HTMLButtonElement,
    document.getElementById('export-btn') as HTMLButtonElement,
    document.getElementById('recalc-btn') as HTMLButtonElement,
  );

  const scorePanel = new ScorePanel(
    document.getElementById('score-big')!,
    document.getElementById('sp-profile')!,
    document.getElementById('sp-ranker')!,
    document.getElementById('sp-rows')!,
    document.getElementById('sp-hint')!,
  );

  const candidateList = new CandidateList(
    document.getElementById('candidate-list') as HTMLOListElement,
    document.getElementById('candidates-section')!,
  );

  const controller = new AppController({
    state,
    viewport,
    fileDrop,
    configPanel,
    scorePanel,
    candidateList,
    workerFactory: () => new Worker(new URL('./orient.worker.ts', import.meta.url), { type: 'module' }),
    statusEl: document.getElementById('status')!,
    progressContainer: document.getElementById('progress-container')!,
    progressBar: document.getElementById('progress-bar')!,
    progressLabel: document.getElementById('progress-label')!,
    resultsPlaceholder: document.getElementById('results-placeholder')!,
    panelRight: document.getElementById('results')!,
    cancelBtn: document.getElementById('cancel-btn') as HTMLButtonElement,
  });

  controller.boot();
}

boot();
