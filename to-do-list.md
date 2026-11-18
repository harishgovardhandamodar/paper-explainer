# Paper Explainer — improvement backlog

The 25 improvements proposed after reviewing the current dashboard. Each item is implemented and
committed on its own, in this order. Effort: S = hours, M = days, L = multi-day.

Status legend: `[ ]` planned · `[x]` done and committed.

## Present & communicate

- [x] **1. Presentation mode** (M) — Slideshow from a paper: question → hypotheses → method → results → caveats → implications. Arrow keys, fullscreen, per-slide speaker notes.
- [x] **2. Audience lens** (M) — Researcher / Engineer / Executive / Student switch that changes depth and vocabulary across views; deterministic text stays the baseline.
- [x] **3. Compare-papers view** (M) — Side-by-side 2–N comparison of question, method, datasets/metrics, results and caveats, plus graph focus.
- [x] **4. One-pager + multi-format export** (S–M) — Designed 1-page HTML/PDF brief, self-contained shareable readout, BibTeX, CSV/JSON findings, Marp/Slidev deck.
- [x] **5. Focus / spotlight mode** (S) — Dim everything but the selected claim + evidence; fullscreen a single Mermaid diagram; presenter timer.

## Read & understand

- [x] **6. Linear "Read this paper" narrative** (M) — One scrolling story with sticky evidence rail, replacing tab-hopping.
- [x] **7. Source-linked evidence panel** (M) — Every claim links to its exact section/paragraph in a highlighted source panel; page/line anchors where available.
- [ ] **8. Personal annotations & highlights** (M) — User notes anchored to entities/text ranges, tagged, timestamped, exported; distinct from extractor `notes`.
- [ ] **9. Claim–evidence matrix** (S–M) — Sortable hypotheses × experiments × findings grid with status/confidence and unsupported-claim highlighting.
- [ ] **10. "What this paper does NOT show" view** (S) — Author caveats vs methodological limits vs my critique; prevents overclaiming for nontechnical readers.
- [ ] **11. Reproducibility checklist** (S) — Turn `ExperimentSetup` into a scored card; flag missing code, seed, hyperparameters, compute; link artifacts.

## Follow up & act

- [ ] **12. Follow-up board** (M) — Tasks/questions attached to claims with status, priority and notes.
- [ ] **13. Auto-extracted open questions** (S–M) — Mine limitations/future-work into reviewable follow-up candidates.
- [ ] **14. Hypothesis tracker across papers** (M) — Aggregate hypothesis status across the corpus into a supported/contested/open timeline with counter-evidence.
- [ ] **15. Cross-paper synthesis builder** (L) — Select N papers → stored synthesis: related-work narrative, comparison table, combined diagram.
- [ ] **16. Reading progress & triage inbox** (S–M) — Unread/in-progress/done, last position, priority and "why read"; new Library facets.

## Knowledge base & scale

- [ ] **17. PDF / DOI ingest with version dedupe** (M–L) — PDF upload, DOI/arXiv lookup, recognize versioned IDs as one paper.
- [ ] **18. Citation graph** (L) — Parse references, add cites/cited_by edges distinct from concept bridges; optional Crossref enrichment.
- [ ] **19. Glossary & concept explorer** (S–M) — Plain-language definitions, aliases, prerequisites, "where used"; student-friendly.
- [ ] **20. Projects / literature-review workspaces** (M) — Named groups of papers with their own notes, follow-ups and exported readout.
- [ ] **21. Provenance & confidence on every item** (S–M) — Per-item curated/heuristic/AI/verified badges, extraction rule, confidence.

## Trust, correction & reach

- [ ] **22. Correction + verification loop** (M–L) — Inline edit of extracted fields, mark verified, feed corrections back; extraction-quality dashboard.
- [ ] **23. AI guardrails: source-anchored insights** (M) — Each AI insight cites a section; unverified flagged; one-click "verify against paper".
- [ ] **24. Version history & safe re-import diffs** (M) — Snapshot before refresh/enhance, field-level diff, restore; protect manual edits.
- [ ] **25. Accessible & reachable everywhere** (M) — `/` search, `j/k/o` navigation, `p` present, ARIA, table alternative to force graph, responsive mobile, offline PWA cache.

---

## Progress log

| # | Feature | Commit | Notes |
|---|---------|--------|-------|
| 0 | Dark/light theme baseline | `9dfb6c1` | Pending work from prior session committed first |
| 1 | Presentation mode | `d2bd6fb` | Generated slides from paper structure; arrow/Home/End/N/Esc keys, fullscreen stage, notes panel, progress dots; `MermaidDiagram` bare mode |
| 2 | Audience lens | `7c6fd7b` | Persisted Researcher/Engineer/Executive/Student switch in topbar; plain labels, collapsible raw evidence, student glossary; deterministic text unchanged |
| 3 | Compare-papers view | `1a72e29` | 2–N picker, aligned dimension matrix with differences-only filter, shared-bridge list and focused force graph |
| 4 | One-pager + multi-format export | `454d5be` | Self-contained print-ready HTML brief, BibTeX, findings CSV/JSON, Slidev/Marp deck; Rust formatters with 5 new unit tests; Export view with preview and share link |
| 5 | Focus / spotlight mode | `dab7f3e` | Reusable spotlight controller in hypotheses, findings, steps and diagrams; dim-unselected, fullscreen diagram, count-up/countdown presenter timer, Esc to exit |
| 6 | Linear read narrative | `294f175` | Six-section scrolling story with IntersectionObserver-driven section tracking, sticky evidence rail and section progress bar |
| 7 | Source-linked evidence panel | `2177dad` | Claim→section/paragraph matching with phrase highlighting, page/figure/line anchors, filter and prev/next navigation |
