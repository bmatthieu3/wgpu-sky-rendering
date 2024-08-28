#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

pub(crate) struct Clock {
    #[cfg(not(target_arch = "wasm32"))]
    instant: std::time::Instant,
    #[cfg(target_arch = "wasm32")]
    start: f32,
}

impl Clock {
    pub(crate) fn now() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let instant = Instant::now();
            Self { instant }
        }
        #[cfg(target_arch = "wasm32")]
        {
            let window = web_sys::window().expect("should have a window in this context");
            let performance = window
                .performance()
                .expect("performance should be available");
            let start = performance.now() as f32;

            Self { start }
        }
    }

    pub(crate) fn elapsed_as_secs(&self) -> f32 {
        #[cfg(target_arch = "wasm32")]
        {
            let window = web_sys::window().expect("should have a window in this context");
            let performance = window
                .performance()
                .expect("performance should be available");
            (performance.now() as f32 - self.start) / 1000.0
        }
        #[cfg(not(target_arch = "wasm32"))]
        self.instant.elapsed().as_secs_f32()
    }
}
