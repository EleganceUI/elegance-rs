//! A timed, pure-Rust GPU producer + native Elegance UI integration smoke test.
//! No CPU pixel upload/readback. Run `cargo run -p elegance-wgpu --bin wgpu-image -- --smoke`.
use elegance::{App, AppHandle, ExternalGpuImageView, WindowDesc};
use elegance_wgpu::{wgpu, Bridge, Config, Error, GpuImageOrigin};
use std::{
    cell::RefCell,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

thread_local! { static APP: RefCell<Option<AppHandle>> = const { RefCell::new(None) }; }

fn block_on<T>(future: impl std::future::Future<Output = T>) -> T {
    struct Wake;
    impl std::task::Wake for Wake {
        fn wake(self: Arc<Self>) {}
    }
    let waker = std::task::Waker::from(Arc::new(Wake));
    let mut context = std::task::Context::from_waker(&waker);
    let mut future = std::pin::pin!(future);
    loop {
        if let std::task::Poll::Ready(value) = future.as_mut().poll(&mut context) {
            return value;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let smoke = std::env::args().any(|arg| arg == "--smoke");
    let two_windows = std::env::args().any(|arg| arg == "--two-windows");
    let frames = if smoke { 12 } else { 180 };

    let app = App::new();
    APP.with(|slot| *slot.borrow_mut() = Some(app.handle()));
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = block_on(instance.request_adapter(&Default::default()))?;
    let mut bridge = block_on(Bridge::new(
        &adapter,
        Config {
            width: 320,
            height: 240,
            origin: GpuImageOrigin::TopLeft,
            format: wgpu::TextureFormat::Bgra8Unorm,
            max_frames: 4,
        },
    ))?;

    app.window(
        WindowDesc::new("wgpu → Elegance").size(640, 480),
        ExternalGpuImageView::new(bridge.image()),
    );
    if two_windows {
        app.window(
            WindowDesc::new("Second consumer of the same GPU image").size(400, 300),
            ExternalGpuImageView::new(bridge.image()),
        );
    }

    let stopped = Arc::new(AtomicBool::new(false));
    let stop = stopped.clone();
    let producer = std::thread::spawn(move || -> Result<(), String> {
        // Always ask the UI to quit, including error/panic paths.
        struct Quit;
        impl Drop for Quit {
            fn drop(&mut self) {
                elegance::post_to_ui_thread(|| {
                    APP.with(|slot| {
                        if let Some(app) = slot.borrow().as_ref() {
                            app.quit();
                        }
                    })
                });
            }
        }

        let _quit = Quit;
        let deadline = Instant::now() + Duration::from_secs(20);
        let mut accepted = 0;
        while accepted < frames && !stop.load(Ordering::Acquire) {
            if Instant::now() >= deadline {
                return Err(format!("timed out after {accepted} accepted publications"));
            }
            let t = accepted as f64 / frames as f64;
            match bridge.clear_color(wgpu::Color {
                r: t,
                g: 0.2,
                b: 1.0 - t,
                a: 1.0,
            }) {
                Ok(()) => accepted += 1,
                Err(Error::Busy) => {}
                // The UI/render thread may not have initialized its device yet.
                Err(Error::Publish(e))
                    if e == elegance_sys::external_gpu_image::STATUS_DEVICE_UNAVAILABLE => {}
                Err(e) => return Err(format!("publication failed: {e}")),
            }
            std::thread::sleep(Duration::from_millis(16));
        }

        // Keep the final frame visible long enough for actual native rendering.
        std::thread::sleep(Duration::from_millis(200));
        bridge.clear();
        let deadline = Instant::now() + Duration::from_secs(5);
        while bridge.in_flight() != 0 && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(2));
        }
        let remaining = bridge.in_flight();
        let quarantined = bridge.quarantined();
        eprintln!("accepted={accepted}, in_flight={remaining}, quarantined={quarantined}");
        if accepted < frames || remaining != 0 || quarantined != 0 {
            return Err("native publication/retirement did not drain cleanly".into());
        }
        Ok(())
    });

    let result = app.run_code();
    stopped.store(true, Ordering::Release);
    let producer_result = producer.join().map_err(|_| "producer panicked")?;
    APP.with(|slot| *slot.borrow_mut() = None);

    let code = result?;
    if code != 0 {
        return Err(format!("native application exited with {code}").into());
    }

    producer_result.map_err(Into::into)
}
