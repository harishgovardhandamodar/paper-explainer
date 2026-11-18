//! File-backed persistence with atomic writes.
//!
//! The store is intentionally swappable: `Store` is a plain serde value, so a
//! future SQLite/graph-database backend only needs to reimplement these methods.

use crate::categories;
use crate::error::{AppError, AppResult};
use crate::graph;
use crate::models::*;
use crate::seed::{seed_paper, SEED_ID};
use crate::util::make_id;
use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

pub type SharedStore = Arc<RwLock<Store>>;

#[derive(Debug, Clone)]
pub struct StoreHandle {
    pub path: PathBuf,
    pub inner: SharedStore,
}

impl StoreHandle {
    pub fn open() -> AppResult<Self> {
        let path = std::env::var("PAPER_EXPLAINER_DATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("data/store.json"));
        Self::open_at(path)
    }

    pub fn open_at(path: PathBuf) -> AppResult<Self> {
        let mut store = if path.exists() {
            let raw = fs::read_to_string(&path)?;
            if raw.trim().is_empty() {
                seeded_store()
            } else {
                let s: Store = serde_json::from_str(&raw).map_err(|e| {
                    AppError::Internal(format!("corrupt store at {}: {}", path.display(), e))
                })?;
                if s.papers.is_empty() { seeded_store() } else { s }
            }
        } else {
            seeded_store()
        };

        // Migration: papers saved before categorisation existed get categories
        // derived from their own text, then the change is persisted below.
        let _backfilled = categories::backfill(&mut store.papers);
        // Likewise, older papers gain per-item provenance on first open so
        // every badge is present without any user action.
        let _provenance_backfilled = crate::provenance::backfill(&mut store.papers);
        // Older AI explanations gain section anchors + verification state on
        // first open, so every insight is checkable without regenerating it.
        for paper in &mut store.papers {
            crate::guardrails::ensure_anchors(paper, None);
        }

        let handle = Self { path, inner: Arc::new(RwLock::new(store)) };
        handle.rebuild_graph()?;
        handle.flush()?;
        Ok(handle)
    }

    pub fn read(&self) -> AppResult<std::sync::RwLockReadGuard<'_, Store>> {
        self.inner.read().map_err(|e| AppError::Internal(format!("store lock poisoned: {e}")))
    }

