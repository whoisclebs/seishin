use std::collections::VecDeque;

use crate::{
    diff::{SceneDiff, SceneDiffError},
    document::SceneDocument,
};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SceneReloadQueue {
    pending: VecDeque<SceneReloadRequest>,
}

impl SceneReloadQueue {
    pub fn push_scene(&mut self, source: impl Into<String>, scene: SceneDocument) {
        self.pending
            .push_back(SceneReloadRequest::scene(source, scene));
    }

    pub fn push_diff(&mut self, source: impl Into<String>, diff: SceneDiff) {
        self.pending
            .push_back(SceneReloadRequest::diff(source, diff));
    }

    pub fn apply_next(
        &mut self,
        target: &mut SceneDocument,
    ) -> Result<Option<SceneReloadResult>, SceneReloadError> {
        let Some(request) = self.pending.front().cloned() else {
            return Ok(None);
        };
        let mut staged = target.clone();
        let result = request.apply_to(&mut staged)?;

        *target = staged;
        self.pending.pop_front();
        Ok(Some(result))
    }

    pub fn apply_all(
        &mut self,
        target: &mut SceneDocument,
    ) -> Result<SceneReloadReport, SceneReloadError> {
        let mut staged_queue = self.clone();
        let mut staged_target = target.clone();
        let mut results = Vec::new();

        while let Some(result) = staged_queue.apply_next(&mut staged_target)? {
            results.push(result);
        }

        *target = staged_target;
        *self = staged_queue;

        Ok(SceneReloadReport { results })
    }

    pub fn len(&self) -> usize {
        self.pending.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneReloadRequest {
    source: String,
    update: SceneReloadUpdate,
}

impl SceneReloadRequest {
    pub fn scene(source: impl Into<String>, scene: SceneDocument) -> Self {
        Self {
            source: source.into(),
            update: SceneReloadUpdate::Scene(scene),
        }
    }

    pub fn diff(source: impl Into<String>, diff: SceneDiff) -> Self {
        Self {
            source: source.into(),
            update: SceneReloadUpdate::Diff(diff),
        }
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn apply_to(
        &self,
        target: &mut SceneDocument,
    ) -> Result<SceneReloadResult, SceneReloadError> {
        let change_count = match &self.update {
            SceneReloadUpdate::Scene(scene) => {
                let diff = SceneDiff::between(target, scene)?;
                let change_count = diff.changes().len();
                diff.apply_to(target)?;
                change_count
            }
            SceneReloadUpdate::Diff(diff) => {
                let change_count = diff.changes().len();
                diff.apply_to(target)?;
                change_count
            }
        };

        Ok(SceneReloadResult {
            source: self.source.clone(),
            change_count,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SceneReloadUpdate {
    Scene(SceneDocument),
    Diff(SceneDiff),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SceneReloadReport {
    results: Vec<SceneReloadResult>,
}

impl SceneReloadReport {
    pub fn results(&self) -> &[SceneReloadResult] {
        &self.results
    }

    pub fn total_change_count(&self) -> usize {
        self.results
            .iter()
            .map(SceneReloadResult::change_count)
            .sum()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneReloadResult {
    source: String,
    change_count: usize,
}

impl SceneReloadResult {
    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn change_count(&self) -> usize {
        self.change_count
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SceneReloadError {
    Diff(SceneDiffError),
}

impl std::fmt::Display for SceneReloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Diff(error) => write!(f, "scene reload diff failed: {error}"),
        }
    }
}

impl std::error::Error for SceneReloadError {}

impl From<SceneDiffError> for SceneReloadError {
    fn from(error: SceneDiffError) -> Self {
        Self::Diff(error)
    }
}
