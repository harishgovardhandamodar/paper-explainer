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
- [x] **8. Personal annotations & highlights** (M) — User notes anchored to entities/text ranges, tagged, timestamped, exported; distinct from extractor `notes`.
- [x] **9. Claim–evidence matrix** (S–M) — Sortable hypotheses × experiments × findings grid with status/confidence and unsupported-claim highlighting.
- [x] **10. "What this paper does NOT show" view** (S) — Author caveats vs methodological limits vs my critique; prevents overclaiming for nontechnical readers.
- [x] **11. Reproducibility checklist** (S) — Turn `ExperimentSetup` into a scored card; flag missing code, seed, hyperparameters, compute; link artifacts.

## Follow up & act

- [x] **12. Follow-up board** (M) — Tasks/questions attached to claims with status, priority and notes.
- [x] **13. Auto-extracted open questions** (S–M) — Mine limitations/future-work into reviewable follow-up candidates.
- [x] **14. Hypothesis tracker across papers** (M) — Aggregate hypothesis status across the corpus into a supported/contested/open timeline with counter-evidence.
- [x] **15. Cross-paper synthesis builder** (L) — Select N papers → stored synthesis: related-work narrative, comparison table, combined diagram.
- [x] **16. Reading progress & triage inbox** (S–M) — Unread/in-progress/done, last position, priority and "why read"; new Library facets.

## Knowledge base & scale

- [x] **17. PDF / DOI ingest with version dedupe** (M–L) — PDF upload, DOI/arXiv lookup, recognize versioned IDs as one paper.
- [x] **18. Citation graph** (L) — Parse references, add cites/cited_by edges distinct from concept bridges; optional Crossref enrichment.
- [x] **19. Glossary & concept explorer** (S–M) — Plain-language definitions, aliases, prerequisites, "where used"; student-friendly.
- [x] **20. Projects / literature-review workspaces** (M) — Named groups of papers with their own notes, follow-ups and exported readout.
- [x] **21. Provenance & confidence on every item** (S–M) — Per-item curated/heuristic/AI/verified badges, extraction rule, confidence.

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
| 8 | Personal annotations | `3f1cc5b` | New persisted `Annotation` model, CRUD API and workspace with entity/quote/section anchors, tags, colors, timestamps, JSON/Markdown export |
| 9 | Claim–evidence matrix | `14082f1` | Sortable hypotheses × experiments grid with linked findings in cells, status/confidence sorting, gaps-only filter and unsupported-claim highlighting |
| 10 | What it does NOT show | `1c04035` | Three-column view separating stated author caveats, deterministic setup flags (baseline/dataset/budget/reproducibility/evidence links) and personal critique persisted as annotations |
| 11 | Reproducibility checklist | `10f34cb` | Ten-check scored card per experiment (dataset, baseline, metrics, conditions, compute, seed, hyperparameters, code, steps, linked results), overall signal bar and detected artifact links |
| 12 | Follow-up board | `e52a2f2` | New persisted `FollowUp` model and CRUD API; three-column open/in-progress/done board with priority, anchors, notes, tags, claim-derived suggestions and JSON/Markdown export |
| 13 | Open questions | `4b361e5` | New deterministic `questions` extractor + `OpenQuestion` model and `GET /api/papers/{id}/open-questions`; mines limitation/future-work/discussion/conclusion sections, caveats, unresolved claims and notes with cue/confidence/source; review UI with editable suggestions, filters, selection and promote-to-followup |
| 14 | Hypothesis tracker | `5a39b8a` | New `tracker` module + `GET /api/hypothesis-tracker`; token-overlap clustering across papers into supported/contested/rejected/open themes with status-minibars, cross-paper tension, finding-level counter-evidence and a flat chronological timeline; global view with paper deep-links |
| 15 | Synthesis builder | `d3e1447` | New persisted `Synthesis` model + store CRUD/refresh and 6 endpoints; deterministic `synthesis` generator produces related-work narrative, 11-row comparison matrix and shared-theme Mermaid diagram; builder UI with paper picker, preview, editable notes, saved list, regenerate and Markdown export |
| 16 | Reading progress & triage | `454e241` | New persisted `ReadingState` (status/priority/why/last view/anchor), 3 endpoints and library `reading_status`/`priority` filters + facets; global triage inbox with stats, filters, inline editing and resume-from-view; App records last view on paper navigation (auto-tracks first view as in-progress) |
| 17 | PDF / DOI ingest + dedupe | `845db0b` | New `identifiers` module canonicalises arXiv ids (incl. old-style + DOIs) to version-free ids; `pdf` module extracts text via streamed `pdftotext` (poppler-utils added to image); ingest accepts `doi`/`pdf`/`pdf_filename`, preserves `added_at` on re-import, sets `arxiv_id`/`version` and new `pdf` source; `GET /api/resolve` preview; 4-mode dialog with duplicate warning |
| 18 | Citation graph | `f8fff20` | New `citations` module parses bibliography entries (arXiv id/DOI/year/title heuristics) from HTML + PDF text; deterministic re-resolution against corpus at every graph rebuild; `cites` edges to paper/reference nodes (distinct from concept bridges), `CitedBy` inverse view; Crossref enrichment endpoints (preview + persist); `CitationsView` with Mermaid network, stats, sortable table and per-reference enrich |
| 19 | Glossary & concept explorer | `24e34ba` | New `glossary` module aggregates per-paper `Concept`s collection-wide (merges duplicates, prefers longest definition, derives aliases for acronyms and prerequisites from definition mentions with teaching-order ranking); deterministic plain-language rewrite (fixed jargon swaps, strips asides); `GET /api/glossary` + detail lookup by name/id/alias; `GlossaryView` with search, category facets, A–Z/usage sorts, plain/source toggle, prerequisite jumping and "where used" paper links |
| 20 | Literature-review projects | `f860fd9` | New persisted `Project` + `ProjectFollowUp` in main store with full CRUD endpoints (papers, tags, notes, scoped tasks); `projects` module renders a deterministic markdown readout (paper table, hypotheses, findings + evidence + caveats, shared themes, within-project citations, notes and task checklist); `ProjectsView` with project list, create/edit forms, paper picker, notes autosave-style editing, task board and readout preview/download/copy |
| 21 | Provenance & confidence | `0778eee` | New `provenance` module assigns every item (summary, hypotheses, experiments, findings, diagrams, workflows, concepts, each AI insight) an origin badge (curated/heuristic/AI/manual/verified), a named extraction rule and a strength-based confidence (clearly not a truth probability); startup + ingest backfill, AI lifecycle hooks, overrides carried across re-import; `GET /api/provenance` (+ paper filter), item PUT for badge/confidence/note/verify, backfill endpoint; `ProvenanceView` with stats, filters, sortable table and inline editing |
