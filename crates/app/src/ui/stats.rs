//! The stats screen (sidebar → Stats): totals, data per day, share by type and the top sites,
//! for the last 7 or 30 days or everything. The numbers come from `stats.rs`.

use super::charts;
use super::icon::{Icon, bold};
use super::style;
use super::theme::Colors;
use super::{small, tiny};
use crate::format;
use crate::state::Model;
use crate::stats::{self, Range, Stats};
use crate::update::Message;
use iced::widget::text::Wrapping;
use iced::widget::{Space, button, column, container, progress_bar, row, scrollable, text};
use iced::{Alignment, Element, Fill, FillPortion};

/// `grow` (0 … 1): how far the charts have risen as the screen opened.
pub fn view(m: &Model, grow: f32, c: Colors) -> Element<'_, Message> {
    let s = stats::compute(&m.items, &m.daily, m.stats_range, chrono::Local::now().date_naive());
    let header = row![
        column![text("Stats").size(19).font(style::SEMIBOLD), small("What Snag downloaded: by day, by type and by site", c.text3)].spacing(3),
        Space::new().width(Fill),
        range_switch(m.stats_range, c),
    ]
    .align_y(Alignment::Center);

    let mut page = column![header].spacing(14);
    if s.total == 0 && s.files == 0 {
        page = page.push(empty(m.stats_range, c));
    } else {
        page = page.push(tiles(&s, m.stats_range, c));
        let per = if s.monthly { "Data per month" } else { "Data per day" };
        page = page.push(card(per, charts::bar_chart(s.bars.clone(), grow, 200.0, c), c));
        page = page.push(row![card("By type", by_type(&s, grow, c), c).width(FillPortion(1)), card("Top sites", top_sites(&s, c), c).width(FillPortion(1))].spacing(12));
    }
    let page = scrollable(page.padding(iced::Padding { right: 10.0, ..Default::default() })).style(style::scroll(c)).height(Fill);
    container(page).width(Fill).height(Fill).padding(18).style(style::panel(c)).into()
}

/// 7 days / 30 days / All.
fn range_switch<'a>(current: Range, c: Colors) -> Element<'a, Message> {
    let choices = Range::ALL.iter().fold(row![].spacing(2), |r, &range| {
        let on = range == current;
        let label = text(range.label()).size(12.5).font(if on { style::MEDIUM } else { style::INTER });
        r.push(button(label).style(style::choice(c, on, 6.0)).padding([5, 12]).on_press(Message::StatsRange(range)))
    });
    container(choices).padding(3).style(style::segmented(c)).into()
}

/// The four numbers along the top.
fn tiles<'a>(s: &Stats, range: Range, c: Colors) -> Element<'a, Message> {
    let tile = |glyph: Icon, label: &'static str, value: String, note: String| {
        let content = column![
            row![bold(glyph, 13).color(c.accent), tiny(label, c.text2)].spacing(6).align_y(Alignment::Center),
            text(value).size(20).font(style::SEMIBOLD).wrapping(Wrapping::None),
            tiny(note, c.text3).wrapping(Wrapping::None),
        ]
        .spacing(5);
        container(content).width(Fill).padding([12, 14]).clip(true).style(style::card(c))
    };
    let busiest = s.bars.iter().max_by_key(|b| b.bytes).filter(|b| b.bytes > 0);
    let when = if s.monthly { "Busiest month" } else { "Busiest day" };
    let span = match range {
        Range::Week => "last 7 days".to_string(),
        Range::Month => "last 30 days".to_string(),
        Range::All => s.bars.first().map(|b| format!("since {}", b.label)).unwrap_or_default(),
    };
    row![
        tile(Icon::Download, "Downloaded", format::size(s.total), span),
        tile(Icon::CheckCircle, "Files finished", s.files.to_string(), format!("{} type{}", s.categories.len(), if s.categories.len() == 1 { "" } else { "s" })),
        tile(Icon::Gauge, "Average speed", s.avg_bps.map(format::speed).unwrap_or_else(|| "—".into()), "while downloading".into()),
        tile(Icon::ChartBar, when, busiest.map(|b| format::size(b.bytes)).unwrap_or_else(|| "—".into()), busiest.map(|b| b.label.clone()).unwrap_or_default()),
    ]
    .spacing(12)
    .into()
}

