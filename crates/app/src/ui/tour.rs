//! The first-run tour (`tour.rs` holds its state): a 5-step sheet, then coach marks that point
//! at toolbar buttons.

use super::icon::{Icon, bold, icon};
use super::style;
use super::theme::{self, Colors, SWATCHES};
use super::{small, tiny};
use crate::state::Model;
use crate::tour::{MARKS, Mark, SHEET, Step};
use crate::update::Message;
use iced::widget::canvas::{self, Frame, Geometry, Path};
use iced::widget::{Space, button, column, container, float, row, text, text_input, toggler};
use iced::{Alignment, Color, Element, Fill, Padding, Point, Rectangle, Renderer, Theme, Vector, mouse};
use rdm_core::ThemeMode;

const SHEET_WIDTH: f32 = 540.0;
const BUBBLE_WIDTH: f32 = 290.0;
/// The bubble's arrow: its size, and how far its tip sits from the bubble's right edge.
const ARROW: (f32, f32) = (18.0, 9.0);
const ARROW_RIGHT: f32 = 34.0;
/// How far each coach mark's button centre is from the window's right edge: the caption buttons
/// (3 × 46), the toolbar's 12 px padding, then the toolbar's buttons right to left with 8 px
/// between them (+ Add ~69, "?" 32, the gear 32, the gauge 32).
const ADD_CENTRE: f32 = 138.0 + 12.0 + 34.5;
const HELP_CENTRE: f32 = 138.0 + 12.0 + 69.0 + 8.0 + 16.0;
const SPEED_CENTRE: f32 = HELP_CENTRE + 2.0 * (32.0 + 8.0);
/// Below the 52 px toolbar.
const BUBBLE_TOP: f32 = 46.0;

/// The extension pages to open, per browser (they can't be opened from outside the browser, so
/// they're copied).
const BROWSERS: [(&str, &str); 4] = [
    ("Chrome", "chrome://extensions"),
    ("Edge", "edge://extensions"),
    ("Brave", "brave://extensions"),
    ("Firefox", "about:debugging#/runtime/this-firefox"),
];

pub fn sheet<'a>(m: &'a Model, step: Step, c: Colors) -> Element<'a, Message> {
    let at = step.sheet_index().unwrap_or(0);
    let dots = SHEET.iter().enumerate().fold(row![].spacing(5).align_y(Alignment::Center), |r, (k, _)| {
        let (width, fill) = match k.cmp(&at) {
            std::cmp::Ordering::Equal => (18.0, c.accent),
            std::cmp::Ordering::Less => (6.0, Color { a: 0.5 * c.accent.a, ..c.accent }),
            std::cmp::Ordering::Greater => (6.0, c.line_strong),
        };
        r.push(container(Space::new()).width(width).height(6).style(style::tag(fill, fill)))
    });
    let skip = button(text("Skip tour").size(12)).style(style::ghost(c)).padding([4, 8]).on_press(Message::SkipTour);
    let top = row![dots, Space::new().width(Fill), tiny(format!("Step {} of {}", at + 1, SHEET.len()), c.text3), skip].spacing(10).align_y(Alignment::Center);

    let body = match step {
        Step::Welcome => welcome(c),
        Step::Folder => folder(m, c),
        Step::Look => look(m, c),
        Step::Extension => extension(m, c),
        _ => clipboard(m, c),
    };
    let mut content = column![top, body].spacing(18);
    if let Some(e) = &m.settings_error {
        content = content.push(row![icon(Icon::WarningCircle, 14).color(c.danger), text(e).size(12.5)].spacing(8).align_y(Alignment::Center));
    }
    content = content.push(footer(step, c));
    container(content).width(SHEET_WIDTH).padding([20, 24]).style(style::sheet(c)).into()
}

