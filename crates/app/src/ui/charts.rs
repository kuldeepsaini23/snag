//! Canvas charts: the speed sparkline, the segment map, the stats bars and donut, and the
//! status ring on grid cards. Everything is drawn in the theme's colours (the accent and its
//! tints); nothing is cached because each is a handful of shapes, drawn only when the view is.

use super::style;
use super::theme::Colors;
use crate::format;
use crate::speed_history::WINDOW;
use crate::stats::Bar;
use crate::update::Message;
use iced::alignment::{Horizontal, Vertical};
use iced::widget::canvas::{self, Frame, Geometry, LineCap, Path, Stroke, Text, gradient, path::Arc};
use iced::{Color, Element, Length, Point, Radians, Rectangle, Renderer, Size, Theme, mouse};
use rdm_engine::segments::Segment;
use std::f32::consts::{FRAC_PI_2, TAU};

/// Colour `i` of a chart's series: the accent first, then its deeper and lighter tints, then greys.
pub fn series(c: &Colors, i: usize) -> Color {
    let mix = |a: Color, b: Color, t: f32| Color::from_rgba(a.r + (b.r - a.r) * t, a.g + (b.g - a.g) * t, a.b + (b.b - a.b) * t, c.alpha);
    let text = Color { a: 1.0, ..c.text };
    match i {
        0 => mix(c.accent, c.accent, 0.0),
        1 => mix(c.accent, c.panel, 0.45),
        2 => mix(c.accent, text, 0.5),
        3 => mix(c.accent, c.panel, 0.7),
        4 => mix(c.panel, text, 0.55),
        _ => mix(c.panel, text, 0.3),
    }
}

fn canvas<'a, P: canvas::Program<Message> + 'a>(program: P, width: impl Into<Length>, height: impl Into<Length>) -> Element<'a, Message> {
    canvas::Canvas::new(program).width(width).height(height).into()
}

fn label(content: String, at: Point, size: f32, color: Color, align_x: Horizontal, align_y: Vertical) -> Text {
    Text { content, position: at, color, size: size.into(), font: style::INTER, align_x: align_x.into(), align_y, ..Text::default() }
}

// ---------- sparkline ----------

/// The line's points in a `w`×`h` box: one per second, left (oldest) to right (now), scaled so the
/// peak touches the top (a flat zero line lies on the bottom edge).
pub fn spark_points(values: &[u64], w: f32, h: f32) -> Vec<Point> {
    let peak = values.iter().copied().max().unwrap_or(0).max(1) as f32;
    let step = w / (values.len().max(2) - 1) as f32;
    values.iter().enumerate().map(|(k, v)| Point::new(k as f32 * step, h - (*v as f32 / peak) * h)).collect()
}

/// The last minute of speed: an accent line over a soft fill.
pub fn sparkline<'a>(values: [u64; WINDOW], width: impl Into<Length>, height: f32, c: Colors) -> Element<'a, Message> {
    canvas(Sparkline { values, c }, width, height)
}

struct Sparkline {
    values: [u64; WINDOW],
    c: Colors,
}

impl canvas::Program<Message> for Sparkline {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        // Inset by half the line width, so the peak isn't clipped.
        let (w, h) = (bounds.width, bounds.height - 1.5);
        let points: Vec<Point> = spark_points(&self.values, w, h).into_iter().map(|p| Point::new(p.x, p.y + 0.75)).collect();
        let line = Path::new(|b| {
            b.move_to(points[0]);
            points[1..].iter().for_each(|p| b.line_to(*p));
        });
        let area = Path::new(|b| {
            b.move_to(Point::new(0.0, bounds.height));
            points.iter().for_each(|p| b.line_to(*p));
            b.line_to(Point::new(w, bounds.height));
            b.close();
        });
        let soft = gradient::Linear::new(Point::ORIGIN, Point::new(0.0, bounds.height))
            .add_stop(0.0, Color { a: 0.28 * self.c.alpha, ..self.c.accent })
            .add_stop(1.0, Color { a: 0.0, ..self.c.accent });
        frame.fill(&area, soft);
        frame.stroke(&line, Stroke::default().with_width(1.5).with_color(self.c.accent).with_line_cap(LineCap::Round));
        vec![frame.into_geometry()]
    }
}

