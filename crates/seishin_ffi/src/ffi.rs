use seishin_core::{Engine, EngineConfig};
use std::ffi::CStr;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

use crate::{SeishinEngine, SeishinEngineConfig, SeishinStatus};

/// Creates an engine handle for C ABI consumers.
///
/// # Safety
///
/// `out_engine` must be either null or a valid writable pointer to a
/// `*mut SeishinEngine`. If `config.app_name` is non-null, it must point to a
/// valid NUL-terminated string for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn seishin_engine_create(
    config: SeishinEngineConfig,
    out_engine: *mut *mut SeishinEngine,
) -> SeishinStatus {
    ffi_guard(|| {
        if out_engine.is_null() {
            return SeishinStatus::NullPointer;
        }

        unsafe { ptr::write(out_engine, ptr::null_mut()) };

        let Some(config) = engine_config_from_ffi(config) else {
            return SeishinStatus::InvalidArgument;
        };

        match Engine::new(config) {
            Ok(engine) => {
                let handle = Box::new(SeishinEngine::new(engine));
                unsafe { ptr::write(out_engine, Box::into_raw(handle)) };
                SeishinStatus::Ok
            }
            Err(_) => SeishinStatus::InvalidArgument,
        }
    })
}

/// Destroys an engine handle previously returned by `seishin_engine_create`.
///
/// # Safety
///
/// `engine` must be a pointer returned by `seishin_engine_create` that has not
/// already been destroyed. Passing any other non-null pointer is undefined
/// behavior.
#[no_mangle]
pub unsafe extern "C" fn seishin_engine_destroy(engine: *mut SeishinEngine) -> SeishinStatus {
    ffi_guard(|| {
        if engine.is_null() {
            return SeishinStatus::NullPointer;
        }

        unsafe { drop(Box::from_raw(engine)) };
        SeishinStatus::Ok
    })
}

/// Advances the engine by one frame.
///
/// # Safety
///
/// `engine` must be a valid live pointer returned by `seishin_engine_create`.
#[no_mangle]
pub unsafe extern "C" fn seishin_engine_tick(
    engine: *mut SeishinEngine,
    delta_seconds: f32,
) -> SeishinStatus {
    ffi_guard(|| {
        let Some(engine) = engine_mut(engine) else {
            return SeishinStatus::NullPointer;
        };

        match engine.engine.tick(delta_seconds) {
            Ok(_) => SeishinStatus::Ok,
            Err(_) => SeishinStatus::InvalidArgument,
        }
    })
}

/// Writes the current frame count to `out_frame`.
///
/// # Safety
///
/// `engine` must be null or a valid live pointer returned by
/// `seishin_engine_create`. `out_frame` must be null or a valid writable pointer
/// to a `u64`.
#[no_mangle]
pub unsafe extern "C" fn seishin_engine_frame(
    engine: *const SeishinEngine,
    out_frame: *mut u64,
) -> SeishinStatus {
    ffi_guard(|| {
        if out_frame.is_null() {
            return SeishinStatus::NullPointer;
        }

        let Some(engine) = engine_ref(engine) else {
            return SeishinStatus::NullPointer;
        };

        unsafe { ptr::write(out_frame, engine.engine.frame()) };
        SeishinStatus::Ok
    })
}

pub(crate) fn ffi_guard(call: impl FnOnce() -> SeishinStatus) -> SeishinStatus {
    match catch_unwind(AssertUnwindSafe(call)) {
        Ok(status) => status,
        Err(_) => SeishinStatus::Panic,
    }
}

fn engine_config_from_ffi(config: SeishinEngineConfig) -> Option<EngineConfig> {
    let app_name = if config.app_name.is_null() {
        EngineConfig::default().app_name
    } else {
        unsafe { CStr::from_ptr(config.app_name) }
            .to_str()
            .ok()?
            .to_string()
    };

    Some(EngineConfig {
        app_name,
        target_fps: config.target_fps,
    })
}

fn engine_mut<'a>(engine: *mut SeishinEngine) -> Option<&'a mut SeishinEngine> {
    if engine.is_null() {
        None
    } else {
        unsafe { engine.as_mut() }
    }
}

