use seishin_core::EntityId;

use std::collections::BTreeMap;

use crate::{
    ParsedTileMap, SceneDocument, SceneEntityBuilder, SceneEntityDocument, TileCell,
    TileDefinition, TileSetDefinition,
};

const SPLITMIX_INCREMENT: u64 = 0x9E37_79B9_7F4A_7C15;
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProceduralSeed(u64);

impl ProceduralSeed {
    pub const fn from_u64(value: u64) -> Self {
        Self(value)
    }

    pub fn from_text(value: &str) -> Self {
        let mut hash = FNV_OFFSET;
        for byte in value.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        Self(hash)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProceduralRng {
    state: u64,
}

impl ProceduralRng {
    pub const fn new(seed: ProceduralSeed) -> Self {
        Self { state: seed.raw() }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(SPLITMIX_INCREMENT);
        mix_u64(self.state)
    }

    pub fn next_f32(&mut self) -> f32 {
        let value = (self.next_u64() >> 40) as u32;
        value as f32 / (1_u32 << 24) as f32
    }

    pub fn range_f32(&mut self, start: f32, end: f32) -> f32 {
        if !start.is_finite() || !end.is_finite() || end <= start {
            return start;
        }

        start + (end - start) * self.next_f32()
    }

    pub fn range_u32(&mut self, start: u32, end: u32) -> u32 {
        if end <= start {
            return start;
        }

        let width = u64::from(end - start);
        start + (self.next_u64() % width) as u32
    }

    pub fn choose_index(&mut self, len: usize) -> Option<usize> {
        if len == 0 {
            return None;
        }

        Some((self.next_u64() % len as u64) as usize)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProceduralSceneBuilder {
    rng: ProceduralRng,
    next_synthetic_id: u64,
    entities: Vec<SceneEntityDocument>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProceduralTileMapBuilder {
    rng: ProceduralRng,
    tile_size: f32,
    width: u32,
    height: u32,
    tileset: Option<TileSetDefinition>,
    legend: BTreeMap<u8, TileDefinition>,
    rows: Vec<Vec<TileCell>>,
    spawns: BTreeMap<String, (i32, i32)>,
}

impl ProceduralTileMapBuilder {
    pub fn new(seed: ProceduralSeed, width: u32, height: u32) -> Self {
        Self {
            rng: ProceduralRng::new(seed),
            tile_size: 80.0,
            width,
            height,
            tileset: None,
            legend: BTreeMap::new(),
            rows: Vec::new(),
            spawns: BTreeMap::new(),
        }
    }

    pub fn tile_size(mut self, tile_size: f32) -> Self {
        if tile_size > 0.0 {
            self.tile_size = tile_size;
        }
        self
    }

    pub fn tileset(mut self, tileset: TileSetDefinition) -> Self {
        self.tileset = Some(tileset);
        self
    }

    pub fn legend_tile(mut self, code: u8, definition: TileDefinition) -> Self {
        self.legend.insert(code, definition);
        self
    }

    pub fn spawn(mut self, name: impl Into<String>, column: i32, row: i32) -> Self {
        self.spawns.insert(name.into(), (column, row));
        self
    }

    pub fn fill(mut self, mut choose: impl FnMut(&mut ProceduralRng, u32, u32) -> u8) -> Self {
        self.rows = (0..self.height)
            .map(|row| {
                (0..self.width)
                    .map(|column| TileCell {
                        code: choose(&mut self.rng, column, row),
                    })
                    .collect()
            })
            .collect();
        self
    }

    pub fn build(mut self) -> ParsedTileMap {
        if self.rows.is_empty() {
            self = self.fill(|_, _, _| 0);
        }

        ParsedTileMap {
            tile_size: self.tile_size,
            tileset: self.tileset,
            legend: self.legend,
            rows: self.rows,
            spawns: self.spawns,
        }
    }
}

impl ProceduralSceneBuilder {
    pub fn new(seed: ProceduralSeed) -> Self {
        Self {
            rng: ProceduralRng::new(seed),
            next_synthetic_id: synthetic_id_base(seed),
            entities: Vec::new(),
        }
    }

    pub fn rng_mut(&mut self) -> &mut ProceduralRng {
        &mut self.rng
    }

    pub fn next_entity_id(&mut self) -> EntityId {
        loop {
            let id = self.next_synthetic_id;
            self.next_synthetic_id = self.next_synthetic_id.wrapping_add(1);

            if id != 0 && id != u64::MAX {
                return EntityId::new(id);
            }
        }
    }

    pub fn push_entity(&mut self, entity: SceneEntityBuilder) {
        self.entities.push(entity.build());
    }

    pub fn push_document_entity(&mut self, entity: SceneEntityDocument) {
        self.entities.push(entity);
    }

    pub fn push_generated_entity(
        &mut self,
        configure: impl FnOnce(SceneEntityBuilder, EntityId) -> SceneEntityBuilder,
    ) -> EntityId {
        let id = self.next_entity_id();
        let entity = configure(SceneEntityBuilder::new().id(id), id).build();
        self.entities.push(entity);
        id
    }

    pub fn build(self) -> SceneDocument {
        SceneDocument {
            maps: Vec::new(),
            entities: self.entities,
        }
    }
}

fn synthetic_id_base(seed: ProceduralSeed) -> u64 {
    const MAX_SYNTHETIC_ID_BASE: u64 = i64::MAX as u64 - 1_000_000;

    let mixed = mix_u64(seed.raw() ^ 0x53e1_5e1d_0000_0001);
    1 + mixed % MAX_SYNTHETIC_ID_BASE
}

fn mix_u64(value: u64) -> u64 {
    let mut value = value;
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}