/// Back on the left; Next (or the step's own words) on the right.
fn footer<'a>(step: Step, c: Colors) -> Element<'a, Message> {
    let label = |s: &'static str| text(s).size(12.5).font(style::SEMIBOLD);
    let mut bar = row![].spacing(8).align_y(Alignment::Center);
    if step.back().is_some() {
        bar = bar.push(button(label("Back")).style(style::secondary(c)).padding([8, 14]).on_press(Message::TourBack));
    }
    bar = bar.push(Space::new().width(Fill));
    match step {
        Step::Welcome => bar = bar.push(button(label("Get started")).style(style::accent(c)).padding([8, 18]).on_press(Message::TourNext)),
        Step::Clipboard => {
            bar = bar.push(button(label("Finish")).style(style::secondary(c)).padding([8, 14]).on_press(Message::TourNext));
            let try_it = row![bold(Icon::Link, 13), label("Try a link")].spacing(6).align_y(Alignment::Center);
            bar = bar.push(button(try_it).style(style::accent(c)).padding([8, 16]).on_press(Message::TryLink));
        }
        _ => bar = bar.push(button(label("Next")).style(style::accent(c)).padding([8, 18]).on_press(Message::TourNext)),
    }
    bar.into()
}

/// A step's icon, title and one line under it.
fn heading<'a>(glyph: Icon, title: &'a str, line: &'a str, c: Colors) -> Element<'a, Message> {
    let badge = container(icon(glyph, 22).color(c.accent)).center(44).style(style::tag(c.accent_soft, c.accent));
    row![badge, column![text(title).size(17).font(style::SEMIBOLD), text(line).size(12.5).color(c.text2)].spacing(4).width(Fill)]
        .spacing(14)
        .align_y(Alignment::Center)
        .into()
}

/// One row in a card: icon, title and description.
fn point<'a>(glyph: Icon, title: &'a str, desc: &'a str, c: Colors) -> Element<'a, Message> {
    row![bold(glyph, 16).color(c.accent), column![text(title).size(13).font(style::MEDIUM), small(desc, c.text3)].spacing(2).width(Fill)]
        .spacing(12)
        .align_y(Alignment::Center)
        .padding([10, 14])
        .into()
}

fn card<'a>(rows: Vec<Element<'a, Message>>, c: Colors) -> Element<'a, Message> {
    container(column(rows)).width(Fill).style(style::card(c)).into()
}

fn welcome<'a>(c: Colors) -> Element<'a, Message> {
    let logo = iced::widget::image(super::icon::logo(c.accent)).width(52).height(52).opacity(c.alpha);
    let hero = row![
        logo,
        column![
            text("Welcome to Snag").size(20).font(style::SEMIBOLD),
            text("Fast, resumable downloads: files, videos, music and torrents. Four quick choices and you're set; skip any time.").size(12.5).color(c.text2),
        ]
        .spacing(4)
        .width(Fill),
    ]
    .spacing(16)
    .align_y(Alignment::Center);
    let points = card(
        vec![
            point(Icon::Link, "Paste a link, or just copy one", "Snag spots copied links and offers them in the corner", c),
            point(Icon::FilmStrip, "Videos and music from 1,000+ sites", "Pick the quality, an MP3, or a whole playlist", c),
            point(Icon::ArrowClockwise, "Pause, resume, schedule", "Downloads continue where they stopped, even after a restart", c),
        ],
        c,
    );
    column![hero, points].spacing(18).into()
}

fn folder<'a>(m: &'a Model, c: Colors) -> Element<'a, Message> {
    let d = &m.draft;
    let field = text_input(r"C:\Users\you\Downloads\Snag", &d.download_dir)
        .on_input(Message::DraftDir)
        .on_submit(Message::TourNext)
        .size(12.5)
        .padding([8, 10])
        .style(style::input(c));
    let sort = row![
        column![text("Sort into category folders").size(13).font(style::MEDIUM), small("Videos, Music, Images, Archives, Documents and Programs each get a folder inside it", c.text3)]
            .spacing(2)
            .width(Fill),
        toggler(d.sort_into_folders).on_toggle(Message::DraftSort).size(18).style(style::toggle(c)),
    ]
    .spacing(16)
    .align_y(Alignment::Center)
    .padding([10, 14]);
    column![
        heading(Icon::FolderOpen, "Where should downloads go?", "Change it any time in Settings → General.", c),
        column![text("Download folder").size(13).font(style::MEDIUM), field].spacing(6),
        card(vec![sort.into()], c),
    ]
    .spacing(16)
    .into()
}