// ---------- segment map ----------

/// Where each segment's bar goes in a strip `width` wide: (x, width, part done 0 … 1). Bars are as
/// wide as their share of the file, `gap` apart, never thinner than 2 px.
pub fn segment_bars(segments: &[Segment], width: f32, gap: f32) -> Vec<(f32, f32, f32)> {
    let size = segments.iter().map(|s| s.end).max().unwrap_or(0).max(1) as f32;
    segments
        .iter()
        .map(|s| {
            let x = s.start as f32 / size * width;
            let w = ((s.end - s.start) as f32 / size * width - gap).max(2.0);
            let done = ((s.written + s.inflight) as f32 / (s.end - s.start).max(1) as f32).clamp(0.0, 1.0);
            (x, w, done)
        })
        .collect()
}

/// One bar per connection, placed over its range of the file and filled as far as it got.
pub fn segment_map<'a>(segments: Vec<Segment>, height: f32, c: Colors) -> Element<'a, Message> {
    let mut sorted = segments;
    sorted.sort_by_key(|s| s.start);
    canvas(SegmentMap { segments: sorted, c }, Length::Fill, height)
}

struct SegmentMap {
    segments: Vec<Segment>,
    c: Colors,
}

impl canvas::Program<Message> for SegmentMap {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let h = bounds.height;
        for (x, w, done) in segment_bars(&self.segments, bounds.width + 3.0, 3.0) {
            let radius = 4.0f32.min(w / 2.0);
            frame.fill(&Path::rounded_rectangle(Point::new(x, 0.0), Size::new(w, h), radius.into()), self.c.raised);
            if done > 0.0 {
                frame.fill(&Path::rounded_rectangle(Point::new(x, 0.0), Size::new((w * done).max(2.0), h), radius.into()), self.c.accent);
            }
        }
        vec![frame.into_geometry()]
    }
}

// ---------- stats: bars ----------

/// The bar chart: `grow` (0 … 1) is how far the bars have risen as the screen opens.
pub fn bar_chart<'a>(bars: Vec<Bar>, grow: f32, height: f32, c: Colors) -> Element<'a, Message> {
    canvas(BarChart { bars, grow, c }, Length::Fill, height)
}

/// Which bars get an x label: all of a week, every 5th of a month (and the last), at most ~12.
pub fn x_labels(n: usize) -> Vec<usize> {
    let every = if n <= 12 { 1 } else { n.div_ceil(7) };
    let mut out: Vec<usize> = (0..n).rev().step_by(every).collect();
    out.reverse();
    out
}

struct BarChart {
    bars: Vec<Bar>,
    grow: f32,
    c: Colors,
}

impl canvas::Program<Message> for BarChart {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, cursor: mouse::Cursor) -> Vec<Geometry> {
        let c = self.c;
        let mut frame = Frame::new(renderer, bounds.size());
        let (top, bottom, left) = (18.0, 20.0, 0.0);
        let plot = Rectangle::new(Point::new(left, top), Size::new(bounds.width - left, bounds.height - top - bottom));
        let peak = self.bars.iter().map(|b| b.bytes).max().unwrap_or(0);
        // Guides at the peak and half of it, labelled on the left.
        for (k, f) in [1.0f32, 0.5].into_iter().enumerate() {
            let y = plot.y + plot.height * (1.0 - f);
            frame.fill_rectangle(Point::new(plot.x, y), Size::new(plot.width, 1.0), c.line);
            if peak > 0 {
                let value = if k == 0 { format::size(peak) } else { format::size(peak / 2) };
                frame.fill_text(label(value, Point::new(plot.x, y - 3.0), 10.5, c.text3, Horizontal::Left, Vertical::Bottom));
            }
        }
        frame.fill_rectangle(Point::new(plot.x, plot.y + plot.height), Size::new(plot.width, 1.0), c.line_strong);

