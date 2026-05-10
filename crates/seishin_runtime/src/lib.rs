#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
mod block_on;
#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
mod desktop;
mod error;
mod headless;
#[cfg(feature = "mobile")]
mod mobile;
mod time;
#[cfg(all(target_arch = "wasm32", feature = "web"))]
mod web;

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub use desktop::{run_desktop, DesktopGame, DesktopRunConfig, WindowConfig, WindowSize};
pub use error::DesktopRuntimeError;
pub use headless::{run_headless, HeadlessRunConfig};
#[cfg(feature = "mobile")]
pub use mobile::{
    MobileLifecycleEvent, MobileRunConfig, MobileRuntime, MobileRuntimeState, MobileSurface,
    MobileTouchEvent, MobileTouchPhase,
};
pub use time::FixedTimestep;
#[cfg(all(target_arch = "wasm32", feature = "web"))]
pub use web::{run_web as run_desktop, DesktopGame, DesktopRunConfig, WindowConfig, WindowSize};

#[cfg(test)]
mod tests {
    #[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
    mod block_on {
        use std::{
            future::Future,
            pin::Pin,
            sync::{
                atomic::{AtomicBool, Ordering},
                Arc,
            },
            task::{Context, Poll},
            thread,
            time::Duration,
        };

        use crate::block_on::block_on;

        #[test]
        fn block_on_waits_until_a_future_wakes_the_current_thread() {
            let completed = Arc::new(AtomicBool::new(false));

            let result = block_on(WakeOnce::new(Arc::clone(&completed)));

            assert_eq!(result, "ready");
            assert!(completed.load(Ordering::Acquire));
        }

        struct WakeOnce {
            completed: Arc<AtomicBool>,
            spawned: bool,
        }

        impl WakeOnce {
            fn new(completed: Arc<AtomicBool>) -> Self {
                Self {
                    completed,
                    spawned: false,
                }
            }
        }

        impl Future for WakeOnce {
            type Output = &'static str;

            fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
                if self.completed.load(Ordering::Acquire) {
                    return Poll::Ready("ready");
                }

                if !self.spawned {
                    self.spawned = true;
                    let completed = Arc::clone(&self.completed);
                    let waker = context.waker().clone();

                    thread::spawn(move || {
                        thread::sleep(Duration::from_millis(5));
                        completed.store(true, Ordering::Release);
                        waker.wake();
                    });
                }

                Poll::Pending
            }
        }
    }

    #[cfg(feature = "mobile")]
    mod mobile {
        use seishin_core::{Engine, EngineConfig, EngineResult, Game, UpdateContext};

        use crate::{
            FixedTimestep, MobileLifecycleEvent, MobileRunConfig, MobileRuntime,
            MobileRuntimeState, MobileSurface, MobileTouchEvent, MobileTouchPhase,
        };

        struct CountingGame {
            ready: bool,
            updates: u64,
            shutdown: bool,
        }

        impl Game for CountingGame {
            fn ready(&mut self, _engine: &mut Engine) -> EngineResult<()> {
                self.ready = true;
                Ok(())
            }

            fn update(
                &mut self,
                _engine: &mut Engine,
                _context: UpdateContext,
            ) -> EngineResult<()> {
                self.updates += 1;
                Ok(())
            }

            fn shutdown(&mut self, _engine: &mut Engine) -> EngineResult<()> {
                self.shutdown = true;
                Ok(())
            }
        }

        #[test]
        fn mobile_runtime_tracks_lifecycle_surface_touch_and_updates() {
            let engine = Engine::new(EngineConfig::default()).expect("engine");
            let game = CountingGame {
                ready: false,
                updates: 0,
                shutdown: false,
            };
            let config = MobileRunConfig::new(FixedTimestep::from_fps(30))
                .with_user_data_root("app-data/save");
            let mut runtime = MobileRuntime::new(engine, game, config).expect("runtime");

            runtime.handle_event(MobileLifecycleEvent::Resumed);
            runtime.handle_event(MobileLifecycleEvent::SurfaceAvailable(MobileSurface::new(
                640, 360, 2.0,
            )));
            runtime.handle_event(MobileLifecycleEvent::Touch(MobileTouchEvent::new(
                42,
                MobileTouchPhase::Started,
                20.0,
                30.0,
            )));

            runtime.update_frame().expect("frame update");

            assert_eq!(runtime.state(), MobileRuntimeState::Running);
            assert_eq!(
                runtime.surface(),
                Some(MobileSurface::new(640, 360, 2.0)).as_ref()
            );
            assert!(runtime.input_state().touch_pressed(42));
            assert_eq!(runtime.game().updates, 1);
            assert_eq!(
                runtime
                    .config()
                    .user_data_root()
                    .map(|path| path.as_os_str()),
                Some(std::ffi::OsStr::new("app-data/save"))
            );

            runtime.handle_event(MobileLifecycleEvent::Paused);
            assert_eq!(runtime.state(), MobileRuntimeState::Paused);

            runtime.shutdown().expect("shutdown");
            assert!(runtime.game().ready);
            assert!(runtime.game().shutdown);
        }
    }
}
