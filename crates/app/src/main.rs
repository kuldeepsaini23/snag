mod choices;
mod format;
mod queues;
#[cfg(debug_assertions)]
mod snap;
mod state;
mod ui;
mod update;
mod view;

use std::path::PathBuf;

fn main() -> iced::Result {
    // The download manager runs on its own runtime, independent of the UI's.
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    let state_path = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_default().join("rdm").join("state.json");
    // Only one RDM: a second copy would run the same downloads into the same files.
    let data_dir = state_path.parent().map(PathBuf::from).unwrap_or_default();
    let Some(_instance) = rdm_core::instance::lock(&data_dir) else {
        let token = rdm_core::store::load(&state_path).settings.extension_token;
        rdm_bridge::focus_running(rdm_bridge::PORTS, &token);
        return Ok(());
    };
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
    let window = iced::window::Settings {
        size: iced::Size::new(1280.0, 800.0),
        min_size: Some(iced::Size::new(960.0, 600.0)),
        // The toolbar is the title bar (spec §2.5).
        decorations: false,
        ..Default::default()
    };
    let result = iced::application(move || update::boot(boot_manager.clone(), bridge_status.clone(), boot_dir.clone()), update::update, ui::view)
        .title("RDM")
        .subscription(update::subscription)
        .theme(|app: &update::App| ui::theme::theme(&ui::theme::colors(app.model.accent_hex())))
        .font(ui::icon::INTER_REGULAR)
        .font(ui::icon::INTER_MEDIUM)
        .font(ui::icon::INTER_SEMIBOLD)
        .font(ui::icon::JETBRAINS_MONO)
        .font(ui::icon::PHOSPHOR)
        .font(ui::icon::PHOSPHOR_FILL)
        .default_font(ui::style::INTER)
        // The Figma sizes are drawn for dense screens; at 100% Windows scaling they read small.
        .scale_factor(|_| ui::theme::UI_SCALE)
        .window(window)
        .centered()
        .run();

    // Window closed: pause running downloads and save before exiting.
    runtime.block_on(manager.shutdown());
    result
}
