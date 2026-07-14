import type { Candidate } from '../types';
import styles from '../styles/CandidateList.module.css';

export class CandidateList {
  private onSelectCallback: ((index: number) => void) | null = null;
  private boundClick: (e: MouseEvent) => void;

  constructor(
    private listEl: HTMLOListElement,
    private sectionEl: HTMLElement,
  ) {
    this.boundClick = (e: MouseEvent) => {
      const li = (e.target as HTMLElement).closest('li');
      if (!li || !li.dataset.index) return;
      this.onSelectCallback?.(parseInt(li.dataset.index));
    };
    this.listEl.addEventListener('click', this.boundClick);
  }

  render(candidates: Candidate[], currentIndex: number): void {
    this.listEl.innerHTML = candidates
      .map(
        (c, i) =>
          `<li class="${i === currentIndex ? styles.active : ''}" data-index="${i}">#${i + 1} — ${(c.compositeScore * 100).toFixed(0)}%` +
          `<span class="${styles.infoIcon}" title="o: ${c.overhangPenalty.toFixed(0)} f: ${c.footprint.toFixed(0)} x: ${c.maxCross.toFixed(0)} s: ${(c.shadowed * 100).toFixed(0)}% q: ${c.surfaceQuality.toFixed(2)} · H: ${c.estHeight.toFixed(1)}mm · ${c.stability}">ⓘ</span></li>`,
      )
      .join('');
  }

  show(): void {
    this.sectionEl.style.display = 'block';
  }

  hide(): void {
    this.sectionEl.style.display = 'none';
  }

  onSelect(callback: (index: number) => void): void {
    this.onSelectCallback = callback;
  }
}
