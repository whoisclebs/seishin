pub mod builder;
pub mod diff;
pub mod document;
pub mod procedural;
pub mod record;
pub mod reload;
pub mod resolve;
pub mod save;
pub mod tile_map;
pub mod world;

pub use builder::{SceneDocumentBuilder, SceneEntityBuilder};
pub use diff::{SceneChange, SceneDiff, SceneDiffError, SceneDiffSide};
pub use document::{
    CustomComponentDocument, PrefabDocument, SceneAudioDocument, SceneDocument,
    SceneEntityDocument, SceneInstanceDocument, SceneMapDocument, SceneSpriteDocument,
    SceneTransformDocument, SceneUiDocument, SceneUiImageDocument, SceneUiInteractionDocument,
    SceneUiLayoutDocument, SceneUiTextDocument, TagsDocument,
};
pub use procedural::{ProceduralRng, ProceduralSceneBuilder, ProceduralSeed};
pub use record::{
    AudioRef, CustomComponentRef, EntityRecord, InstanceSource, SpriteRef, UiAnchor,
    UiFlexDirection, UiImageRef, UiInteractionRef, UiLayoutRef, UiRef, UiTextRef,
};
pub use reload::{
    SceneReloadError, SceneReloadQueue, SceneReloadReport, SceneReloadRequest, SceneReloadResult,
    SceneReloadUpdate,
};
pub use resolve::{resolve_scene_entity, ResolveError, ResolvedEntity};
pub use save::{
    scene_document_export_from_resolved_entities, scene_document_from_records,
    scene_document_from_resolved_entities, SceneDocumentExport, SceneExportOmission,
};
pub use seishin_core::{EntityId, Transform2D};
pub use tile_map::{
    parse_tile_map, tile_map_to_scene_entities, ParsedTileMap, TileCell, TileMapError,
};
pub use world::{LoadedScene, SceneInstance, World, WorldError};

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::TypeId;

    #[test]
    fn world_crate_reexports_core_entity_primitives() {
        let entity = EntityId::new(7);
        let transform = Transform2D::from_translation(1.0, 2.0);

        assert_eq!(entity.raw(), 7);
        assert_eq!(transform.x, 1.0);
        assert_eq!(transform.y, 2.0);
    }

    #[test]
    fn world_exports_scene_document_with_stable_ids_and_record_fields() {
        struct PlayerController;

        let mut world = World::default();
        let entity = EntityId::new(10);
        let mut data_refs = std::collections::HashMap::new();
        data_refs.insert(
            "character".to_string(),
            "res://data/characters/player.toml".to_string(),
        );

        world
            .insert(
                entity,
                EntityRecord {
                    name: Some("Player".to_string()),
                    tags: vec!["player".to_string(), "spawn".to_string()],
                    data_refs,
                    custom_components: vec![CustomComponentRef {
                        type_name: "PlayerController".to_string(),
                        type_id: Some(TypeId::of::<PlayerController>()),
                        config: toml::Value::Table(
                            [("speed".to_string(), toml::Value::Float(180.0))]
                                .into_iter()
                                .collect(),
                        ),
                    }],
                    transform: Transform2D {
                        x: 4.0,
                        y: 8.0,
                        rotation_radians: 1.5,
                        scale_x: 2.0,
                        scale_y: 3.0,
                    },
                    sprite: Some(SpriteRef {
                        texture: "asset://sprites/player.png".to_string(),
                        width: Some(96.0),
                        height: Some(96.0),
                        source_x: None,
                        source_y: None,
                        source_width: None,
                        source_height: None,
                        layer: 0,
                        sort_order: 0,
                        tint: None,
                    }),
                    audio: Some(AudioRef {
                        sound: "asset://audio/step.wav".to_string(),
                    }),
                    ui: None,
                    instance_source: Some(InstanceSource {
                        scene: "res://scenes/source.scene.toml".to_string(),
                        source_entity: EntityId::new(1),
                    }),
                },
            )
            .expect("insert entity");

        let export = world.to_scene_document_export();
        let scene = export.document();
        let exported = scene.entities.first().expect("exported entity");

        assert_eq!(exported.id, Some(10));
        assert_eq!(exported.name.as_deref(), Some("Player"));
        assert_eq!(
            exported.tags.as_ref().map(|tags| tags.values.as_slice()),
            Some(["player".to_string(), "spawn".to_string()].as_slice())
        );
        assert_eq!(
            exported
                .data
                .as_ref()
                .and_then(|data| data.get("character"))
                .map(String::as_str),
            Some("res://data/characters/player.toml")
        );
        assert_eq!(exported.transform.expect("transform").scale_y, Some(3.0));
        assert_eq!(
            exported
                .sprite
                .as_ref()
                .and_then(|sprite| sprite.texture.as_deref()),
            Some("asset://sprites/player.png")
        );
        assert_eq!(
            exported
                .audio
                .as_ref()
                .and_then(|audio| audio.sound.as_deref()),
            Some("asset://audio/step.wav")
        );
        assert_eq!(exported.components.len(), 1);
        assert_eq!(exported.components[0].type_name, "PlayerController");
        assert_eq!(
            exported.components[0]
                .config
                .get("speed")
                .and_then(toml::Value::as_float),
            Some(180.0)
        );
        assert_eq!(
            exported
                .instance
                .as_ref()
                .map(|instance| (instance.scene.as_str(), instance.source_entity)),
            Some(("res://scenes/source.scene.toml", 1))
        );
        assert!(export.omissions().is_empty());

        let round_trip =
            SceneDocument::from_toml_str(&scene.to_toml_string().expect("serialize scene"))
                .expect("parse serialized scene");
        assert_eq!(round_trip, *scene);
    }

    #[test]
    fn resolved_entities_export_prefab_paths_when_available() {
        let scene = scene_document_from_resolved_entities([ResolvedEntity {
            id: Some(EntityId::new(7)),
            prefab: Some("res://prefabs/player.prefab.toml".to_string()),
            record: EntityRecord::named("Player"),
        }]);

        assert_eq!(scene.entities.len(), 1);
        assert_eq!(scene.entities[0].id, Some(7));
        assert_eq!(
            scene.entities[0].prefab.as_deref(),
            Some("res://prefabs/player.prefab.toml")
        );
    }

    #[test]
    fn resolved_entities_export_is_sorted_by_entity_id() {
        let scene = scene_document_from_resolved_entities([
            ResolvedEntity {
                id: Some(EntityId::new(9)),
                prefab: None,
                record: EntityRecord::named("High"),
            },
            ResolvedEntity {
                id: Some(EntityId::new(2)),
                prefab: None,
                record: EntityRecord::named("Low"),
            },
        ]);

        assert_eq!(scene.entities[0].id, Some(2));
        assert_eq!(scene.entities[1].id, Some(9));
    }

    #[test]
    fn scene_document_toml_serializes_maps_in_stable_key_order() {
        let mut world = World::default();
        let mut data_refs = std::collections::HashMap::new();
        data_refs.insert("zeta".to_string(), "last".to_string());
        data_refs.insert("alpha".to_string(), "first".to_string());

        world
            .insert(
                EntityId::new(1),
                EntityRecord {
                    data_refs,
                    custom_components: vec![CustomComponentRef {
                        type_name: "OrderedConfig".to_string(),
                        type_id: None,
                        config: toml::Value::Table(
                            [
                                ("zeta".to_string(), toml::Value::Integer(2)),
                                ("alpha".to_string(), toml::Value::Integer(1)),
                            ]
                            .into_iter()
                            .collect(),
                        ),
                    }],
                    ..EntityRecord::default()
                },
            )
            .expect("insert entity");

        let toml = world
            .to_scene_document()
            .to_toml_string()
            .expect("serialize scene");

        assert!(
            toml.find("alpha = \"first\"").expect("alpha data key")
                < toml.find("zeta = \"last\"").expect("zeta data key")
        );
        assert!(
            toml.find("alpha = 1").expect("alpha config key")
                < toml.find("zeta = 2").expect("zeta config key")
        );
    }

    #[test]
    fn export_reports_custom_component_config_that_scene_documents_cannot_represent() {
        let mut world = World::default();
        let entity = world.spawn(EntityRecord {
            custom_components: vec![CustomComponentRef {
                type_name: "ScalarConfig".to_string(),
                type_id: None,
                config: toml::Value::String("not-a-table".to_string()),
            }],
            ..EntityRecord::default()
        });

        let export = world.to_scene_document_export();

        assert_eq!(export.document().entities[0].components[0].config.len(), 0);
        assert_eq!(
            export.omissions(),
            &[SceneExportOmission::CustomComponentConfigNotRepresented {
                entity: Some(entity),
                type_name: "ScalarConfig".to_string(),
            }]
        );
    }

    #[test]
    fn scene_diff_lists_removed_updated_and_added_entities_by_id() {
        let old = SceneDocument {
            maps: Vec::new(),
            entities: vec![
                scene_entity(8, "Unchanged"),
                scene_entity(2, "Removed"),
                scene_entity(5, "OldName"),
            ],
        };
        let new = SceneDocument {
            maps: Vec::new(),
            entities: vec![
                scene_entity(7, "Added"),
                scene_entity(8, "Unchanged"),
                scene_entity(5, "NewName"),
            ],
        };

        let diff = SceneDiff::between(&old, &new).expect("scene diff");

        assert_eq!(
            diff.changes(),
            &[
                SceneChange::Removed {
                    id: EntityId::new(2)
                },
                SceneChange::Updated {
                    id: EntityId::new(5),
                    entity: scene_entity(5, "NewName"),
                },
                SceneChange::Added {
                    entity: scene_entity(7, "Added"),
                },
            ]
        );
    }

    #[test]
    fn scene_diff_applies_to_document_and_normalizes_entity_order() {
        let mut target = SceneDocument {
            maps: Vec::new(),
            entities: vec![scene_entity(5, "Removed"), scene_entity(1, "OldName")],
        };
        let new = SceneDocument {
            maps: Vec::new(),
            entities: vec![scene_entity(3, "Added"), scene_entity(1, "NewName")],
        };
        let diff = SceneDiff::between(&target, &new).expect("scene diff");

        diff.apply_to(&mut target).expect("apply diff");

        assert_eq!(
            target,
            SceneDocument {
                maps: Vec::new(),
                entities: vec![scene_entity(1, "NewName"), scene_entity(3, "Added")]
            }
        );
    }

    #[test]
    fn scene_diff_requires_explicit_unique_entity_ids() {
        let old = SceneDocument {
            maps: Vec::new(),
            entities: vec![SceneEntityDocument::default()],
        };
        let new = SceneDocument::default();

        let error = SceneDiff::between(&old, &new).expect_err("missing ids cannot diff");

        assert_eq!(
            error,
            SceneDiffError::MissingEntityId {
                side: SceneDiffSide::Old,
                index: 0,
            }
        );
    }

    #[test]
    fn ui_records_round_trip_through_scene_resolution_world_and_toml() {
        let scene = SceneDocument::from_toml_str(
            r##"
            [[entities]]
            id = 1
            name = "StartButton"

            [entities.ui.layout]
            parent = 99
            anchor = "center"
            offset_x = 8.0
            offset_y = 12.0
            width = 180.0
            height = 48.0
            z_index = 5
            flex_direction = "column"
            gap = 6.0
            padding = 4.0
            grow = 1.0

            [entities.ui.text]
            value = "Start"
            font_size = 18.0
            color = "#ffffff"

            [entities.ui.interaction]
            action = "start_game"
            enabled = true
            "##,
        )
        .expect("parse scene ui");
        let resolved = resolve_scene_entity(scene.entities[0].clone(), None).expect("resolve ui");
        let mut world = World::default();

        world.load_resolved([resolved]).expect("load ui entity");

        let ui = world.ui(EntityId::new(1)).expect("ui component");
        assert_eq!(ui.layout.parent, Some(EntityId::new(99)));
        assert_eq!(ui.layout.anchor, UiAnchor::Center);
        assert_eq!(ui.layout.width, 180.0);
        assert_eq!(ui.layout.flex_direction, UiFlexDirection::Column);
        assert_eq!(ui.layout.gap, 6.0);
        assert_eq!(ui.layout.padding, 4.0);
        assert_eq!(ui.layout.grow, 1.0);
        assert_eq!(
            ui.text.as_ref().map(|text| text.value.as_str()),
            Some("Start")
        );
        assert_eq!(
            ui.interaction
                .as_ref()
                .map(|interaction| interaction.action.as_str()),
            Some("start_game")
        );
        assert_eq!(world.entities_with_ui(), vec![EntityId::new(1)]);

        let exported = world.to_scene_document();
        let serialized = exported.to_toml_string().expect("serialize ui scene");
        let round_trip =
            SceneDocument::from_toml_str(&serialized).expect("parse serialized ui scene");

        assert_eq!(round_trip, exported);
    }

    #[test]
    fn sprite_order_and_instance_source_round_trip_through_scene_data() {
        let scene = SceneDocument::from_toml_str(
            r##"
            [[entities]]
            id = 10
            name = "Instanced Enemy"

            [entities.sprite]
            texture = "asset://sprites/enemy.png"
            width = 48.0
            height = 64.0
            layer = 2
            sort_order = -3
            tint = "#80ffcc99"

            [entities.instance]
            scene = "res://scenes/enemy_pack.scene.toml"
            source_entity = 4
            "##,
        )
        .expect("parse scene");
        let resolved = resolve_scene_entity(scene.entities[0].clone(), None).expect("resolve");
        let mut world = World::default();

        world.load_resolved([resolved]).expect("load scene");

        let entity = EntityId::new(10);
        let sprite = world.sprite(entity).expect("sprite");
        assert_eq!(sprite.layer, 2);
        assert_eq!(sprite.sort_order, -3);
        assert_eq!(sprite.tint.as_deref(), Some("#80ffcc99"));
        assert_eq!(
            world
                .entity(entity)
                .and_then(|record| record.instance_source.as_ref()),
            Some(&InstanceSource {
                scene: "res://scenes/enemy_pack.scene.toml".to_string(),
                source_entity: EntityId::new(4),
            })
        );

        let exported = world.to_scene_document_export();
        let exported_entity = &exported.document().entities[0];

        assert!(exported.omissions().is_empty());
        assert_eq!(
            exported_entity
                .sprite
                .as_ref()
                .and_then(|sprite| sprite.layer),
            Some(2)
        );
        assert_eq!(
            exported_entity
                .sprite
                .as_ref()
                .and_then(|sprite| sprite.sort_order),
            Some(-3)
        );
        assert_eq!(
            exported_entity
                .sprite
                .as_ref()
                .and_then(|sprite| sprite.tint.as_deref()),
            Some("#80ffcc99")
        );
        assert_eq!(
            exported_entity
                .instance
                .as_ref()
                .map(|instance| (instance.scene.as_str(), instance.source_entity)),
            Some(("res://scenes/enemy_pack.scene.toml", 4))
        );

        let serialized = exported
            .document()
            .to_toml_string()
            .expect("serialize scene");
        let round_trip = SceneDocument::from_toml_str(&serialized).expect("parse serialized scene");

        assert_eq!(round_trip, *exported.document());
    }

    #[test]
    fn world_queries_find_builtin_and_custom_components_in_entity_order() {
        struct EnemyBrain;

        let mut world = World::default();
        world
            .insert(
                EntityId::new(9),
                EntityRecord {
                    sprite: Some(SpriteRef {
                        texture: "asset://sprites/high.png".to_string(),
                        width: None,
                        height: None,
                        source_x: None,
                        source_y: None,
                        source_width: None,
                        source_height: None,
                        layer: 0,
                        sort_order: 0,
                        tint: None,
                    }),
                    audio: Some(AudioRef {
                        sound: "asset://audio/high.wav".to_string(),
                    }),
                    ui: Some(UiRef::text("High", UiAnchor::Center)),
                    custom_components: vec![CustomComponentRef {
                        type_name: "EnemyBrain".to_string(),
                        type_id: Some(TypeId::of::<EnemyBrain>()),
                        config: toml::Value::Table(Default::default()),
                    }],
                    ..EntityRecord::default()
                },
            )
            .expect("insert high entity");
        world
            .insert(
                EntityId::new(2),
                EntityRecord {
                    sprite: Some(SpriteRef {
                        texture: "asset://sprites/low.png".to_string(),
                        width: None,
                        height: None,
                        source_x: None,
                        source_y: None,
                        source_width: None,
                        source_height: None,
                        layer: 0,
                        sort_order: 0,
                        tint: None,
                    }),
                    audio: Some(AudioRef {
                        sound: "asset://audio/low.wav".to_string(),
                    }),
                    custom_components: vec![CustomComponentRef {
                        type_name: "EnemyBrain".to_string(),
                        type_id: None,
                        config: toml::Value::Table(Default::default()),
                    }],
                    ..EntityRecord::default()
                },
            )
            .expect("insert low entity");

        assert_eq!(
            world.entities_with_sprite(),
            vec![EntityId::new(2), EntityId::new(9)]
        );
        assert_eq!(
            world.entities_with_audio(),
            vec![EntityId::new(2), EntityId::new(9)]
        );
        assert_eq!(world.entities_with_ui(), vec![EntityId::new(9)]);
        assert_eq!(
            world.entities_with_component_type_name("EnemyBrain"),
            vec![EntityId::new(2), EntityId::new(9)]
        );
        assert_eq!(
            world.entities_with_component_type_id(TypeId::of::<EnemyBrain>()),
            vec![EntityId::new(9)]
        );
        assert_eq!(
            world
                .audio(EntityId::new(2))
                .map(|audio| audio.sound.as_str()),
            Some("asset://audio/low.wav")
        );
        assert_eq!(
            world
                .custom_components(EntityId::new(9))
                .map(|components| components.len()),
            Some(1)
        );
    }

    #[test]
    fn scene_document_builder_creates_stable_procedural_scene_documents() {
        let scene = SceneDocumentBuilder::new()
            .entity(
                SceneEntityBuilder::new()
                    .id(EntityId::new(10))
                    .named("High")
                    .tag("enemy")
                    .ui(UiRef::text("High", UiAnchor::Center)),
            )
            .entity(
                SceneEntityBuilder::new()
                    .id(EntityId::new(2))
                    .named("Low")
                    .sprite(SpriteRef {
                        texture: "asset://sprites/low.png".to_string(),
                        width: Some(32.0),
                        height: Some(32.0),
                        source_x: None,
                        source_y: None,
                        source_width: None,
                        source_height: None,
                        layer: 0,
                        sort_order: 0,
                        tint: None,
                    }),
            )
            .build();

        assert_eq!(scene.entities[0].id, Some(10));
        assert_eq!(scene.entities[1].id, Some(2));
        assert_eq!(scene.entities[0].tags.as_ref().unwrap().values, ["enemy"]);
        assert_eq!(
            scene.entities[0]
                .ui
                .as_ref()
                .and_then(|ui| ui.text.as_ref())
                .and_then(|text| text.value.as_deref()),
            Some("High")
        );
    }

    #[test]
    fn scene_document_builder_preserves_order_for_implicit_id_allocation() {
        let scene = SceneDocumentBuilder::new()
            .entity(SceneEntityBuilder::new().named("Implicit"))
            .entity(
                SceneEntityBuilder::new()
                    .id(EntityId::new(100))
                    .named("Explicit"),
            )
            .build();
        let resolved = scene
            .entities
            .into_iter()
            .map(|entity| resolve_scene_entity(entity, None).expect("resolve generated entity"))
            .collect::<Vec<_>>();
        let mut world = World::default();

        let loaded = world.load_resolved(resolved).expect("load generated scene");

        assert_eq!(loaded, vec![EntityId::new(1), EntityId::new(100)]);
        assert_eq!(world.name(EntityId::new(1)), Some("Implicit"));
        assert_eq!(world.name(EntityId::new(100)), Some("Explicit"));
    }

    #[test]
    fn procedural_rng_repeats_values_for_the_same_seed() {
        let seed = ProceduralSeed::from_u64(42);
        let mut left = ProceduralRng::new(seed);
        let mut right = ProceduralRng::new(seed);

        let left_values = [left.next_u64(), left.next_u64(), left.next_u64()];
        let right_values = [right.next_u64(), right.next_u64(), right.next_u64()];

        assert_eq!(left_values, right_values);
        assert_ne!(left_values, [0, 0, 0]);
    }

    #[test]
    fn procedural_scene_builder_generates_stable_ids_toml_and_loads_world() {
        fn generated_scene() -> SceneDocument {
            let mut scene = ProceduralSceneBuilder::new(ProceduralSeed::from_text("forest-room"));

            for index in 0..3 {
                let x = scene.rng_mut().range_f32(-8.0, 8.0);
                let y = scene.rng_mut().range_f32(-4.0, 4.0);
                scene.push_generated_entity(|entity, id| {
                    entity
                        .named(format!("Tree {}", id.raw()))
                        .tag("tree")
                        .data_ref("index", index.to_string())
                        .transform(Transform2D::from_translation(x, y))
                });
            }

            scene.build()
        }

        let first = generated_scene();
        let second = generated_scene();

        assert_eq!(first, second);
        assert_eq!(
            first
                .entities
                .iter()
                .map(|entity| entity.id)
                .collect::<Vec<_>>(),
            second
                .entities
                .iter()
                .map(|entity| entity.id)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            first.to_toml_string().expect("serialize generated scene"),
            second.to_toml_string().expect("serialize generated scene")
        );

        let resolved = first
            .entities
            .clone()
            .into_iter()
            .map(|entity| resolve_scene_entity(entity, None).expect("resolve generated entity"))
            .collect::<Vec<_>>();
        let mut world = World::default();
        let loaded = world.load_resolved(resolved).expect("load generated scene");

        assert_eq!(loaded.len(), 3);
        assert!(loaded
            .iter()
            .all(|entity| world.tags(*entity).is_some_and(|tags| tags == ["tree"])));
    }

    #[test]
    fn scene_reload_queue_applies_scene_updates_explicitly_and_preserves_ids() {
        let mut target = SceneDocument {
            maps: Vec::new(),
            entities: vec![scene_entity(5, "Removed"), scene_entity(1, "OldName")],
        };
        let updated = SceneDocument {
            maps: Vec::new(),
            entities: vec![scene_entity(1, "NewName"), scene_entity(3, "Added")],
        };
        let mut queue = SceneReloadQueue::default();

        queue.push_scene("res://scenes/main.scene.toml", updated.clone());
        let result = queue
            .apply_next(&mut target)
            .expect("apply scene reload")
            .expect("reload result");

        assert_eq!(result.source(), "res://scenes/main.scene.toml");
        assert_eq!(result.change_count(), 3);
        assert_eq!(target, updated);
        assert!(queue.is_empty());
    }

    #[test]
    fn scene_reload_queue_keeps_failed_update_pending() {
        let mut target = SceneDocument {
            maps: Vec::new(),
            entities: vec![scene_entity(1, "Player")],
        };
        let original = target.clone();
        let mut queue = SceneReloadQueue::default();

        queue.push_scene(
            "res://scenes/bad.scene.toml",
            SceneDocument {
                maps: Vec::new(),
                entities: vec![SceneEntityDocument::default()],
            },
        );
        let error = queue
            .apply_next(&mut target)
            .expect_err("missing entity id must fail");

        assert_eq!(
            error,
            SceneReloadError::Diff(SceneDiffError::MissingEntityId {
                side: SceneDiffSide::New,
                index: 0,
            })
        );
        assert_eq!(target, original);
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn scene_reload_queue_apply_all_is_atomic_when_later_update_fails() {
        let mut target = SceneDocument {
            maps: Vec::new(),
            entities: vec![scene_entity(1, "Player")],
        };
        let original = target.clone();
        let mut queue = SceneReloadQueue::default();

        queue.push_scene(
            "res://scenes/valid.scene.toml",
            SceneDocument {
                maps: Vec::new(),
                entities: vec![scene_entity(1, "Updated")],
            },
        );
        queue.push_scene(
            "res://scenes/bad.scene.toml",
            SceneDocument {
                maps: Vec::new(),
                entities: vec![SceneEntityDocument::default()],
            },
        );

        let error = queue
            .apply_all(&mut target)
            .expect_err("later missing entity id must fail atomically");

        assert_eq!(
            error,
            SceneReloadError::Diff(SceneDiffError::MissingEntityId {
                side: SceneDiffSide::New,
                index: 0,
            })
        );
        assert_eq!(target, original);
        assert_eq!(queue.len(), 2);
    }

    fn scene_entity(id: u64, name: &str) -> SceneEntityDocument {
        SceneEntityDocument {
            id: Some(id),
            name: Some(name.to_string()),
            ..SceneEntityDocument::default()
        }
    }
}
