//! The custom `NSView` that owns the window's game loop, input, and Metal layer.
//!
//! `AppKit` calls `tick:` sixty times a second. Each callback advances the
//! headless simulation with a bounded fixed-step accumulator, dispatches audio
//! cues, and draws a frame. Keyboard handling lives here too, translating key
//! codes into simulation commands rather than mutating game state directly.

use std::cell::RefCell;
use std::ptr::NonNull;
use std::time::{Duration, Instant};

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{DefinedClass, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSAutoresizingMaskOptions, NSEvent, NSEventModifierFlags, NSView};
use objc2_foundation::{MainThreadMarker, NSObjectProtocol, NSPoint, NSRect, NSSize, NSTimer};
use objc2_metal::{MTLCreateSystemDefaultDevice, MTLPixelFormat};
use objc2_quartz_core::{CALayer, CAMetalLayer};

use space_invaders_core::{Command, Game, GameEvent, Phase};

use crate::audio::{AudioEngine, SoundCue};
use crate::preferences;
use crate::renderer::Renderer;
use crate::scene;

const TICK_SECONDS: f64 = 1.0 / 60.0;
const MAX_CATCH_UP_TICKS: u32 = 8;
const DESIGN_WIDTH: f64 = 672.0;
const DESIGN_HEIGHT: f64 = 768.0;

#[derive(Default)]
pub struct GameViewIvars {
    state: RefCell<Option<GameViewState>>,
}

struct GameViewState {
    game: Game,
    renderer: Renderer,
    audio: Option<AudioEngine>,
    layer: Retained<CAMetalLayer>,
    last_frame: Instant,
    accumulator: f64,
    muted: bool,
    high_score: u32,
    march_step: u8,
    last_error_log: Option<Instant>,
}

define_class!(
    // SAFETY: NSView has no extra subclassing requirements here, the class is
    // main-thread-only, and the ivars contain no borrowed data.
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[ivars = GameViewIvars]
    pub struct GameView;

    // SAFETY: NSObjectProtocol has no safety requirements.
    unsafe impl NSObjectProtocol for GameView {}

    impl GameView {
        #[unsafe(method(acceptsFirstResponder))]
        fn accepts_first_responder(&self) -> bool {
            true
        }

        #[unsafe(method(keyDown:))]
        fn key_down(&self, event: &NSEvent) {
            self.handle_key(event, true);
        }

        #[unsafe(method(keyUp:))]
        fn key_up(&self, event: &NSEvent) {
            self.handle_key(event, false);
        }

        #[unsafe(method(newGame:))]
        fn menu_new_game(&self, _sender: Option<&AnyObject>) {
            self.start_new_game();
        }

        #[unsafe(method(togglePause:))]
        fn menu_toggle_pause(&self, _sender: Option<&AnyObject>) {
            self.with_game(|game| game.command(Command::TogglePause));
        }

        #[unsafe(method(toggleMute:))]
        fn menu_toggle_mute(&self, _sender: Option<&AnyObject>) {
            self.toggle_mute();
        }
    }
);

impl GameView {
    pub fn new(
        mtm: MainThreadMarker,
        high_score: u32,
        muted: bool,
    ) -> Result<Retained<Self>, String> {
        let this = Self::alloc(mtm).set_ivars(GameViewIvars::default());
        let frame = NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(DESIGN_WIDTH, DESIGN_HEIGHT),
        );
        // SAFETY: `NSView`'s designated initializer accepts this frame.
        let this: Retained<Self> = unsafe { msg_send![super(this), initWithFrame: frame] };

        let device = MTLCreateSystemDefaultDevice()
            .ok_or_else(|| "This Mac does not expose a Metal device".to_string())?;

        let layer = CAMetalLayer::new();
        layer.setDevice(Some(&device));
        layer.setPixelFormat(MTLPixelFormat::BGRA8Unorm);
        layer.setFramebufferOnly(true);
        layer.setMaximumDrawableCount(3);
        layer.setContentsScale(2.0);

        this.setWantsLayer(true);
        let layer_base: &CALayer = &layer;
        this.setLayer(Some(layer_base));
        this.setAutoresizingMask(
            NSAutoresizingMaskOptions::ViewWidthSizable
                | NSAutoresizingMaskOptions::ViewHeightSizable,
        );

        let renderer = Renderer::new(device, layer.clone()).map_err(|error| error.to_string())?;
        let audio = AudioEngine::new().ok();
        if let Some(engine) = &audio {
            engine.set_muted(muted);
        }

        let state = GameViewState {
            game: Game::new(high_score),
            renderer,
            audio,
            layer,
            last_frame: Instant::now(),
            accumulator: 0.0,
            muted,
            high_score,
            march_step: 0,
            last_error_log: None,
        };
        *this.ivars().state.borrow_mut() = Some(state);
        this.sync_drawable_size();

        let timer_view = this.clone();
        let block = RcBlock::new(move |_timer: NonNull<NSTimer>| {
            timer_view.frame_tick();
        });
        // SAFETY: the block has the signature NSTimer expects and the timer is
        // retained by the current run loop. The cycle is intentional for the
        // life of the application, which exits when the window closes.
        unsafe {
            NSTimer::scheduledTimerWithTimeInterval_repeats_block(1.0 / 60.0, true, &block);
        }

