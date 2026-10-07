mod changelog;
mod choices;
mod format;
mod hsv;
mod motion;
mod queues;
mod report;
mod rules_form;
#[cfg(debug_assertions)]
mod snap;
mod notify;
mod state;
mod tray;
mod ui;
mod update;
mod view;

use std::path::PathBuf;

fn main() -> iced::Result {
    // The download manager runs on its own runtime, independent of the UI's.
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    // %APPDATA%\Snag (an older %APPDATA%\rdm is moved there once).
    let appdata = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_default();
    let state_path = rdm_core::store::data_dir(&appdata).join("state.json");
    // Only one RDM: a second copy would run the same downloads into the same files.
    let data_dir = state_path.parent().map(PathBuf::from).unwrap_or_default();
    // `rdm --quit`: ask the running RDM to pause, save and quit (used by the uninstaller).
    let quit = std::env::args().any(|a| a == "--quit");
    let Some(_instance) = rdm_core::instance::lock(&data_dir) else {
        let token = rdm_core::store::load(&state_path).settings.extension_token;
        if quit {
            rdm_bridge::quit_running(rdm_bridge::PORTS, &token);
        } else {
            rdm_bridge::focus_running(rdm_bridge::PORTS, &token);
        }
        return Ok(());
    };
    if quit {
        return Ok(()); // nothing running
    }
    let manager = {
        let _enter = runtime.enter();
        rdm_core::Manager::start(state_path)
    };
    let bridge_status = match runtime.block_on(rdm_bridge::start(manager.clone(), rdm_bridge::PORTS)) {
        Ok(b) => format!("Listening for the extension on 127.0.0.1:{}", b.port),
        Err(e) => format!("Extension bridge unavailable: {e}"),
    };

    let boot_manager = manager.clone();
    let boot_dir = data_dir.clone();
    let boot_runtime = runtime.handle().clone();
    let window = iced::window::Settings {
        size: iced::Size::new(1280.0, 800.0),
        min_size: Some(iced::Size::new(960.0, 600.0)),
        // The toolbar is the title bar (spec §2.5).
        decorations: false,
        icon: iced::window::icon::from_rgba(tray::ICON_64.to_vec(), 64, 64).ok(),
        ..Default::default()
    };
    let result = iced::application(move || update::boot(boot_manager.clone(), bridge_status.clone(), boot_dir.clone(), boot_runtime.clone()), update::update, ui::view)
        .title("Snag")
        .subscription(update::subscription)
        .theme(|app: &update::App| ui::theme::theme(&ui::theme::colors(app.model.accent_hex())))
        .font(ui::icon::INTER_REGULAR)
        .font(ui::icon::INTER_MEDIUM)
        .font(ui::icon::INTER_SEMIBOLD)
        .font(ui::icon::JETBRAINS_MONO)
        .font(ui::icon::PHOSPHOR)
        .font(ui::icon::PHOSPHOR_FILL)
        .font(ui::icon::PHOSPHOR_BOLD)
        .default_font(ui::style::INTER)
        // The Figma sizes are drawn for dense screens; at 100% Windows scaling they read small.
        .scale_factor(|_| ui::theme::UI_SCALE)
        .window(window)
        // Closing the window hides it to the tray; Quit is in the tray menu.
        .exit_on_close_request(false)
        .centered()
        .run();

    // Window closed: pause running downloads and save before exiting.
    runtime.block_on(manager.shutdown());
    result
}
