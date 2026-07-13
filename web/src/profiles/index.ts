import type { ScoreWeights } from '../compute';

const modules = import.meta.glob('./*.json', { eager: true, import: 'default' }) as Record<string, ScoreWeights>;

export function loadProfiles(): Record<string, ScoreWeights> {
  const profiles: Record<string, ScoreWeights> = {};
  for (const [path, data] of Object.entries(modules)) {
    // Strip directory prefix and .json suffix to get the profile name.
    const name = path.replace(/^\.\/(.+)\.json$/, '$1');
    profiles[name] = data;
  }
  return profiles;
}