    pub fn write(&self) -> AppResult<std::sync::RwLockWriteGuard<'_, Store>> {
        self.inner.write().map_err(|e| AppError::Internal(format!("store lock poisoned: {e}")))
    }

    pub fn list_papers(&self) -> AppResult<Vec<PaperSummary>> {
        let store = self.read()?;
        let mut papers: Vec<_> = store.papers.iter().map(|p| p.to_summary()).collect();
        papers.sort_by(|a, b| b.added_at.cmp(&a.added_at));
        Ok(papers)
    }

    /// Filter, sort and paginate the library, returning facet counts computed
    /// over the whole collection so they stay stable while filters are active.
    pub fn query_library(&self, params: &LibraryQuery) -> AppResult<LibraryResponse> {
        use std::collections::HashMap;
        let store = self.read()?;
        let needle = params.q.trim().to_lowercase();
        let tokens: Vec<&str> = needle.split_whitespace().collect();
        let category = params.category.trim();
        let tag = params.tag.trim().to_lowercase();
        let source = params.source.trim().to_lowercase();
        let status = params.status.trim().to_lowercase();
        let reading_status = params.reading_status.trim().to_lowercase();
        let priority = params.priority.trim().to_lowercase();
        let reading_by_id: HashMap<&str, &ReadingState> = store.reading_states.iter()
            .map(|r| (r.paper_id.as_str(), r)).collect();
        let state_for = |id: &str| -> ReadingState {
            reading_by_id.get(id).map(|r| (*r).clone()).unwrap_or_else(|| ReadingState::for_paper(id))
        };

        let mut matched: Vec<&Paper> = store.papers.iter().filter(|p| {
            if !needle.is_empty() && !p.all_text().to_lowercase().contains(&needle) {
                return false;
            }
            if !category.is_empty() && !p.categories.iter().any(|c| c == category) {
                return false;
            }
            if !tag.is_empty() && !p.tags.iter().any(|t| t.to_lowercase() == tag) {
                return false;
            }
            if !source.is_empty() && source_name(&p.source) != source {
                return false;
            }
            if !status.is_empty() && status_name(&p.status) != status {
                return false;
            }
            if !reading_status.is_empty() {
                let s = state_for(&p.id);
                let name = match s.status {
                    ReadingStatus::Unread => "unread",
                    ReadingStatus::InProgress => "in_progress",
                    ReadingStatus::Done => "done",
                };
                if name != reading_status { return false; }
            }
            if !priority.is_empty() {
                let s = state_for(&p.id);
                let name = match s.priority {
                    ReadingPriority::High => "high",
                    ReadingPriority::Medium => "medium",
                    ReadingPriority::Low => "low",
                };
                if name != priority { return false; }
            }
            if let Some(from) = params.year_from {
                if p.year.map_or(true, |y| y < from) { return false; }
            }
            if let Some(to) = params.year_to {
                if p.year.map_or(true, |y| y > to) { return false; }
            }
            true
        }).collect();

        let relevance = |p: &Paper| -> usize {
            if tokens.is_empty() { return 0; }
            let text = p.all_text().to_lowercase();
            tokens.iter().filter(|t| text.contains(**t)).count()
        };

        match params.sort.as_str() {
            "title" => matched.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase())),
            "year" => matched.sort_by(|a, b| b.year.cmp(&a.year)),
            "relevance" => matched.sort_by(|a, b| {
                relevance(b).cmp(&relevance(a)).then_with(|| b.added_at.cmp(&a.added_at))
            }),
            "findings" => matched.sort_by(|a, b| {
                b.findings.len().cmp(&a.findings.len()).then_with(|| b.added_at.cmp(&a.added_at))
            }),
            _ => matched.sort_by(|a, b| b.added_at.cmp(&a.added_at)), // "recent"
        }

        let total = matched.len();
        let limit = params.limit.clamp(1, 200);
        let offset = params.offset.min(total);
        let page: Vec<&Paper> = matched.into_iter().skip(offset).take(limit).collect();
        let reading: Vec<ReadingState> = page.iter().map(|p| state_for(&p.id)).collect();
        let papers: Vec<PaperSummary> = page.into_iter().map(|p| p.to_summary()).collect();

        Ok(LibraryResponse {
            papers,
            total,
            collection_total: store.papers.len(),
            limit,
            offset,
            facets: build_facets(&store.papers, &store.reading_states),
            categories: categories::category_defs(),
            reading,
        })
    }

    pub fn get_paper(&self, id: &str) -> AppResult<Paper> {
        self.read()?.find_paper(id).cloned().ok_or_else(|| AppError::NotFound(format!("paper '{id}'")))
    }

    pub fn upsert_paper(&self, mut paper: Paper) -> AppResult<Paper> {
        // A re-import replaces extracted fields, but human corrections and
        // provenance edits (verification, badges, notes, confidence) survive:
        // corrections are carried over and re-applied to the new extraction.
        let (carried_prov, carried_corrections) = {
            let guard = self.read()?;
            match guard.find_paper(&paper.id) {
                Some(old) => (
                    old.item_provenance.iter()
                        .filter(|(_, p)| p.overridden)
                        .map(|(k, p)| (k.clone(), p.clone()))
                        .collect::<Vec<_>>(),
                    old.corrections.clone(),
                ),
                None => (Vec::new(), Vec::new()),
            }
        };
        for (key, prov) in carried_prov {
            paper.item_provenance.entry(key).or_insert(prov);
        }
        if paper.corrections.is_empty() {
            paper.corrections = carried_corrections;
        }
        crate::corrections::apply_all(&mut paper);
        crate::provenance::backfill_paper(&mut paper);
        {
            let mut store = self.write()?;
            match store.papers.iter().position(|p| p.id == paper.id) {
                Some(i) => store.papers[i] = paper.clone(),
                None => store.papers.push(paper.clone()),
            }
        }
        self.rebuild_graph()?;
        self.flush()?;
        Ok(paper)
    }

    pub fn delete_paper(&self, id: &str) -> AppResult<()> {
        {
            let mut store = self.write()?;
            let before = store.papers.len();
            store.papers.retain(|p| p.id != id);
            if store.papers.len() == before {
                return Err(AppError::NotFound(format!("paper '{id}'")));
            }
        }
        self.rebuild_graph()?;
        self.flush()?;
        Ok(())
    }

    pub fn graph(&self) -> AppResult<KnowledgeGraph> {
        Ok(self.read()?.graph.clone())
    }

    pub fn add_edge(&self, edge: GraphEdge) -> AppResult<GraphEdge> {
        {
            let mut store = self.write()?;
            if !store.graph.nodes.iter().any(|n| n.id == edge.source) {
                return Err(AppError::BadRequest(format!("unknown source node '{}'", edge.source)));
            }
            if !store.graph.nodes.iter().any(|n| n.id == edge.target) {
                return Err(AppError::BadRequest(format!("unknown target node '{}'", edge.target)));
            }
            if store.custom_edges.iter().any(|e| e.id == edge.id) {
                return Err(AppError::BadRequest("edge already exists".into()));
            }
            store.custom_edges.push(edge.clone());
        }
        self.rebuild_graph()?;
        self.flush()?;
        Ok(edge)
    }

    pub fn annotations(&self, paper_id: Option<&str>) -> AppResult<Vec<Annotation>> {
        let store = self.read()?;
        Ok(store.annotations.iter()
            .filter(|a| paper_id.map_or(true, |id| a.paper_id == id))
            .cloned().collect())
    }

    pub fn add_annotation(&self, req: CreateAnnotation) -> AppResult<Annotation> {
        if req.body.trim().is_empty() {
            return Err(AppError::BadRequest("annotation body cannot be empty".into()));
        }
        {
            let store = self.read()?;
            if store.find_paper(&req.paper_id).is_none() {
                return Err(AppError::NotFound(format!("paper '{}'", req.paper_id)));
            }
        }
        let now = Utc::now();
        let id = make_id("ann", &format!("{}-{}-{}", req.paper_id, req.target, now.timestamp_millis()));
        let annotation = Annotation {
            id,
            paper_id: req.paper_id,
            target_kind: req.target_kind,
            target: req.target,
            quote: req.quote,
            body: req.body.trim().to_string(),
            tags: req.tags,
            color: if req.color.is_empty() { "amber".into() } else { req.color },
            created_at: now,
            updated_at: now,
        };
        { self.write()?.annotations.push(annotation.clone()); }
        self.flush()?;
        Ok(annotation)
    }

    pub fn update_annotation(&self, id: &str, req: UpdateAnnotation) -> AppResult<Annotation> {
        let updated = {
            let mut store = self.write()?;
            let a = store.annotations.iter_mut().find(|a| a.id == id)
                .ok_or_else(|| AppError::NotFound(format!("annotation '{id}'")))?;
            if let Some(body) = req.body {
                if body.trim().is_empty() {
                    return Err(AppError::BadRequest("annotation body cannot be empty".into()));
                }
                a.body = body.trim().to_string();
            }
            if let Some(tags) = req.tags { a.tags = tags; }
            if let Some(color) = req.color { a.color = color; }
            a.updated_at = Utc::now();
            a.clone()
        };
        self.flush()?;
        Ok(updated)
    }

    pub fn delete_annotation(&self, id: &str) -> AppResult<()> {
        {
            let mut store = self.write()?;
            let before = store.annotations.len();
            store.annotations.retain(|a| a.id != id);
            if store.annotations.len() == before {
                return Err(AppError::NotFound(format!("annotation '{id}'")));
            }
        }
        self.flush()
    }

    pub fn follow_ups(&self, paper_id: Option<&str>) -> AppResult<Vec<FollowUp>> {
        let store = self.read()?;
        Ok(store.follow_ups.iter()
            .filter(|f| paper_id.map_or(true, |id| f.paper_id == id))
            .cloned().collect())
    }

    pub fn add_follow_up(&self, req: CreateFollowUp) -> AppResult<FollowUp> {
        if req.title.trim().is_empty() {
            return Err(AppError::BadRequest("follow-up title cannot be empty".into()));
        }
        {
            let store = self.read()?;
            if store.find_paper(&req.paper_id).is_none() {
                return Err(AppError::NotFound(format!("paper '{}'", req.paper_id)));
            }
        }
        let now = Utc::now();
        let id = make_id("fu", &format!("{}-{}-{}", req.paper_id, req.title, now.timestamp_millis()));
        let item = FollowUp {
            id,
            paper_id: req.paper_id,
            target: req.target,
            title: req.title.trim().to_string(),
            notes: req.notes.trim().to_string(),
            status: req.status,
            priority: req.priority,
            tags: req.tags,
            created_at: now,
            updated_at: now,
        };
        { self.write()?.follow_ups.push(item.clone()); }
        self.flush()?;
        Ok(item)
    }

    pub fn update_follow_up(&self, id: &str, req: UpdateFollowUp) -> AppResult<FollowUp> {
        let updated = {
            let mut store = self.write()?;
            let f = store.follow_ups.iter_mut().find(|f| f.id == id)
                .ok_or_else(|| AppError::NotFound(format!("follow-up '{id}'")))?;
            if let Some(title) = req.title {
                if title.trim().is_empty() {
                    return Err(AppError::BadRequest("follow-up title cannot be empty".into()));
                }
                f.title = title.trim().to_string();
            }
            if let Some(notes) = req.notes { f.notes = notes.trim().to_string(); }
            if let Some(status) = req.status { f.status = status; }
            if let Some(priority) = req.priority { f.priority = priority; }
            if let Some(tags) = req.tags { f.tags = tags; }
            f.updated_at = Utc::now();
            f.clone()
        };
        self.flush()?;
        Ok(updated)
    }

    pub fn delete_follow_up(&self, id: &str) -> AppResult<()> {
        {
            let mut store = self.write()?;
            let before = store.follow_ups.len();
            store.follow_ups.retain(|f| f.id != id);
            if store.follow_ups.len() == before {
                return Err(AppError::NotFound(format!("follow-up '{id}'")));
            }
        }
        self.flush()
    }

    // -- Reading progress & triage -------------------------------------------

    /// All tracked reading states, most recently touched first.
    pub fn reading_states(&self) -> AppResult<Vec<ReadingState>> {
        let store = self.read()?;
        let mut list = store.reading_states.clone();
        list.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(list)
    }

    /// Return the tracked state for a paper, or an untracked default. Missing
    /// papers are an error so the UI cannot silently track a deleted paper.
    pub fn get_reading_state(&self, paper_id: &str) -> AppResult<ReadingState> {
        let store = self.read()?;
        if store.find_paper(paper_id).is_none() {
            return Err(AppError::NotFound(format!("paper '{paper_id}'")));
        }
        Ok(store.reading_states.iter().find(|r| r.paper_id == paper_id).cloned()
            .unwrap_or_else(|| ReadingState::for_paper(paper_id)))
    }

    /// Create or update the reading state for a paper. A first write that only
    /// records a view (no explicit status) defaults the paper to `in_progress`,
    /// so simply opening a paper makes it show up in the triage inbox.
    pub fn upsert_reading_state(&self, paper_id: &str, req: UpdateReadingState) -> AppResult<ReadingState> {
        let updated = {
            let mut store = self.write()?;
            if store.find_paper(paper_id).is_none() {
                return Err(AppError::NotFound(format!("paper '{paper_id}'")));
            }
            let pos = store.reading_states.iter().position(|r| r.paper_id == paper_id);
            let existing = pos.map(|i| store.reading_states[i].clone());
            let mut state = existing.clone().unwrap_or_else(|| ReadingState::for_paper(paper_id));
            let has_last_view = req.last_view.is_some();
            let has_status = req.status.is_some();

            if let Some(status) = req.status { state.status = status; }
            if let Some(priority) = req.priority { state.priority = priority; }
            if let Some(why) = req.why { state.why = why; }
            if let Some(view) = req.last_view { state.last_view = view; }
            if let Some(anchor) = req.last_anchor { state.last_anchor = anchor; }

            // Auto-track on first view: opening a paper shouldn't require an
            // explicit triage action to move it out of the unread bucket.
            let first_view_write = existing.is_none() && has_last_view && !has_status;
            if first_view_write { state.status = ReadingStatus::InProgress; }

            state.updated_at = Utc::now();
            match pos {
                Some(i) => store.reading_states[i] = state.clone(),
                None => store.reading_states.push(state.clone()),
            }
            state
        };
        self.flush()?;
        Ok(updated)
    }

    // -- Cross-paper syntheses ------------------------------------------------

    pub fn syntheses(&self) -> AppResult<Vec<Synthesis>> {
        let store = self.read()?;
        let mut list = store.syntheses.clone();
        list.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(list)
    }

    pub fn get_synthesis(&self, id: &str) -> AppResult<Synthesis> {
        self.read()?.syntheses.iter().find(|s| s.id == id).cloned()
            .ok_or_else(|| AppError::NotFound(format!("synthesis '{id}'")))
    }

    fn resolve_synthesis_papers(&self, req: &CreateSynthesis) -> AppResult<Vec<Paper>> {
        if req.paper_ids.is_empty() {
            return Err(AppError::BadRequest("select at least one paper".into()));
        }
        if req.paper_ids.len() > crate::synthesis::MAX_PAPERS {
            return Err(AppError::BadRequest(format!("a synthesis can combine at most {} papers", crate::synthesis::MAX_PAPERS)));
        }
        let mut unique = Vec::new();
        for id in &req.paper_ids {
            if !unique.contains(id) { unique.push(id.clone()); }
        }
        let store = self.read()?;
        unique.iter().map(|id| store.find_paper(id)
            .cloned().ok_or_else(|| AppError::NotFound(format!("paper '{id}'"))))
            .collect::<AppResult<Vec<_>>>()
    }

    /// Generate without persisting — used by the UI preview step.
    pub fn preview_synthesis(&self, req: CreateSynthesis) -> AppResult<Synthesis> {
        let papers = self.resolve_synthesis_papers(&req)?;
        Ok(crate::synthesis::build(&papers, req.title, req.notes))
    }

    pub fn add_synthesis(&self, req: CreateSynthesis) -> AppResult<Synthesis> {
        let papers = self.resolve_synthesis_papers(&req)?;
        let syn = crate::synthesis::build(&papers, req.title, req.notes);
        { self.write()?.syntheses.push(syn.clone()); }
        self.flush()?;
        Ok(syn)
    }

    pub fn update_synthesis(&self, id: &str, req: UpdateSynthesis) -> AppResult<Synthesis> {
        let updated = {
            let mut store = self.write()?;
            let syn = store.syntheses.iter_mut().find(|s| s.id == id)
                .ok_or_else(|| AppError::NotFound(format!("synthesis '{id}'")))?;
            if let Some(title) = req.title {
                let title = title.trim().to_string();
                if title.is_empty() { return Err(AppError::BadRequest("title cannot be empty".into())); }
                syn.title = title;
            }
            if let Some(notes) = req.notes { syn.notes = notes; }
            syn.updated_at = Utc::now();
            syn.clone()
        };
        self.flush()?;
        Ok(updated)
    }

    pub fn refresh_synthesis(&self, id: &str) -> AppResult<Synthesis> {
        let next = {
            let store = self.read()?;
            let syn = store.syntheses.iter().find(|s| s.id == id).cloned()
                .ok_or_else(|| AppError::NotFound(format!("synthesis '{id}'")))?;
            let papers = syn.paper_ids.iter().map(|pid| store.find_paper(pid).cloned())
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| AppError::BadRequest("a source paper no longer exists".into()))?;
            crate::synthesis::refresh(&syn, &papers)
        };
        {
            let mut store = self.write()?;
            if let Some(syn) = store.syntheses.iter_mut().find(|s| s.id == id) { *syn = next.clone(); }
        }
        self.flush()?;
        Ok(next)
    }

    pub fn delete_synthesis(&self, id: &str) -> AppResult<()> {
        {
            let mut store = self.write()?;
            let before = store.syntheses.len();
            store.syntheses.retain(|s| s.id != id);
            if store.syntheses.len() == before {
                return Err(AppError::NotFound(format!("synthesis '{id}'")));
            }
        }
        self.flush()
    }

    // -- Literature-review projects ------------------------------------------

    pub fn projects(&self) -> AppResult<Vec<Project>> {
        let store = self.read()?;
        let mut list = store.projects.clone();
        list.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(list)
    }

    pub fn get_project(&self, id: &str) -> AppResult<Project> {
        self.read()?.projects.iter().find(|p| p.id == id).cloned()
            .ok_or_else(|| AppError::NotFound(format!("project '{id}'")))
    }

    fn resolve_project_papers(&self, ids: &[String]) -> AppResult<Vec<String>> {
        let store = self.read()?;
        let mut unique = Vec::new();
        for id in ids {
            if !unique.contains(id) { unique.push(id.clone()); }
        }
        for id in &unique {
            if store.find_paper(id).is_none() {
                return Err(AppError::NotFound(format!("paper '{id}'")));
            }
        }
        Ok(unique)
    }

    pub fn create_project(&self, req: CreateProject) -> AppResult<Project> {
        let name = req.name.trim().to_string();
        if name.is_empty() { return Err(AppError::BadRequest("project name cannot be empty".into())); }
        let paper_ids = self.resolve_project_papers(&req.paper_ids)?;
        let now = Utc::now();
        let project = Project {
            id: make_id("project", &name),
            name,
            description: req.description.unwrap_or_default().trim().to_string(),
            paper_ids,
            notes: req.notes.unwrap_or_default(),
            tags: req.tags,
            follow_ups: Vec::new(),
            created_at: now,
            updated_at: now,
        };
        {
            let mut store = self.write()?;
            if store.projects.iter().any(|p| p.id == project.id) {
                return Err(AppError::BadRequest(format!("a project named '{}' already exists", project.name)));
            }
            store.projects.push(project.clone());
        }
        self.flush()?;
        Ok(project)
    }

    pub fn update_project(&self, id: &str, req: UpdateProject) -> AppResult<Project> {
        if let Some(name) = &req.name {
            if name.trim().is_empty() { return Err(AppError::BadRequest("project name cannot be empty".into())); }
        }
        let paper_ids = match &req.paper_ids {
            Some(ids) => Some(self.resolve_project_papers(ids)?),
            None => None,
        };
        let updated = {
            let mut store = self.write()?;
            let project = store.projects.iter_mut().find(|p| p.id == id)
                .ok_or_else(|| AppError::NotFound(format!("project '{id}'")))?;
            if let Some(name) = req.name { project.name = name.trim().to_string(); }
            if let Some(description) = req.description { project.description = description; }
            if let Some(ids) = paper_ids { project.paper_ids = ids; }
            if let Some(notes) = req.notes { project.notes = notes; }
            if let Some(tags) = req.tags { project.tags = tags; }
            project.updated_at = Utc::now();
            project.clone()
        };
        self.flush()?;
        Ok(updated)
    }

    pub fn delete_project(&self, id: &str) -> AppResult<()> {
        {
            let mut store = self.write()?;
            let before = store.projects.len();
            store.projects.retain(|p| p.id != id);
            if store.projects.len() == before {
                return Err(AppError::NotFound(format!("project '{id}'")));
            }
        }
        self.flush()
    }

    fn mutate_project_follow_ups(
        &self, project_id: &str,
        f: impl FnOnce(&mut Vec<ProjectFollowUp>) -> AppResult<ProjectFollowUp>,
    ) -> AppResult<ProjectFollowUp> {
        let result = {
            let mut store = self.write()?;
            let project = store.projects.iter_mut().find(|p| p.id == project_id)
                .ok_or_else(|| AppError::NotFound(format!("project '{project_id}'")))?;
            let result = f(&mut project.follow_ups)?;
            project.updated_at = Utc::now();
            result
        };
        self.flush()?;
        Ok(result)
    }

    pub fn add_project_follow_up(
        &self, project_id: &str, req: CreateProjectFollowUp,
    ) -> AppResult<ProjectFollowUp> {
        let title = req.title.trim().to_string();
        if title.is_empty() { return Err(AppError::BadRequest("follow-up title cannot be empty".into())); }
        self.mutate_project_follow_ups(project_id, move |list| {
            let task = ProjectFollowUp {
                id: make_id("task", &format!("{project_id}:{title}:{}", list.len())),
                title,
                notes: req.notes.unwrap_or_default(),
                done: false,
                paper_id: req.paper_id,
                created_at: Utc::now(),
            };
            list.push(task.clone());
            Ok(task)
        })
    }

    pub fn update_project_follow_up(
        &self, project_id: &str, task_id: &str, req: UpdateProjectFollowUp,
    ) -> AppResult<ProjectFollowUp> {
        if let Some(title) = &req.title {
            if title.trim().is_empty() { return Err(AppError::BadRequest("follow-up title cannot be empty".into())); }
        }
        self.mutate_project_follow_ups(project_id, move |list| {
            let task = list.iter_mut().find(|t| t.id == task_id)
                .ok_or_else(|| AppError::NotFound(format!("project follow-up '{task_id}'")))?;
            if let Some(title) = req.title { task.title = title.trim().to_string(); }
            if let Some(notes) = req.notes { task.notes = notes; }
            if let Some(done) = req.done { task.done = done; }
            if let Some(paper_id) = req.paper_id { task.paper_id = Some(paper_id); }
            Ok(task.clone())
        })
    }

    pub fn delete_project_follow_up(&self, project_id: &str, task_id: &str) -> AppResult<()> {
        self.mutate_project_follow_ups(project_id, |list| {
            let before = list.len();
            list.retain(|t| t.id != task_id);
            if list.len() == before {
                return Err(AppError::NotFound(format!("project follow-up '{task_id}'")));
            }
            // No single task to return on delete; a sentinel is enough because
            // the wrapper only needs to timestamp and flush the project.
            Ok(ProjectFollowUp::default())
        })?;
        Ok(())
    }

    pub fn project_readout(&self, id: &str) -> AppResult<ProjectReadout> {
        let store = self.read()?;
        let project = store.projects.iter().find(|p| p.id == id)
            .ok_or_else(|| AppError::NotFound(format!("project '{id}'")))?;
        let papers = project.paper_ids.iter()
            .filter_map(|pid| store.find_paper(pid))
            .cloned().collect::<Vec<_>>();
        Ok(ProjectReadout {
            project_id: id.into(),
            title: project.name.clone(),
            markdown: crate::projects::readout(project, &papers),
        })
    }

    // -- Provenance & confidence ---------------------------------------------

    pub fn provenance_report(&self, paper_id: Option<&str>) -> AppResult<crate::provenance::ProvenanceReport> {
        let store = self.read()?;
        let papers: Vec<Paper> = store.papers.iter()
            .filter(|p| paper_id.map_or(true, |id| p.id == id))
            .cloned().collect();
        Ok(crate::provenance::report(&papers))
    }

    pub fn update_provenance(
        &self, paper_id: &str, key: &str, req: UpdateProvenance,
    ) -> AppResult<Provenance> {
        let updated = {
            let mut store = self.write()?;
            let paper = store.find_paper_mut(paper_id)
                .ok_or_else(|| AppError::NotFound(format!("paper '{paper_id}'")))?;
            let prov = paper.item_provenance.get_mut(key)
                .ok_or_else(|| AppError::NotFound(format!("provenance for item '{key}'")))?;
            if let Some(verified) = req.verified {
                prov.verified_at = if verified { Some(Utc::now()) } else { None };
                prov.kind = if verified { ProvenanceKind::Verified } else { prov.origin };
            }
            if let Some(kind) = req.kind { prov.kind = kind; }
            if let Some(rule) = req.rule { prov.rule = rule; }
            if let Some(confidence) = req.confidence { prov.confidence = confidence.clamp(0.0, 1.0); }
            if let Some(section) = req.section {
                if !section.trim().is_empty() { prov.section = Some(section); }
            }
            if let Some(note) = req.note { prov.note = note; }
            prov.overridden = true;
            prov.updated_at = Utc::now();
            prov.clone()
        };
        self.flush()?;
        Ok(updated)
    }

    // -- Corrections & extraction quality ------------------------------------

    pub fn editable_items(&self, paper_id: &str) -> AppResult<Vec<crate::corrections::EditableItem>> {
        let store = self.read()?;
        let paper = store.find_paper(paper_id)
            .ok_or_else(|| AppError::NotFound(format!("paper '{paper_id}'")))?;
        Ok(crate::corrections::editable_items(paper))
    }

    pub fn corrections(&self, paper_id: Option<&str>) -> AppResult<Vec<crate::corrections::CorrectionView>> {
        let store = self.read()?;
        let mut out = Vec::new();
        for paper in store.papers.iter().filter(|p| paper_id.map_or(true, |id| p.id == id)) {
            for c in &paper.corrections {
                out.push(crate::corrections::CorrectionView {
                    id: c.id.clone(),
                    paper_id: paper.id.clone(),
                    paper_title: paper.title.clone(),
                    item_key: c.item_key.clone(),
                    item_type: crate::provenance::item_type(&c.item_key).into(),
                    field: c.field.clone(),
                    original: c.original.clone(),
                    corrected: c.corrected.clone(),
                    rule: paper.item_provenance.get(&c.item_key)
                        .map(|p| p.rule.clone()).unwrap_or_default(),
                    created_at: c.created_at,
                    updated_at: c.updated_at,
                });
            }
        }
        out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(out)
    }

    pub fn apply_correction(&self, paper_id: &str, req: CreateCorrection) -> AppResult<FieldCorrection> {
        let item_key = req.item_key.trim().to_string();
        let field = req.field.trim().to_string();
        if item_key.is_empty() || field.is_empty() {
            return Err(AppError::BadRequest("item_key and field are required".into()));
        }
        let saved = {
            let mut store = self.write()?;
            let paper = store.find_paper_mut(paper_id)
                .ok_or_else(|| AppError::NotFound(format!("paper '{paper_id}'")))?;
            let saved = crate::corrections::apply_correction(paper, &item_key, &field, &req.corrected)?;
            paper.updated_at = Utc::now();
            saved
        };
        self.rebuild_graph()?;
        self.flush()?;
        Ok(saved)
    }

    pub fn revert_correction(&self, paper_id: &str, correction_id: &str) -> AppResult<()> {
        {
            let mut store = self.write()?;
            let paper = store.find_paper_mut(paper_id)
                .ok_or_else(|| AppError::NotFound(format!("paper '{paper_id}'")))?;
            crate::corrections::revert_correction(paper, correction_id)?;
            paper.updated_at = Utc::now();
        }
        self.rebuild_graph()?;
        self.flush()
    }

    pub fn quality_report(&self) -> AppResult<crate::corrections::QualityReport> {
        Ok(crate::corrections::quality_report(&self.read()?.papers))
    }

    // -- AI guardrails --------------------------------------------------------

    pub fn insight_report(&self, paper_id: &str) -> AppResult<crate::guardrails::GuardrailReport> {
        Ok(crate::guardrails::report(&self.get_paper(paper_id)?))
    }

    pub fn verify_insight(&self, paper_id: &str, key: &str) -> AppResult<InsightAnchor> {
        let saved = {
            let mut store = self.write()?;
            let paper = store.find_paper_mut(paper_id)
                .ok_or_else(|| AppError::NotFound(format!("paper '{paper_id}'")))?;
            crate::guardrails::verify_insight(paper, key)?
        };
        self.flush()?;
        Ok(saved)
    }

    pub fn set_insight_verification(
        &self, paper_id: &str, key: &str, verification: InsightVerification, note: String,
    ) -> AppResult<InsightAnchor> {
        let saved = {
            let mut store = self.write()?;
            let paper = store.find_paper_mut(paper_id)
                .ok_or_else(|| AppError::NotFound(format!("paper '{paper_id}'")))?;
            crate::guardrails::set_verification(paper, key, verification, note)?
        };
        self.flush()?;
        Ok(saved)
    }

    pub fn set_insight_section(&self, paper_id: &str, key: &str, section: String) -> AppResult<InsightAnchor> {
        let saved = {
            let mut store = self.write()?;
            let paper = store.find_paper_mut(paper_id)
                .ok_or_else(|| AppError::NotFound(format!("paper '{paper_id}'")))?;
            crate::guardrails::set_section(paper, key, section)?
        };
        self.flush()?;
        Ok(saved)
    }

    pub fn rebuild_insight_anchors(&self, paper_id: &str) -> AppResult<usize> {
        let changed = {
            let mut store = self.write()?;
            let paper = store.find_paper_mut(paper_id)
                .ok_or_else(|| AppError::NotFound(format!("paper '{paper_id}'")))?;
            crate::guardrails::ensure_anchors(paper, None)
        };
        if changed { self.flush()?; }
        Ok(if changed { 1 } else { 0 })
    }

    pub fn backfill_provenance(&self) -> AppResult<usize> {
        let added = {
            let mut store = self.write()?;
            crate::provenance::backfill(&mut store.papers)
        };
        if added > 0 { self.flush()?; }
        Ok(added)
    }

    /// Refresh AI provenance after the enhancement handler changed a paper.
    pub fn refresh_enhancement_provenance(&self, id: &str) -> AppResult<()> {
        {
            let mut store = self.write()?;
            let paper = store.find_paper_mut(id)
                .ok_or_else(|| AppError::NotFound(format!("paper '{id}'")))?;
            crate::provenance::refresh_enhancement_provenance(paper);
        }
        self.flush()
    }

    pub fn reset(&self) -> AppResult<()> {
        {
            let mut store = self.write()?;
            *store = seeded_store();
        }
        self.rebuild_graph()?;
        self.flush()?;
        Ok(())
    }

    pub fn rebuild_graph(&self) -> AppResult<()> {
        let mut store = self.write()?;
        // Re-resolve bibliography entries against the current corpus so a
        // reference becomes a citation edge as soon as its paper is added.
        crate::citations::resolve_all(&mut store.papers);
        let custom = store.custom_edges.clone();
        store.graph = graph::rebuild(&store.papers, &custom);
        Ok(())
    }

    pub fn flush(&self) -> AppResult<()> {
        let store = self.read()?;
        let json = serde_json::to_string_pretty(&*store)?;
        write_atomic(&self.path, &json)?;
        Ok(())
    }
}