fn card<'a>(title: &'static str, body: Element<'a, Message>, c: Colors) -> container::Container<'a, Message> {
    container(column![text(title).size(13).font(style::SEMIBOLD), body].spacing(12)).padding(14).width(Fill).style(style::card(c))
}

/// The donut with a legend: colour, type, share and size.
fn by_type<'a>(s: &Stats, grow: f32, c: Colors) -> Element<'a, Message> {
    if s.categories.is_empty() {
        return small("No finished files in this range.", c.text3).into();
    }
    let total: u64 = s.categories.iter().map(|(_, b)| b).sum();
    let slices = s.categories.iter().enumerate().map(|(k, (_, b))| (*b, charts::series(&c, k))).collect();
    let center = (format::size(total), format!("{} file{}", s.files, if s.files == 1 { "" } else { "s" }));
    let legend = s.categories.iter().enumerate().fold(column![].spacing(7), |col, (k, (cat, bytes))| {
        let swatch = container(Space::new()).width(9).height(9).style(style::tag(charts::series(&c, k), c.text));
        let share = *bytes as f64 * 100.0 / total.max(1) as f64;
        let share = if share > 0.0 && share < 0.5 { "<1%".to_string() } else { format!("{share:.0}%") };
        col.push(
            row![
                swatch,
                text(stats::category_label(*cat)).size(12.5).width(Fill),
                text(share).size(12).font(style::MONO).color(c.text2),
                text(format::size(*bytes)).size(11.5).color(c.text3).width(62).align_x(Alignment::End),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
    });
    row![charts::donut(slices, grow, center, 150.0, c), container(legend).width(Fill).center_y(150)].spacing(18).align_y(Alignment::Center).into()
}

/// The five sites most downloaded from, with a bar against the first.
fn top_sites<'a>(s: &Stats, c: Colors) -> Element<'a, Message> {
    if s.sites.is_empty() {
        return small("No finished files in this range.", c.text3).into();
    }
    let top = s.sites[0].bytes.max(1) as f32;
    s.sites
        .iter()
        .enumerate()
        .fold(column![].spacing(10), |col, (k, site)| {
            let line = row![
                text(format!("{}", k + 1)).size(11.5).font(style::MONO).color(c.text3).width(14),
                text(crate::view::ellipsize(&site.host, 34)).size(12.5).width(Fill).wrapping(Wrapping::None),
                tiny(format!("{} file{}", site.files, if site.files == 1 { "" } else { "s" }), c.text3),
                text(format::size(site.bytes)).size(12).font(style::MONO).color(c.text2).width(70).align_x(Alignment::End),
            ]
            .spacing(8)
            .align_y(Alignment::Center);
            let fill = if k == 0 { c.accent } else { charts::series(&c, 1) };
            let bar = row![Space::new().width(22), progress_bar(0.0..=top, site.bytes as f32).girth(4).style(style::progress(c.raised, fill))];
            col.push(column![line, bar].spacing(5))
        })
        .into()
}

fn empty<'a>(range: Range, c: Colors) -> Element<'a, Message> {
    let badge = container(bold(Icon::ChartBar, 24).color(c.accent)).center(52).style(style::tag(c.accent_soft, c.accent));
    let when = match range {
        Range::Week => "in the last 7 days",
        Range::Month => "in the last 30 days",
        Range::All => "yet",
    };
    let content = column![badge, text(format!("Nothing downloaded {when}")).size(16).font(style::SEMIBOLD), small("Finished downloads and the data they took show up here.", c.text3)]
        .spacing(10)
        .align_x(Alignment::Center);
    container(content).width(Fill).height(360).center(Fill).into()
}
