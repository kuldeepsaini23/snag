//! The Figma views. Logic lives in `state.rs` / `view.rs` / `update.rs`.

pub mod icon;
pub mod theme;

use crate::update::{App, Message};
use iced::Element;

pub fn view(app: &App) -> Element<'_, Message> {
    iced::widget::text(app.model.items.len().to_string()).into()
}