fn source_name(source: &PaperSource) -> &'static str {
    match source {
        PaperSource::Curated => "curated",
        PaperSource::Heuristic => "heuristic",
        PaperSource::LlmAssisted => "llm_assisted",
        PaperSource::Manual => "manual",
        PaperSource::Pdf => "pdf",
    }
}

fn status_name(status: &PaperStatus) -> &'static str {
    match status {
        PaperStatus::Ready => "ready",
        PaperStatus::Processing => "processing",
        PaperStatus::Failed => "failed",
    }
}

fn build_facets(papers: &[Paper], reading: &[ReadingState]) -> LibraryFacets {
    use std::collections::HashMap;

    let defs = categories::category_defs();
    let category_labels: HashMap<String, String> = defs.iter()
        .map(|d| (d.id.clone(), d.label.clone())).collect();

    let mut cat_counts: HashMap<String, usize> = HashMap::new();
    let mut tag_counts: HashMap<String, usize> = HashMap::new();
    let mut source_counts: HashMap<String, usize> = HashMap::new();
    let mut status_counts: HashMap<String, usize> = HashMap::new();
    let mut year_counts: HashMap<String, usize> = HashMap::new();
    let mut reading_counts: HashMap<String, usize> = HashMap::new();
    let mut priority_counts: HashMap<String, usize> = HashMap::new();
    let reading_by_id: HashMap<&str, &ReadingState> = reading.iter()
        .map(|r| (r.paper_id.as_str(), r)).collect();

    for p in papers {
        for c in &p.categories {
            *cat_counts.entry(c.clone()).or_insert(0) += 1;
        }
        for t in &p.tags {
            *tag_counts.entry(t.clone()).or_insert(0) += 1;
        }
        *source_counts.entry(source_name(&p.source).into()).or_insert(0) += 1;
        *status_counts.entry(status_name(&p.status).into()).or_insert(0) += 1;
        if let Some(y) = p.year {
            *year_counts.entry(y.to_string()).or_insert(0) += 1;
        }

        let state = reading_by_id.get(p.id.as_str());
        let rstatus = match state.map(|s| s.status) {
            Some(ReadingStatus::InProgress) => "in_progress",
            Some(ReadingStatus::Done) => "done",
            _ => "unread",
        };
        let rpriority = match state.map(|s| s.priority) {
            Some(ReadingPriority::High) => "high",
            Some(ReadingPriority::Low) => "low",
            _ => "medium",
        };
        *reading_counts.entry(rstatus.into()).or_insert(0) += 1;
        *priority_counts.entry(rpriority.into()).or_insert(0) += 1;
    }

    // Categories follow the taxonomy order (with its friendly labels).
    let mut category_facets: Vec<FacetCount> = defs.iter()
        .filter_map(|d| cat_counts.get(&d.id).map(|count| FacetCount {
            value: d.id.clone(), label: d.label.clone(), count: *count,
        }))
        .collect();
    // Include any custom/unknown category that is not in the taxonomy.
    for (value, count) in &cat_counts {
        if !defs.iter().any(|d| &d.id == value) {
            category_facets.push(FacetCount {
                value: value.clone(),
                label: category_labels.get(value).cloned().unwrap_or_else(|| value.clone()),
                count: *count,
            });
        }
    }

    let count_vec = |counts: HashMap<String, usize>, by_count: bool| -> Vec<FacetCount> {
        let mut v: Vec<FacetCount> = counts.into_iter()
            .map(|(value, count)| FacetCount { label: value.clone(), value, count })
            .collect();
        if by_count {
            v.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.label.cmp(&b.label)));
        } else {
            v.sort_by(|a, b| b.value.cmp(&a.value));
        }
        v
    };

    let mut tags = count_vec(tag_counts, true);
    tags.truncate(40);

    let mut years = count_vec(year_counts, false);
    years.truncate(30);

    // Reading facets have a stable, human-meaningful order rather than a
    // frequency sort: Unread → In progress → Done, and High → Medium → Low.
    let fixed_order = |counts: &HashMap<String, usize>, order: &[&str]| -> Vec<FacetCount> {
        let labels: HashMap<&str, &str> = [
            ("unread", "Unread"), ("in_progress", "In progress"), ("done", "Done"),
            ("high", "High"), ("medium", "Medium"), ("low", "Low"),
        ].into_iter().collect();
        order.iter().filter_map(|key| counts.get(*key).map(|count| FacetCount {
            value: (*key).to_string(),
            label: labels.get(key).unwrap_or(key).to_string(),
            count: *count,
        })).collect()
    };

    LibraryFacets {
        categories: category_facets,
        tags,
        sources: count_vec(source_counts, true),
        statuses: count_vec(status_counts, true),
        years,
        reading_statuses: fixed_order(&reading_counts, &["unread", "in_progress", "done"]),
        priorities: fixed_order(&priority_counts, &["high", "medium", "low"]),
    }
}

