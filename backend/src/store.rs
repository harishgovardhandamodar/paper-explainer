//! File-backed persistence with atomic writes.
//!
//! The store is intentionally swappable: `Store` is a plain serde value, so a
//! future SQLite/graph-database backend only needs to reimplement these methods.

use crate::error::{AppError, AppResult};
use crate::graph;
use crate::models::*;
use crate::seed::{seed_paper, SEED_ID};
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
        let store = if path.exists() {
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
