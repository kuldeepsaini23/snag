//! The speed-limit popover under the gauge button (Figma frame 05).

use super::style;
use super::theme::Colors;
use super::small;
use crate::state::Model;
use crate::update::Message;
use crate::view;
use iced::widget::{Space, button, column, container, row, slider, text, toggler};
use iced::{Alignment, Element, Fill};

const KB: u64 = 1024;
const PRESETS: [(u64, &str); 4] = [(512 * KB, "512 KB/s"), (1024 * KB, "1 MB/s"), (2048 * KB, "2 MB/s"), (5120 * KB, "5 MB/s")];

pub fn view(m: &Model, c: Colors) -> Element<'_, Message> {
    let shown = m.shown_limit();
    let on = shown > 0;
    let (number, unit) = view::speed_parts(shown);
    let (running, _) = m.totals();

    let head = row![
        text("Limit download speed").size(13).font(style::SEMIBOLD).width(Fill),
        toggler(on).on_toggle(Message::SpeedOn).size(18).style(style::toggle(c)),
    ]
    .align_y(Alignment::Center);
    let big = row![text(number).size(28).font(style::MEDIUM), text(unit).size(13).color(c.text2)].spacing(6).align_y(Alignment::End);
    let rail = slider(0.0..=1.0, if on { view::bps_to_slider(shown) } else { 0.0 }, Message::SpeedSlide)
        .on_release(Message::SpeedRelease)
        .step(0.001_f32)
        .style(style::speed_slider(c));

    let chip = |label: &'static str, bps: u64| {
        let chosen = shown == bps;
        button(text(label).size(11.5).color(if chosen { c.accent } else { c.text2 }))
            .padding([4, 9])
            .style(move |t, s| {
                let mut b = style::outline(c)(t, s);
                if chosen {
                    b.background = Some(c.accent_soft.into());
                    b.border.color = c.accent;
                }
                b.border.radius = 20.0.into();
                b
            })
            .on_press(Message::QuickLimit(bps))
    };
    let first = PRESETS.iter().fold(row![].spacing(6), |r, &(bps, label)| r.push(chip(label, bps)));
    let presets = column![first, chip("Unlimited", 0)].spacing(6);

    let note = match running {
        0 => "Applies to every download.".to_string(),
        1 => "Applies to the 1 active download.".to_string(),
        n => format!("Applies to all {n} active downloads."),
    };
    let content = column![head, big, rail, presets, Space::new().height(2), small(note, c.text3)].spacing(12);
    container(content).width(290).padding(16).style(style::sheet(c)).into()
}
