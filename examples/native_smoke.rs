//! Real macOS/native surface, engine input mapper, screenshots and frame timing.
use aige::{InputEvent, Key};
use aige_platform::{UserEvent, WindowOptions, WindowedApp};
use aige_render::Renderer;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::WindowId;

struct Check {
    inner: WindowedApp,
    events: Vec<(u64, Key, bool)>,
    next: usize,
    started: Instant,
    mapped: bool,
    redraws: u64,
}
impl ApplicationHandler<UserEvent> for Check {
    fn resumed(&mut self, e: &ActiveEventLoop) {
        self.inner.resumed(e);
        if let Some(window) = self.inner.window() {
            window.set_visible(true);
            window.focus_window();
        }
    }
    fn window_event(&mut self, e: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if matches!(&event, WindowEvent::RedrawRequested) {
            self.redraws += 1;
        }
        if let WindowEvent::Occluded(hidden) = &event {
            eprintln!("Native window occluded: {hidden}");
        }
        self.inner.window_event(e, id, event);
    }
    fn user_event(&mut self, e: &ActiveEventLoop, event: UserEvent) {
        self.inner.user_event(e, event);
    }
    fn about_to_wait(&mut self, e: &ActiveEventLoop) {
        self.inner.about_to_wait(e);
        while self.next < self.events.len()
            && self.inner.ticks_executed() >= self.events[self.next].0
        {
            let (_, key, pressed) = self.events[self.next];
            self.inner.queue_input(InputEvent::Key { key, pressed });
            self.next += 1;
        }
        if !self.mapped && self.inner.ticks_executed() >= 160 {
            // Exercise the actual OS keyboard mapping API as well as queued events.
            self.inner
                .on_key_input(PhysicalKey::Code(KeyCode::KeyF), ElementState::Pressed);
            self.inner
                .on_key_input(PhysicalKey::Code(KeyCode::KeyF), ElementState::Released);
            self.mapped = true;
        }
        if self.inner.ticks_executed() >= 180 {
            assert_eq!(self.inner.renderer().surface_drawable(), Some((1920, 1080)));
            eprintln!(
                "Redraw requests: {}; surface {:?}; visible {:?}; minimized {:?}; focus {}",
                self.redraws,
                self.inner.renderer().surface_drawable(),
                self.inner.window().unwrap().is_visible(),
                self.inner.window().unwrap().is_minimized(),
                self.inner.window().unwrap().has_focus()
            );
            eprintln!(
                "Native diagnostics: {} frames / {} ticks / {:?}; tick errors {:?}; render errors {:?}",
                self.inner.frames_presented(),
                self.inner.ticks_executed(),
                self.started.elapsed(),
                self.inner.tick_errors(),
                self.inner.render_errors()
            );
            assert!(
                self.inner.frames_presented() > 100,
                "native window must present frames"
            );
            assert!(self.inner.tick_errors().is_empty());
            assert!(self.inner.render_errors().is_empty());
            let g = self
                .inner
                .simulation()
                .world()
                .resource::<neon_clash::model::Game>()
                .unwrap();
            assert_eq!(g.phase, neon_clash::model::Phase::Fight);
            let phase = g.phase;
            let frames = self.inner.frames_presented();
            let frame = self.inner.render_offscreen(1920, 1080).unwrap();
            std::fs::write("artifacts/native-window.png", frame.png).unwrap();
            std::fs::write("artifacts/native-report.json", serde_json::to_string_pretty(&serde_json::json!({
                "frames_presented": frames, "ticks": self.inner.ticks_executed(),
                "resolution": [1920, 1080],
                "redraw_requests": self.redraws, "elapsed_seconds": self.started.elapsed().as_secs_f64(),
                "adapter": aige_render::gpu_adapter_info().unwrap(), "render_errors": 0, "tick_errors": 0,
                "phase": format!("{phase:?}")
            })).unwrap()).unwrap();
            eprintln!(
                "Native surface: {frames} frames, 180 ticks, zero render/tick errors; {}",
                aige_render::gpu_adapter_info().unwrap()
            );
            e.exit();
        }
        assert!(
            self.started.elapsed() < Duration::from_secs(15),
            "native rendering stalled"
        );
    }
}
fn main() {
    std::env::set_current_dir(env!("CARGO_MANIFEST_DIR")).unwrap();
    std::fs::create_dir_all("artifacts").unwrap();
    let sim = aige::build_simulation(neon_clash::build_game, 42).unwrap();
    let inner = WindowedApp::new(
        sim,
        Renderer::new("assets"),
        WindowOptions {
            title: "NEON CLASH | automatic native verification".to_owned(),
            width: 1920,
            height: 1080,
            ..Default::default()
        },
    )
    .unwrap();
    let mut events = Vec::new();
    for (tick, key) in [
        (5, Key::Down),
        (10, Key::Enter),
        (15, Key::Enter),
        (20, Key::Enter),
        (25, Key::Enter),
        (35, Key::D),
        (100, Key::J),
        (115, Key::K),
        (135, Key::U),
    ] {
        events.push((tick, key, true));
        events.push((if key == Key::D { 90 } else { tick + 1 }, key, false));
    }
    events.sort_by_key(|event| event.0);
    let mut check = Check {
        inner,
        events,
        next: 0,
        started: Instant::now(),
        mapped: false,
        redraws: 0,
    };
    let mut builder = EventLoop::<UserEvent>::with_user_event();
    #[cfg(target_os = "macos")]
    {
        use winit::platform::macos::EventLoopBuilderExtMacOS;
        builder.with_activate_ignoring_other_apps(true);
    }
    let event_loop = builder.build().unwrap();
    check.inner.set_event_loop_proxy(event_loop.create_proxy());
    event_loop.run_app(&mut check).unwrap();
}
