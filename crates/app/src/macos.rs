//! macOS's application delegate (winit leaves that slot to the app): a click on the Dock icon
//! shows the window again, and Quit (⌘Q, the Dock, logging out) pauses and saves downloads before
//! the app ends, which `terminate:` would otherwise skip.

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSApplication, NSApplicationDelegate, NSApplicationTerminateReply};
use objc2_foundation::{NSObject, NSObjectProtocol};
use std::sync::OnceLock;

/// Run when the app is asked to quit: pauses downloads and saves (blocking).
static ON_QUIT: OnceLock<Box<dyn Fn() + Send + Sync>> = OnceLock::new();

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "SnagAppDelegate"]
    struct Delegate;

    unsafe impl NSObjectProtocol for Delegate {}

    unsafe impl NSApplicationDelegate for Delegate {
        /// The Dock icon was clicked: bring the hidden window back, as the tray's Open Snag does.
        #[unsafe(method(applicationShouldHandleReopen:hasVisibleWindows:))]
        fn should_handle_reopen(&self, _app: &NSApplication, _visible: bool) -> bool {
            crate::tray::open_window();
            true
        }

        #[unsafe(method(applicationShouldTerminate:))]
        fn should_terminate(&self, _app: &NSApplication) -> NSApplicationTerminateReply {
            if let Some(quit) = ON_QUIT.get() {
                quit();
            }
            NSApplicationTerminateReply::TerminateNow
        }
    }
);

impl Delegate {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        // SAFETY: NSObject's plain `init` on a freshly allocated instance.
        unsafe { msg_send![Self::alloc(mtm), init] }
    }
}

/// Becomes the application's delegate. Main thread only, once the app is running (it's made
/// with the menu bar icon); `on_quit` runs before the app ends.
pub fn install(on_quit: impl Fn() + Send + Sync + 'static) {
    let Some(mtm) = MainThreadMarker::new() else { return };
    if ON_QUIT.set(Box::new(on_quit)).is_err() {
        return; // already installed
    }
    let delegate = Delegate::new(mtm);
    NSApplication::sharedApplication(mtm).setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
    // The application holds its delegate weakly: this one lives as long as the app.
    std::mem::forget(delegate);
}
