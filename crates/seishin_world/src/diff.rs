use std::collections::{BTreeMap, BTreeSet};

use seishin_core::EntityId;

use crate::document::{SceneDocument, SceneEntityDocument};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SceneDiff {
    changes: Vec<SceneChange>,
}

impl SceneDiff {
    pub fn between(old: &SceneDocument, new: &SceneDocument) -> Result<Self, SceneDiffError> {
        let old_entities = index_entities(old, SceneDiffSide::Old)?;
        let new_entities = index_entities(new, SceneDiffSide::New)?;
        let mut ids = BTreeSet::new();
        ids.extend(old_entities.keys().copied());
        ids.extend(new_entities.keys().copied());

        let changes = ids
            .into_iter()
            .filter_map(|id| match (old_entities.get(&id), new_entities.get(&id)) {
                (Some(_), None) => Some(SceneChange::Removed { id }),
                (None, Some(entity)) => Some(SceneChange::Added {
                    entity: entity.clone(),
                }),
                (Some(old), Some(new)) if old != new => Some(SceneChange::Updated {
                    id,
                    entity: new.clone(),
                }),
                _ => None,
            })
            .collect();

        Ok(Self { changes })
    }

    pub fn changes(&self) -> &[SceneChange] {
        &self.changes
    }

    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    pub fn apply_to(&self, document: &mut SceneDocument) -> Result<(), SceneDiffError> {
        let mut entities = index_entities(document, SceneDiffSide::Target)?;

        for change in &self.changes {
            match change {
                SceneChange::Added { entity } => {
                    let id = entity_id(entity, SceneDiffSide::Patch, 0)?;
                    if entities.insert(id, entity.clone()).is_some() {
                        return Err(SceneDiffError::DuplicateEntityId {
                            side: SceneDiffSide::Target,
                            id,
                        });
                    }
                }
                SceneChange::Removed { id } => {
                    if entities.remove(id).is_none() {
                        return Err(SceneDiffError::MissingTargetEntity { id: *id });
                    }
                }
                SceneChange::Updated { id, entity } => {
                    if !entities.contains_key(id) {
                        return Err(SceneDiffError::MissingTargetEntity { id: *id });
                    }
                    entities.insert(*id, entity.clone());
                }
            }
        }

        document.entities = entities.into_values().collect();
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SceneChange {
    Added {
        entity: SceneEntityDocument,
    },
    Removed {
        id: EntityId,
    },
    Updated {
        id: EntityId,
        entity: SceneEntityDocument,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneDiffSide {
    Old,
    New,
    Target,
    Patch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SceneDiffError {
    MissingEntityId { side: SceneDiffSide, index: usize },
    DuplicateEntityId { side: SceneDiffSide, id: EntityId },
    MissingTargetEntity { id: EntityId },
}

impl std::fmt::Display for SceneDiffError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingEntityId { side, index } => {
                write!(f, "{side:?} scene entity at index {index} has no id")
            }
            Self::DuplicateEntityId { side, id } => {
                write!(f, "{side:?} scene has duplicate entity id {}", id.raw())
            }
            Self::MissingTargetEntity { id } => {
                write!(f, "target scene is missing entity {}", id.raw())
            }
        }
    }
}

impl std::error::Error for SceneDiffError {}

fn index_entities(
    document: &SceneDocument,
    side: SceneDiffSide,
) -> Result<BTreeMap<EntityId, SceneEntityDocument>, SceneDiffError> {
    let mut entities = BTreeMap::new();

    for (index, entity) in document.entities.iter().enumerate() {
        let id = entity_id(entity, side, index)?;
        if entities.insert(id, entity.clone()).is_some() {
            return Err(SceneDiffError::DuplicateEntityId { side, id });
        }
    }

    Ok(entities)
}

fn entity_id(
    entity: &SceneEntityDocument,
    side: SceneDiffSide,
    index: usize,
) -> Result<EntityId, SceneDiffError> {
    entity
        .id
        .map(EntityId::new)
        .ok_or(SceneDiffError::MissingEntityId { side, index })
}