fn look<'a>(m: &'a Model, c: Colors) -> Element<'a, Message> {
    let modes = [(ThemeMode::Dark, "Dark"), (ThemeMode::Light, "Light"), (ThemeMode::System, "Follow Windows")];
    let segments = modes.iter().fold(row![].spacing(2), |r, &(mode, label)| {
        let on = m.draft.theme == mode;
        r.push(
            button(container(text(label).size(12.5).font(if on { style::SEMIBOLD } else { style::INTER })).center_x(Fill))
                .width(Fill)
                .padding([6, 0])
                .style(style::choice(c, on, 6.0))
                .on_press(Message::DraftTheme(mode)),
        )
    });
    let current = theme::parse_hex(m.accent_hex());
    let swatches = SWATCHES.iter().fold(row![].spacing(10).align_y(Alignment::Center), |r, (_, hex)| {
        let color = theme::parse_hex(hex).unwrap_or(Color::WHITE);
        r.push(button(Space::new()).width(26).height(26).padding(0).style(style::swatch(color, current == Some(color), c.ink)).on_press(Message::DraftAccent((*hex).to_string())))
    });
    column![
        heading(Icon::Palette, "Make it yours", "Pick a theme and an accent colour: the window changes as you choose.", c),
        column![text("Theme").size(13).font(style::MEDIUM), container(segments).padding(3).style(style::segmented(c))].spacing(8),
        column![text("Accent colour").size(13).font(style::MEDIUM), swatches, small("Any colour at all: Settings → Appearance.", c.text3)].spacing(8),
    ]
    .spacing(18)
    .into()
}

fn extension<'a>(m: &'a Model, c: Colors) -> Element<'a, Message> {
    let number = |n: &'static str| container(text(n).size(12).font(style::SEMIBOLD).color(c.accent)).center(24).style(style::tag(c.accent_soft, c.accent));
    let step = |n: &'static str, title: &'static str, desc: &'static str| -> Element<'a, Message> {
        row![number(n), column![text(title).size(13).font(style::MEDIUM), small(desc, c.text3)].spacing(2).width(Fill)]
            .spacing(12)
            .align_y(Alignment::Center)
            .padding([10, 14])
            .into()
    };
    let pages = BROWSERS.iter().fold(row![].spacing(6), |r, &(name, page)| {
        let label = row![bold(Icon::Copy, 11), text(name).size(11.5)].spacing(5).align_y(Alignment::Center);
        r.push(button(label).style(style::outline(c)).padding([4, 9]).on_press(Message::CopyText(page.to_string())))
    });
    let get = column![
        step("1", "Add the Snag extension to your browser", "On the browser's extensions page: Developer mode, then Load unpacked and pick Snag's extension folder."),
        row![Space::new().width(36), column![small("Copy the extensions page's address, then paste it into the address bar:", c.text3), pages].spacing(6)]
            .padding(Padding { bottom: 10.0, right: 14.0, ..Default::default() }),
    ];
    let listening = m.bridge_status.starts_with("Listening");
    let status = row![
        container(Space::new()).width(7).height(7).style(style::tag(if listening { c.success } else { c.danger }, c.text)),
        small(if listening { "Snag is ready for the extension" } else { "The extension can't reach Snag right now" }, if listening { c.success } else { c.danger }),
    ]
    .spacing(7)
    .align_y(Alignment::Center);
    column![
        heading(Icon::Browser, "Catch downloads from your browser", "The extension sends downloads and videos from Chrome, Edge, Brave or Firefox straight here.", c),
        card(
            vec![
                get.into(),
                step("2", "Click Connect in the extension", "Open the Snag button in the browser's toolbar and press Connect."),
                step("3", "Click Allow here", "Snag asks once. There's no code to copy."),
            ],
            c,
        ),
        status,
    ]
    .spacing(16)
    .into()
}