        let n = self.bars.len().max(1);
        let slot = plot.width / n as f32;
        let bar_w = (slot * 0.62).clamp(2.0, 34.0);
        let hovered = cursor.position_in(bounds).map(|p| ((p.x - plot.x) / slot).floor() as usize);
        let labelled = x_labels(self.bars.len());
        for (k, b) in self.bars.iter().enumerate() {
            let cx = plot.x + slot * (k as f32 + 0.5);
            let h = if peak == 0 { 0.0 } else { plot.height * (b.bytes as f32 / peak as f32) * self.grow };
            let is_hot = hovered == Some(k);
            let fill = if is_hot || b.bytes == peak { c.accent } else { series(&c, 1) };
            if b.bytes > 0 {
                let h = h.max(2.0);
                let radius = 3.0f32.min(bar_w / 2.0).min(h / 2.0);
                frame.fill(&Path::rounded_rectangle(Point::new(cx - bar_w / 2.0, plot.y + plot.height - h), Size::new(bar_w, h), radius.into()), fill);
            }
            // Values: every bar of a short chart, else the hovered one.
            if b.bytes > 0 && (self.bars.len() <= 12 || is_hot) && self.grow >= 1.0 {
                let at = Point::new(cx, plot.y + plot.height - h - 3.0);
                frame.fill_text(label(format::size(b.bytes), at, 10.5, if is_hot { c.text } else { c.text2 }, Horizontal::Center, Vertical::Bottom));
            }
            if labelled.contains(&k) {
                let at = Point::new(cx, plot.y + plot.height + 6.0);
                frame.fill_text(label(b.label.clone(), at, 10.5, if is_hot { c.text } else { c.text3 }, Horizontal::Center, Vertical::Top));
            }
        }
        vec![frame.into_geometry()]
    }

    fn mouse_interaction(&self, _: &(), bounds: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
        if cursor.is_over(bounds) { mouse::Interaction::Crosshair } else { mouse::Interaction::default() }
    }

    fn update(&self, _: &mut (), event: &canvas::Event, bounds: Rectangle, cursor: mouse::Cursor) -> Option<canvas::Action<Message>> {
        // Moving over the bars shows the value under the mouse: redraw on moves inside.
        match event {
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) if cursor.is_over(bounds) => Some(canvas::Action::request_redraw()),
            canvas::Event::Mouse(mouse::Event::CursorLeft) => Some(canvas::Action::request_redraw()),
            _ => None,
        }
    }
}

// ---------- stats: donut ----------

/// The angle each slice spans (radians, clockwise from the top), from their values.
pub fn donut_angles(values: &[u64]) -> Vec<(f32, f32)> {
    let total = values.iter().sum::<u64>().max(1) as f32;
    let mut at = -FRAC_PI_2;
    values
        .iter()
        .map(|v| {
            let span = *v as f32 / total * TAU;
            let slice = (at, at + span);
            at += span;
            slice
        })
        .collect()
}

/// The share donut; `sweep` (0 … 1) draws it round as the screen opens. `center` is the big
/// number and its caption in the hole.
pub fn donut<'a>(slices: Vec<(u64, Color)>, sweep: f32, center: (String, String), size: f32, c: Colors) -> Element<'a, Message> {
    canvas(Donut { slices, sweep, center, c }, size, size)
}

struct Donut {
    slices: Vec<(u64, Color)>,
    sweep: f32,
    center: (String, String),
    c: Colors,
}

impl canvas::Program<Message> for Donut {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let center = frame.center();
        let thick = 16.0;
        let radius = bounds.width.min(bounds.height) / 2.0 - thick / 2.0 - 1.0;
        let ring = |from: f32, to: f32| Path::new(|b| b.arc(Arc { center, radius, start_angle: Radians(from), end_angle: Radians(to) }));
        frame.stroke(&Path::circle(center, radius), Stroke::default().with_width(thick).with_color(self.c.raised));
        let values: Vec<u64> = self.slices.iter().map(|(v, _)| *v).collect();
        let limit = -FRAC_PI_2 + TAU * self.sweep;
        // A hairline of the card between slices keeps neighbouring tints apart.
        let gap = if self.slices.len() > 1 { 0.025 } else { 0.0 };
        for ((from, to), (_, color)) in donut_angles(&values).into_iter().zip(&self.slices) {
            let to = to.min(limit) - gap;
            if to > from {
                frame.stroke(&ring(from, to), Stroke::default().with_width(thick).with_color(*color));
            }
        }
        let (big, small) = &self.center;
        frame.fill_text(Text { font: style::SEMIBOLD, ..label(big.clone(), Point::new(center.x, center.y + 2.0), 17.0, self.c.text, Horizontal::Center, Vertical::Bottom) });
        frame.fill_text(label(small.clone(), Point::new(center.x, center.y + 4.0), 10.5, self.c.text3, Horizontal::Center, Vertical::Top));
        vec![frame.into_geometry()]
    }
}

