mod ffi;
mod types;

pub use ffi::{
    seishin_engine_create, seishin_engine_destroy, seishin_engine_frame, seishin_engine_tick,
};
pub use types::{SeishinEngine, SeishinEngineConfig, SeishinStatus};
