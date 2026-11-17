import type { ReactNode } from 'react';

export function PageHead({ eyebrow, title, children }: { eyebrow?: string; title: string; children?: ReactNode }) {
  return (
    <div className="page-head">
      {eyebrow && <div className="eyebrow">{eyebrow}</div>}
      <h1>{title}</h1>
      {children && <p>{children}</p>}
    </div>
  );
}

export function Badge({ children, tone = '' }: { children: ReactNode; tone?: string }) {
  return <span className={`badge ${tone}`}>{children}</span>;
}

export function Chips({ items, accent = false }: { items: string[]; accent?: boolean }) {
  if (!items.length) return null;
  return <div className="chips">{items.map((x, i) => <span key={`${x}-${i}`} className={`chip ${accent ? 'accent' : ''}`}>{x}</span>)}</div>;
}

export function Empty({ icon = '◌', title, children }: { icon?: string; title: string; children?: ReactNode }) {
  return <div className="empty"><div className="empty-icon">{icon}</div><strong>{title}</strong>{children && <div style={{ marginTop: 6 }}>{children}</div>}</div>;
}

export function SectionDivider({ title }: { title: string }) {
  return <div className="section-divider"><h2>{title}</h2></div>;
}

export function StatCard({ num, label, tone = '' }: { num: number | string; label: string; tone?: string }) {
  return <div className={`stat-card ${tone}`}><div className="sc-num">{num}</div><div className="sc-label">{label}</div></div>;
}

export function relativeDate(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime();
  const days = Math.floor(diff / 86_400_000);
  if (days <= 0) return 'today';
  if (days === 1) return 'yesterday';
  if (days < 30) return `${days}d ago`;
  const months = Math.floor(days / 30);
  if (months < 12) return `${months}mo ago`;
  return `${Math.floor(months / 12)}y ago`;
}