fn seeded_store() -> Store {
    let mut store = Store::new();
    store.papers.push(seed_paper());
    store
}

fn write_atomic(path: &Path, contents: &str) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, contents)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

pub fn ensure_seed(store: &StoreHandle) -> AppResult<()> {
    if store.read()?.find_paper(SEED_ID).is_none() {
        store.upsert_paper(seed_paper())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn temp_store() -> (StoreHandle, std::path::PathBuf) {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!(
            "paper-explainer-reading-{}-{}.json", std::process::id(), n,
        ));
        let _ = fs::remove_file(&path);
        (StoreHandle::open_at(path.clone()).unwrap(), path)
    }

    #[test]
    fn reading_state_defaults_to_unread_for_known_paper() {
        let (store, path) = temp_store();
        let seed = seed_paper();
        let state = store.get_reading_state(&seed.id).unwrap();
        assert_eq!(state.status, ReadingStatus::Unread);
        assert_eq!(state.priority, ReadingPriority::Medium);
        assert!(state.why.is_empty());
        assert!(store.get_reading_state("does-not-exist").is_err());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn first_view_write_auto_tracks_as_in_progress() {
        let (store, path) = temp_store();
        let seed = seed_paper();
        let state = store.upsert_reading_state(&seed.id, UpdateReadingState {
            last_view: Some("overview".into()), ..Default::default()
        }).unwrap();
        assert_eq!(state.status, ReadingStatus::InProgress);
        assert_eq!(state.last_view, "overview");
        // A second view write should not change an explicitly set status.
        let state = store.upsert_reading_state(&seed.id, UpdateReadingState {
            status: Some(ReadingStatus::Done), ..Default::default()
        }).unwrap();
        assert_eq!(state.status, ReadingStatus::Done);
        let state = store.upsert_reading_state(&seed.id, UpdateReadingState {
            last_view: Some("read".into()), ..Default::default()
        }).unwrap();
        assert_eq!(state.status, ReadingStatus::Done);
        assert_eq!(state.last_view, "read");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn library_filters_and_facets_reflect_reading_state() {
        let (store, path) = temp_store();
        let seed = seed_paper();
        store.upsert_reading_state(&seed.id, UpdateReadingState {
            status: Some(ReadingStatus::Done),
            priority: Some(ReadingPriority::High),
            why: Some("Need the O_ext caveat".into()),
            ..Default::default()
        }).unwrap();

        let all = store.query_library(&LibraryQuery::default()).unwrap();
        assert_eq!(all.total, 1);
        assert_eq!(all.papers.len(), 1);
        assert_eq!(all.reading.len(), 1);
        assert_eq!(all.reading[0].status, ReadingStatus::Done);
        assert_eq!(all.reading[0].why, "Need the O_ext caveat");
        assert!(all.facets.reading_statuses.iter().any(|f| f.value == "done" && f.count == 1));
        assert!(all.facets.priorities.iter().any(|f| f.value == "high" && f.count == 1));

        let done = store.query_library(&LibraryQuery {
            reading_status: "done".into(), ..Default::default()
        }).unwrap();
        assert_eq!(done.total, 1);
        let unread = store.query_library(&LibraryQuery {
            reading_status: "unread".into(), ..Default::default()
        }).unwrap();
        assert_eq!(unread.total, 0);
        let high = store.query_library(&LibraryQuery {
            priority: "high".into(), ..Default::default()
        }).unwrap();
        assert_eq!(high.total, 1);
        let low = store.query_library(&LibraryQuery {
            priority: "low".into(), ..Default::default()
        }).unwrap();
        assert_eq!(low.total, 0);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn reading_facets_count_untracked_papers_as_unread_medium() {
        let (store, path) = temp_store();
        let guard = store.read().unwrap();
        let facets = build_facets(&guard.papers, &[]);
        assert!(facets.reading_statuses.iter().any(|f| f.value == "unread" && f.count == 1));
        assert!(facets.priorities.iter().any(|f| f.value == "medium" && f.count == 1));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn provenance_overrides_clamp_confidence_and_survive_reimport() {
        let (store, path) = temp_store();
        let seed = seed_paper();
        assert!(store.get_paper(&seed.id).unwrap().item_provenance.contains_key("summary"));

        let saved = store.update_provenance(&seed.id, "summary", UpdateProvenance {
            verified: Some(true), confidence: Some(5.0), note: Some("checked §1".into()),
            ..Default::default()
        }).unwrap();
        assert_eq!(saved.kind, ProvenanceKind::Verified);
        assert_eq!(saved.origin, ProvenanceKind::Curated);
        assert_eq!(saved.confidence, 1.0);
        assert!(saved.verified_at.is_some());

        // Re-import replaces extracted content but the human edit remains.
        let mut replacement = seed_paper();
        replacement.title = "Re-imported title".into();
        let updated = store.upsert_paper(replacement).unwrap();
        let prov = &updated.item_provenance["summary"];
        assert_eq!(prov.note, "checked §1");
        assert_eq!(prov.kind, ProvenanceKind::Verified);

        let report = store.provenance_report(Some(&seed.id)).unwrap();
        assert!(report.items.iter().any(|i| i.key == "summary" && i.verified));
        let _ = fs::remove_file(path);
    }
}
