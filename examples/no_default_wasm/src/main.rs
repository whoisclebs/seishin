#![allow(dead_code)]

use seishin::prelude::*;

struct NoDefaultWasmGame;

impl Game2D for NoDefaultWasmGame {
    fn new(_context: &mut StartupContext) -> GameResult<Self> {
        Ok(Self)
    }
}

seishin::seishin_main!(NoDefaultWasmGame);
