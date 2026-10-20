//! Application shell: `AppKit` lifecycle, menus, window, and the fixed-size
//! 224:256 content area.

use std::cell::OnceCell;

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{DefinedClass, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate, NSBackingStoreType,
    NSColor, NSMenu, NSMenuItem, NSResponder, NSWindow, NSWindowDelegate, NSWindowStyleMask,
};
use objc2_foundation::{
    MainThreadMarker, NSNotification, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize,
    ns_string,
};

use crate::preferences;
use crate::view::GameView;

const WINDOW_WIDTH: f64 = 672.0;
const WINDOW_HEIGHT: f64 = 768.0;

#[derive(Default)]
struct AppDelegateIvars {
    window: OnceCell<Retained<NSWindow>>,
    view: OnceCell<Retained<GameView>>,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements and this delegate lives
    // for the duration of the application run loop.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = AppDelegateIvars]
    struct AppDelegate;

    // SAFETY: NSObjectProtocol has no safety requirements.
    unsafe impl NSObjectProtocol for AppDelegate {}

    // SAFETY: NSApplicationDelegate has no safety requirements.
    unsafe impl NSApplicationDelegate for AppDelegate {
        #[unsafe(method(applicationDidFinishLaunching:))]
        fn did_finish_launching(&self, notification: &NSNotification) {
            let mtm = self.mtm();
            // SAFETY: AppKit passes the shared application as the notification object.
            let app = notification
                .object()
                .and_then(|object| object.downcast::<NSApplication>().ok())
                .unwrap_or_else(|| NSApplication::sharedApplication(mtm));
            install_menu(&app, mtm);

            // SAFETY: window creation happens on the main thread and the window
            // is retained by this delegate for the app's lifetime.
            let window = unsafe {
                NSWindow::initWithContentRect_styleMask_backing_defer(
                    NSWindow::alloc(mtm),
                    NSRect::new(
                        NSPoint::new(0.0, 0.0),
                        NSSize::new(WINDOW_WIDTH, WINDOW_HEIGHT),
                    ),
                    NSWindowStyleMask::Titled
                        | NSWindowStyleMask::Closable
                        | NSWindowStyleMask::Miniaturizable
                        | NSWindowStyleMask::Resizable,
                    NSBackingStoreType::Buffered,
                    false,
                )
            };
            // SAFETY: disabling auto-release is required for windows created
            // outside a window controller.
            unsafe { window.setReleasedWhenClosed(false) };
            window.setTitle(ns_string!("SPACE INVADERS — Metal Arcade"));
            window.setContentMinSize(NSSize::new(WINDOW_WIDTH / 2.0, WINDOW_HEIGHT / 2.0));
            window.setContentAspectRatio(NSSize::new(WINDOW_WIDTH, WINDOW_HEIGHT));
            window.setBackgroundColor(Some(&NSColor::blackColor()));
            window.setDelegate(Some(ProtocolObject::from_ref(self)));

            if let Some(view) = self.ivars().view.get() {
                window.setContentView(Some(view));
                let responder: &NSResponder = view;
                let _ = window.makeFirstResponder(Some(responder));
            }

            window.center();
            window.makeKeyAndOrderFront(None);
            app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
            #[allow(deprecated)]
            app.activateIgnoringOtherApps(true);

            let _ = self.ivars().window.set(window);
        }

        #[unsafe(method(applicationShouldTerminateAfterLastWindowClosed:))]
        fn should_terminate_after_last_window_closed(&self, _sender: &NSApplication) -> bool {
            true
        }

        #[unsafe(method(applicationWillTerminate:))]
        fn will_terminate(&self, _notification: &NSNotification) {}
    }

    // SAFETY: NSWindowDelegate has no safety requirements.
    unsafe impl NSWindowDelegate for AppDelegate {
        #[unsafe(method(windowWillClose:))]
        fn window_will_close(&self, _notification: &NSNotification) {
            NSApplication::sharedApplication(self.mtm()).terminate(None);
        }
    }
);

impl AppDelegate {
    fn new(mtm: MainThreadMarker, view: Retained<GameView>) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(AppDelegateIvars::default());
        // SAFETY: NSObject's init has no further requirements.
        let this: Retained<Self> = unsafe { msg_send![super(this), init] };
        let _ = this.ivars().view.set(view);
        this
    }
}