fn clipboard<'a>(m: &'a Model, c: Colors) -> Element<'a, Message> {
    let watch = row![
        column![text("Watch the clipboard for links").size(13).font(style::MEDIUM), small("Copy a link anywhere and it pops up in the corner, ready to download", c.text3)]
            .spacing(2)
            .width(Fill),
        toggler(m.draft.clipboard_watch).on_toggle(Message::DraftClipboard).size(18).style(style::toggle(c)),
    ]
    .spacing(16)
    .align_y(Alignment::Center)
    .padding([10, 14]);
    column![
        heading(Icon::ClipboardText, "Copy a link, get a download", "The quickest way in: no pasting needed.", c),
        card(vec![watch.into()], c),
        small("Try a link puts a small test file (1 MB) in the link bar. Then click + Add to download it.", c.text2),
    ]
    .spacing(16)
    .into()
}

/// The tooltip under a toolbar button: what it does, then Next (or Done) and Skip.
pub fn coach<'a>(m: &Model, mark: Mark, c: Colors) -> Element<'a, Message> {
    let (title, body) = match mark {
        Mark::Add if m.url == crate::tour::SAMPLE_LINK => ("Your test link is ready", "Click + Add to download it. Any link you paste or type goes the same way."),
        Mark::Add => ("Add a download", "Paste a link in the bar and click + Add. Copied links pop up by themselves."),
        Mark::Speed => ("Limit the speed", "Cap Snag's bandwidth so browsing and calls stay smooth, with presets for one click."),
        Mark::Help => ("Help is here", "Keyboard shortcuts, help, what's new and Report a bug. F1 opens help any time."),
    };
    let at = MARKS.iter().position(|k| *k == mark).unwrap_or(0);
    let last = at + 1 == MARKS.len();
    let label = |s: &'static str| text(s).size(12).font(style::SEMIBOLD);
    let buttons = row![
        tiny(format!("{} of {}", at + 1, MARKS.len()), c.text3),
        Space::new().width(Fill),
        button(text("Skip").size(12)).style(style::ghost(c)).padding([5, 9]).on_press(Message::SkipTour),
        button(label(if last { "Done" } else { "Next" })).style(style::accent(c)).padding([5, 12]).on_press(Message::TourNext),
    ]
    .spacing(6)
    .align_y(Alignment::Center);
    let bubble = container(column![text(title).size(13.5).font(style::SEMIBOLD), small(body, c.text2), Space::new().height(2), buttons].spacing(6))
        .width(BUBBLE_WIDTH)
        .padding([12, 14])
        .style(style::sheet(c));
    // Drawn over the bubble's top edge, so its outline joins the bubble's and hides the edge below it.
    let arrow = float(canvas::Canvas::new(Arrow { fill: c.panel, line: c.line_strong }).width(ARROW.0).height(ARROW.1)).translate(|_, _| Vector::new(0.0, 1.5));
    let pointer = row![Space::new().width(Fill), arrow, Space::new().width(ARROW_RIGHT - ARROW.0 / 2.0)].width(BUBBLE_WIDTH);
    column![pointer, bubble].into()
}

/// Puts a coach mark's arrow under its button (the bubble is right-aligned in the window).
pub fn coach_padding(mark: Mark) -> Padding {
    let centre = match mark {
        Mark::Add => ADD_CENTRE,
        Mark::Speed => SPEED_CENTRE,
        Mark::Help => HELP_CENTRE,
    };
    Padding { top: BUBBLE_TOP, right: centre - ARROW_RIGHT, ..Default::default() }
}

/// The bubble's upward arrow: a filled triangle with the bubble's outline on its two sides.
struct Arrow {
    fill: Color,
    line: Color,
}

impl canvas::Program<Message> for Arrow {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let (w, h) = (bounds.width, bounds.height);
        let tip = Path::new(|p| {
            p.move_to(Point::new(0.0, h + 1.0));
            p.line_to(Point::new(w / 2.0, 0.5));
            p.line_to(Point::new(w, h + 1.0));
        });
        frame.fill(&tip, self.fill);
        frame.stroke(&tip, canvas::Stroke::default().with_width(1.0).with_color(self.line));
        vec![frame.into_geometry()]
    }
}
