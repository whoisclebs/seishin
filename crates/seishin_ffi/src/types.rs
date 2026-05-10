use seishin_core::Engine;
use std::os::raw::c_char;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeishinStatus {
    Ok = 0,
    NullPointer = -1,
    InvalidArgument = -2,
    Panic = -255,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SeishinEngineConfig {
    pub app_name: *const c_char,
    pub target_fps: u32,
}

pub struct SeishinEngine {
    pub(crate) engine: Engine,
}

impl SeishinEngine {
    pub(crate) fn new(engine: Engine) -> Self {
        Self { engine }
    }
}