fn engine_ref<'a>(engine: *const SeishinEngine) -> Option<&'a SeishinEngine> {
    if engine.is_null() {
        None
    } else {
        unsafe { engine.as_ref() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    fn create_test_engine() -> *mut SeishinEngine {
        let mut engine = ptr::null_mut();

        let status = unsafe {
            seishin_engine_create(
                SeishinEngineConfig {
                    app_name: ptr::null(),
                    target_fps: 60,
                },
                &mut engine,
            )
        };

        assert_eq!(status, SeishinStatus::Ok);
        assert!(!engine.is_null());
        engine
    }

    #[test]
    fn ffi_engine_can_create_tick_report_frame_and_destroy() {
        let engine = create_test_engine();

        assert_eq!(
            unsafe { seishin_engine_tick(engine, 1.0 / 60.0) },
            SeishinStatus::Ok
        );

        let mut frame = 0;
        assert_eq!(
            unsafe { seishin_engine_frame(engine, &mut frame) },
            SeishinStatus::Ok
        );
        assert_eq!(frame, 1);

        assert_eq!(unsafe { seishin_engine_destroy(engine) }, SeishinStatus::Ok);
    }

    #[test]
    fn ffi_rejects_null_output_engine_pointer() {
        let status = unsafe {
            seishin_engine_create(
                SeishinEngineConfig {
                    app_name: ptr::null(),
                    target_fps: 60,
                },
                ptr::null_mut(),
            )
        };

        assert_eq!(status, SeishinStatus::NullPointer);
    }

    #[test]
    fn ffi_rejects_null_engine_handles() {
        assert_eq!(
            unsafe { seishin_engine_tick(ptr::null_mut(), 1.0 / 60.0) },
            SeishinStatus::NullPointer
        );

        let mut frame = 0;
        assert_eq!(
            unsafe { seishin_engine_frame(ptr::null(), &mut frame) },
            SeishinStatus::NullPointer
        );

        assert_eq!(
            unsafe { seishin_engine_destroy(ptr::null_mut()) },
            SeishinStatus::NullPointer
        );
    }

    #[test]
    fn ffi_rejects_null_frame_output_pointer() {
        let engine = create_test_engine();

        assert_eq!(
            unsafe { seishin_engine_frame(engine, ptr::null_mut()) },
            SeishinStatus::NullPointer
        );

        assert_eq!(unsafe { seishin_engine_destroy(engine) }, SeishinStatus::Ok);
    }

    #[test]
    fn ffi_rejects_invalid_config() {
        let empty_name = CString::new(" ").unwrap();
        let mut engine = std::ptr::NonNull::<SeishinEngine>::dangling().as_ptr();

        let status = unsafe {
            seishin_engine_create(
                SeishinEngineConfig {
                    app_name: empty_name.as_ptr(),
                    target_fps: 60,
                },
                &mut engine,
            )
        };

        assert_eq!(status, SeishinStatus::InvalidArgument);
        assert!(engine.is_null());

        let mut engine = std::ptr::NonNull::<SeishinEngine>::dangling().as_ptr();
        let status = unsafe {
            seishin_engine_create(
                SeishinEngineConfig {
                    app_name: ptr::null(),
                    target_fps: 0,
                },
                &mut engine,
            )
        };

        assert_eq!(status, SeishinStatus::InvalidArgument);
        assert!(engine.is_null());
    }

    #[test]
    fn ffi_rejects_invalid_delta_time() {
        let engine = create_test_engine();

        assert_eq!(
            unsafe { seishin_engine_tick(engine, f32::NAN) },
            SeishinStatus::InvalidArgument
        );
        assert_eq!(
            unsafe { seishin_engine_tick(engine, -0.1) },
            SeishinStatus::InvalidArgument
        );

        let mut frame = 0;
        assert_eq!(
            unsafe { seishin_engine_frame(engine, &mut frame) },
            SeishinStatus::Ok
        );
        assert_eq!(frame, 0);

        assert_eq!(unsafe { seishin_engine_destroy(engine) }, SeishinStatus::Ok);
    }

    #[test]
    fn ffi_guard_converts_panic_to_status() {
        let status = ffi_guard(|| panic!("ffi panic smoke test"));

        assert_eq!(status, SeishinStatus::Panic);
    }
}
