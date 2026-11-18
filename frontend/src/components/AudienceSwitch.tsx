import { applyAudience, AUDIENCES, useAudience } from '../audience';

/** Compact top-bar control for choosing how deep and how plain the explanation is. */
export function AudienceSwitch() {
  const audience = useAudience();
  return (
    <div className="lens-switch" role="group" aria-label="View as audience">
      <span className="lens-caption" title="Adapt depth and vocabulary to the reader">View as</span>
      {AUDIENCES.map((a) => (
        <button
          key={a.key}
          className={`lens-btn ${audience === a.key ? 'active' : ''}`}
          onClick={() => applyAudience(a.key)}
          title={a.tagline}
          aria-pressed={audience === a.key}
        >
          <span className="lens-glyph" aria-hidden="true">{a.icon}</span>
          <span className="lens-label">{a.label}</span>
        </button>
      ))}
    </div>
  );
}
