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
        let store = self.read()?;
        let needle = params.q.trim().to_lowercase();
        let tokens: Vec<&str> = needle.split_whitespace().collect();
        let category = params.category.trim();
        let tag = params.tag.trim().to_lowercase();
        let source = params.source.trim().to_lowercase();
        let status = params.status.trim().to_lowercase();

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
        let papers: Vec<PaperSummary> = matched.into_iter()
            .skip(offset).take(limit).map(|p| p.to_summary()).collect();

        Ok(LibraryResponse {
            papers,
            total,
            collection_total: store.papers.len(),
            limit,
            offset,
            facets: build_facets(&store.papers),
            categories: categories::category_defs(),
        })
    }

    pub fn get_paper(&self, id: &str) -> AppResult<Paper> {
        self.read()?.find_paper(id).cloned().ok_or_else(|| AppError::NotFound(format!("paper '{id}'")))
    }

    pub fn upsert_paper(&self, paper: Paper) -> AppResult<Paper> {
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
    }
}

fn status_name(status: &PaperStatus) -> &'static str {
    match status {
        PaperStatus::Ready => "ready",
        PaperStatus::Processing => "processing",
        PaperStatus::Failed => "failed",
    }
}

fn build_facets(papers: &[Paper]) -> LibraryFacets {
    use std::collections::HashMap;

    let defs = categories::category_defs();
    let category_labels: HashMap<String, String> = defs.iter()
        .map(|d| (d.id.clone(), d.label.clone())).collect();

    let mut cat_counts: HashMap<String, usize> = HashMap::new();
    let mut tag_counts: HashMap<String, usize> = HashMap::new();
    let mut source_counts: HashMap<String, usize> = HashMap::new();
    let mut status_counts: HashMap<String, usize> = HashMap::new();
    let mut year_counts: HashMap<String, usize> = HashMap::new();

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

    LibraryFacets {
        categories: category_facets,
        tags,
        sources: count_vec(source_counts, true),
        statuses: count_vec(status_counts, true),
        years,
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
