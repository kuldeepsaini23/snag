mod choices;
mod format;
mod queues;
mod state;
mod ui;
mod update;

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
    let result = iced::application(move || update::boot(boot_manager.clone(), bridge_status.clone()), update::update, ui::view)
        .title("RDM")
        .subscription(update::subscription)
        .theme(iced::Theme::Dark)
        .window_size((1000.0, 700.0))
        .run();

    // Window closed: pause running downloads and save before exiting.
    runtime.block_on(manager.shutdown());
    result
}