/// Starts the `AppKit` event loop. This function does not return until the app
/// quits.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mtm = MainThreadMarker::new().ok_or("Space Invaders must start on the main thread")?;
    let high_score = preferences::load_high_score();
    let muted = preferences::load_muted();
    let view = GameView::new(mtm, high_score, muted).map_err(std::io::Error::other)?;
    let app = NSApplication::sharedApplication(mtm);
    let delegate = AppDelegate::new(mtm, view);
    app.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
    app.run();
    Ok(())
}

fn install_menu(app: &NSApplication, mtm: MainThreadMarker) {
    let main_menu = NSMenu::new(mtm);

    let app_item = NSMenuItem::new(mtm);
    app_item.setTitle(ns_string!("SPACE INVADERS"));
    let app_menu = NSMenu::new(mtm);
    app_menu.setTitle(ns_string!("SPACE INVADERS"));
    // SAFETY: these selectors are implemented by NSApplication.
    unsafe {
        app_menu.addItemWithTitle_action_keyEquivalent(
            ns_string!("About SPACE INVADERS"),
            Some(sel!(orderFrontStandardAboutPanel:)),
            ns_string!(""),
        );
        app_menu.addItem(&NSMenuItem::separatorItem(mtm));
        app_menu.addItemWithTitle_action_keyEquivalent(
            ns_string!("Hide SPACE INVADERS"),
            Some(sel!(hide:)),
            ns_string!("h"),
        );
        let hide_others = app_menu.addItemWithTitle_action_keyEquivalent(
            ns_string!("Hide Others"),
            Some(sel!(hideOtherApplications:)),
            ns_string!("h"),
        );
        hide_others.setKeyEquivalentModifierMask(
            objc2_app_kit::NSEventModifierFlags::Command
                | objc2_app_kit::NSEventModifierFlags::Option,
        );
        app_menu.addItemWithTitle_action_keyEquivalent(
            ns_string!("Show All"),
            Some(sel!(unhideAllApplications:)),
            ns_string!(""),
        );
        app_menu.addItem(&NSMenuItem::separatorItem(mtm));
        let quit = app_menu.addItemWithTitle_action_keyEquivalent(
            ns_string!("Quit SPACE INVADERS"),
            Some(sel!(terminate:)),
            ns_string!("q"),
        );
        quit.setKeyEquivalentModifierMask(objc2_app_kit::NSEventModifierFlags::Command);
    }
    app_item.setSubmenu(Some(&app_menu));
    main_menu.addItem(&app_item);

    let game_item = NSMenuItem::new(mtm);
    game_item.setTitle(ns_string!("Game"));
    let game_menu = NSMenu::new(mtm);
    game_menu.setTitle(ns_string!("Game"));
    // SAFETY: these selectors are implemented by GameView in the responder chain.
    unsafe {
        game_menu.addItemWithTitle_action_keyEquivalent(
            ns_string!("New Game"),
            Some(sel!(newGame:)),
            ns_string!("n"),
        );
        game_menu.addItemWithTitle_action_keyEquivalent(
            ns_string!("Pause / Resume"),
            Some(sel!(togglePause:)),
            ns_string!("p"),
        );
        game_menu.addItemWithTitle_action_keyEquivalent(
            ns_string!("Mute / Unmute"),
            Some(sel!(toggleMute:)),
            ns_string!("m"),
        );
    }
    game_item.setSubmenu(Some(&game_menu));
    main_menu.addItem(&game_item);

    let view_item = NSMenuItem::new(mtm);
    view_item.setTitle(ns_string!("View"));
    let view_menu = NSMenu::new(mtm);
    view_menu.setTitle(ns_string!("View"));
    // SAFETY: AppKit implements toggleFullScreen: on NSWindow.
    unsafe {
        let full_screen = view_menu.addItemWithTitle_action_keyEquivalent(
            ns_string!("Enter Full Screen"),
            Some(sel!(toggleFullScreen:)),
            ns_string!("f"),
        );
        full_screen.setKeyEquivalentModifierMask(
            objc2_app_kit::NSEventModifierFlags::Command
                | objc2_app_kit::NSEventModifierFlags::Control,
        );
    }
    view_item.setSubmenu(Some(&view_menu));
    main_menu.addItem(&view_item);

    app.setMainMenu(Some(&main_menu));
}