// ---------- grid: status ring ----------

/// A small ring: `done` (0 … 1) of it in `color` over a faint track.
pub fn ring<'a>(done: f32, color: Color, track: Color, size: f32) -> Element<'a, Message> {
    canvas(Ring { done, color, track }, size, size)
}

struct Ring {
    done: f32,
    color: Color,
    track: Color,
}

impl canvas::Program<Message> for Ring {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let center = frame.center();
        let width = 2.5;
        let radius = bounds.width.min(bounds.height) / 2.0 - width / 2.0 - 0.5;
        frame.stroke(&Path::circle(center, radius), Stroke::default().with_width(width).with_color(self.track));
        if self.done > 0.0 {
            let end = -FRAC_PI_2 + TAU * self.done.min(1.0);
            let arc = Path::new(|b| b.arc(Arc { center, radius, start_angle: Radians(-FRAC_PI_2), end_angle: Radians(end) }));
            frame.stroke(&arc, Stroke::default().with_width(width).with_color(self.color).with_line_cap(LineCap::Round));
        }
        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spark_points_scale_to_the_peak() {
        let p = spark_points(&[0, 5, 10], 100.0, 20.0);
        assert_eq!(p, vec![Point::new(0.0, 20.0), Point::new(50.0, 10.0), Point::new(100.0, 0.0)]);
        let flat = spark_points(&[0, 0, 0], 100.0, 20.0);
        assert!(flat.iter().all(|p| p.y == 20.0), "nothing downloading: a line on the floor");
    }

    #[test]
    fn segment_bars_follow_ranges_and_progress() {
        let mut segs = vec![Segment::new(0, 50), Segment::new(50, 100)];
        segs[0].written = 25;
        segs[1].written = 50;
        let bars = segment_bars(&segs, 103.0, 3.0);
        assert_eq!(bars.len(), 2);
        assert!((bars[0].0 - 0.0).abs() < 1e-3 && (bars[0].1 - 48.5).abs() < 1e-3 && (bars[0].2 - 0.5).abs() < 1e-6, "{bars:?}");
        assert!((bars[1].0 - 51.5).abs() < 1e-3 && (bars[1].2 - 1.0).abs() < 1e-6, "{bars:?}");
        // A sliver split off a big segment still shows.
        let tiny = segment_bars(&[Segment::new(0, 1000), Segment::new(1000, 1001)], 100.0, 3.0);
        assert_eq!(tiny[1].1, 2.0);
    }

    #[test]
    fn donut_slices_cover_the_circle_from_the_top() {
        let a = donut_angles(&[1, 3]);
        assert!((a[0].0 + FRAC_PI_2).abs() < 1e-6, "starts at 12 o'clock");
        assert!((a[0].1 - a[0].0 - TAU / 4.0).abs() < 1e-5);
        assert!((a[1].1 - (TAU - FRAC_PI_2)).abs() < 1e-5, "ends where it started");
        assert!(donut_angles(&[]).is_empty());
    }

    #[test]
    fn x_labels_thin_out_on_long_charts() {
        assert_eq!(x_labels(7), (0..7).collect::<Vec<_>>());
        let month = x_labels(30);
        assert_eq!(month.last(), Some(&29), "today is always labelled");
        assert!(month.len() <= 8, "{month:?}");
        assert_eq!(x_labels(0), Vec::<usize>::new());
    }

    #[test]
    fn series_starts_with_the_accent_and_differs() {
        let c = super::super::theme::colors("#0a84ff");
        assert_eq!(series(&c, 0), c.accent);
        let all: Vec<Color> = (0..6).map(|i| series(&c, i)).collect();
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                assert_ne!(a, b);
            }
        }
        assert_ne!(series(&super::super::theme::colors("#ff9f0a"), 1), series(&c, 1), "tints follow the accent");
    }
}