        Ok(this)
    }

    fn with_game(&self, action: impl FnOnce(&mut Game)) {
        let mut borrow = self.ivars().state.borrow_mut();
        if let Some(state) = borrow.as_mut() {
            action(&mut state.game);
        }
    }

    fn start_new_game(&self) {
        let mut borrow = self.ivars().state.borrow_mut();
        let Some(state) = borrow.as_mut() else {
            return;
        };
        state.game = Game::new(state.high_score);
        state.game.command(Command::Start);
        state.march_step = 0;
    }

    fn toggle_mute(&self) {
        let mut borrow = self.ivars().state.borrow_mut();
        let Some(state) = borrow.as_mut() else {
            return;
        };
        state.muted = !state.muted;
        if let Some(audio) = &state.audio {
            audio.set_muted(state.muted);
        }
        preferences::save_muted(state.muted);
    }

    fn handle_key(&self, event: &NSEvent, pressed: bool) {
        let key = event.keyCode();
        let command = event
            .modifierFlags()
            .contains(NSEventModifierFlags::Command);

        if command {
            if !pressed {
                return;
            }
            match key {
                45 => self.start_new_game(),                                     // Cmd+N
                35 => self.with_game(|game| game.command(Command::TogglePause)), // Cmd+P
                46 => self.toggle_mute(),                                        // Cmd+M
                _ => {}
            }
            return;
        }

        let mut borrow = self.ivars().state.borrow_mut();
        let Some(state) = borrow.as_mut() else {
            return;
        };
        match key {
            123 | 0 => state.game.command(Command::SetLeft(pressed)), // left arrow / A
            124 | 2 => state.game.command(Command::SetRight(pressed)), // right arrow / D
            49 if pressed && !event.isARepeat() => match state.game.phase() {
                Phase::Attract | Phase::GameOver => state.game.command(Command::Start),
                _ => state.game.command(Command::Fire),
            },
            36 | 76 if pressed => {
                if matches!(state.game.phase(), Phase::Attract | Phase::GameOver) {
                    state.game.command(Command::Start);
                }
            }
            35 | 53 if pressed => state.game.command(Command::TogglePause), // P / Esc
            46 if pressed => {
                drop(borrow);
                self.toggle_mute();
            }
            _ => {}
        }
    }

    fn frame_tick(&self) {
        let mut borrow = self.ivars().state.borrow_mut();
        let Some(state) = borrow.as_mut() else {
            return;
        };

        let now = Instant::now();
        let elapsed = now.duration_since(state.last_frame).as_secs_f64().min(0.25);
        state.last_frame = now;
        state.accumulator = (state.accumulator + elapsed).min(0.25);

        let mut ticks = 0;
        while state.accumulator >= TICK_SECONDS && ticks < MAX_CATCH_UP_TICKS {
            state.game.tick();
            let events = state.game.take_events();
            for event in events {
                dispatch_event(state, event);
            }
            state.accumulator -= TICK_SECONDS;
            ticks += 1;
        }
        if ticks == MAX_CATCH_UP_TICKS {
            // Avoid a spiral of death after a long stall (window drag, debugger).
            state.accumulator = 0.0;
        }

        if state.game.high_score() > state.high_score {
            state.high_score = state.game.high_score();
            preferences::save_high_score(state.high_score);
        }

        let (width, height) = self.sync_drawable_size_for(state);
        let canvas = scene::build(&state.game, state.muted);
        if let Err(error) = state.renderer.draw(&canvas, width, height) {
            // A draw failure is recoverable; report at most once per second
            // rather than flooding stderr at 60 Hz.
            let should_log = state
                .last_error_log
                .is_none_or(|last| now.duration_since(last) >= Duration::from_secs(1));
            if should_log {
                eprintln!("SpaceInvaders render error: {error}");
                state.last_error_log = Some(now);
            }
        }
    }

    fn sync_drawable_size(&self) {
        let mut borrow = self.ivars().state.borrow_mut();
        if let Some(state) = borrow.as_mut() {
            self.sync_drawable_size_for(state);
        }
    }

    fn sync_drawable_size_for(&self, state: &GameViewState) -> (usize, usize) {
        let bounds = self.bounds();
        let scale = self
            .window()
            .map_or(2.0, |window| window.backingScaleFactor());
        state.layer.setContentsScale(scale);
        let width = (bounds.size.width * scale).round().max(1.0) as usize;
        let height = (bounds.size.height * scale).round().max(1.0) as usize;
        state
            .layer
            .setDrawableSize(NSSize::new(width as f64, height as f64));
        (width, height)
    }
}

fn dispatch_event(state: &mut GameViewState, event: GameEvent) {
    if state.muted {
        return;
    }
    let Some(audio) = &state.audio else {
        return;
    };
    match event {
        GameEvent::AlienStep => {
            let step = state.march_step;
            state.march_step = (state.march_step + 1) % 4;
            audio.play(SoundCue::March(step));
        }
        GameEvent::PlayerShoot => audio.play(SoundCue::PlayerShoot),
        GameEvent::EnemyShoot => audio.play(SoundCue::EnemyShoot),
        GameEvent::InvaderKilled { .. } => audio.play(SoundCue::InvaderDeath),
        GameEvent::PlayerHit => audio.play(SoundCue::PlayerDeath),
        GameEvent::MysteryAppeared => audio.play(SoundCue::MysteryShip),
        GameEvent::MysteryKilled { .. } => audio.play(SoundCue::MysteryDeath),
        GameEvent::ExtraLife => audio.play(SoundCue::ExtraLife),
        GameEvent::WaveCleared => audio.play(SoundCue::WaveClear),
        GameEvent::GameOver => audio.play(SoundCue::GameOver),
        GameEvent::GameStarted | GameEvent::WaveStarted => {}
    }
}
