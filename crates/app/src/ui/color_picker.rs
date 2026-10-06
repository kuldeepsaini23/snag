//! The accent picker in Settings → Appearance: a saturation/value square and a hue strip, drawn
//! from the same conversion the hex box uses (`hsv.rs`), so what's under a cursor is the hex.

use super::theme::Colors;
use crate::hsv::Hsv;
use crate::update::Message;
use iced::widget::canvas::{self, Action, Cache, Event, Frame, Geometry, Path, Stroke, fill};
use iced::{Color, Element, Point, Rectangle, Renderer, Size, Theme, mouse};
use std::cell::Cell;

/// Each drawn cell, in logical pixels: small enough that the steps can't be seen.
const CELL: f32 = 3.0;
const RADIUS: f32 = 8.0;
/// The hue strip's bar inside its taller canvas (the cursor overhangs it).
const BAR: f32 = 10.0;
/// How far in from the canvas edge the colours start (the knob overhangs them by this much).
const INSET: f32 = 8.0;

/// The square: saturation left → right, value bottom → top, at the current hue.
pub fn square<'a>(hsv: Hsv, width: f32, height: f32, backdrop: Color, c: Colors) -> Element<'a, Message> {
    canvas(Picker { kind: Kind::Square, hsv, backdrop, c }).width(width).height(height).into()
}

/// The hue strip, left (red) → right (red again).
pub fn hue_strip<'a>(hsv: Hsv, width: f32, height: f32, backdrop: Color, c: Colors) -> Element<'a, Message> {
    canvas(Picker { kind: Kind::Hue, hsv, backdrop, c }).width(width).height(height).into()
}

fn canvas<P: canvas::Program<Message>>(program: P) -> canvas::Canvas<P, Message> {
    canvas::Canvas::new(program)
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Square,
    Hue,
}

struct Picker {
    kind: Kind,
    hsv: Hsv,
    /// The card behind it (its rounded corners are painted over in this colour).
    backdrop: Color,
    c: Colors,
}

#[derive(Default)]
pub struct State {
    dragging: bool,
    /// The colour cells, redrawn only when the hue (square), the opacity or the size changes.
    cells: Cache,
    /// (hue, opacity) the cells were drawn at.
    drawn: Cell<Option<(f32, f32)>>,
}

impl Picker {
    /// Where the colours are drawn in a canvas of `size`: inset by the knob's radius, so the knob
    /// at an edge isn't cut off.
    fn area(&self, size: Size) -> Rectangle {
        let width = (size.width - 2.0 * INSET).max(1.0);
        match self.kind {
            Kind::Square => Rectangle::new(Point::new(INSET, INSET), Size::new(width, (size.height - 2.0 * INSET).max(1.0))),
            Kind::Hue => Rectangle::new(Point::new(INSET, (size.height - BAR) / 2.0), Size::new(width, BAR)),
        }
    }

    /// The message for a point in the canvas (outside the colours: clamped to their edge, so a
    /// drag can overshoot and still reach the extremes).
    fn pick(&self, p: Point, size: Size) -> Message {
        let a = self.area(size);
        let x = ((p.x - a.x) / a.width).clamp(0.0, 1.0);
        let y = ((p.y - a.y) / a.height).clamp(0.0, 1.0);
        match self.kind {
            Kind::Square => Message::AccentSv(x, 1.0 - y),
            // 360° would fold back to red at the left edge.
            Kind::Hue => Message::AccentHue((x * 360.0).min(359.9)),
        }
    }

    fn color(&self, hsv: Hsv) -> Color {
        let [r, g, b] = hsv.to_rgb();
        Color { a: self.c.alpha, ..Color::from_rgb8(r, g, b) }
    }
}

impl canvas::Program<Message> for Picker {
    type State = State;

    fn update(&self, state: &mut State, event: &Event, bounds: Rectangle, cursor: mouse::Cursor) -> Option<Action<Message>> {
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let p = cursor.position_in(bounds)?;
                state.dragging = true;
                Some(Action::publish(self.pick(p, bounds.size())).and_capture())
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) if state.dragging => {
                let p = Point::new(position.x - bounds.x, position.y - bounds.y);
                Some(Action::publish(self.pick(p, bounds.size())).and_capture())
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if state.dragging => {
                state.dragging = false;
                Some(Action::capture())
            }
            _ => None,
        }
    }

    fn draw(&self, state: &State, renderer: &Renderer, _theme: &Theme, bounds: Rectangle, _cursor: mouse::Cursor) -> Vec<Geometry> {
        let (size, a) = (bounds.size(), self.area(bounds.size()));
        // The strip's colours don't depend on the hue picked; the square's do.
        let key = Some((if self.kind == Kind::Square { self.hsv.h } else { 0.0 }, self.c.alpha));
        if state.drawn.get() != key {
            state.cells.clear();
            state.drawn.set(key);
        }
        let cells = state.cells.draw(renderer, size, |frame| self.draw_cells(frame, a));
        let mut top = Frame::new(renderer, size);
        let (at, fill) = match self.kind {
            Kind::Square => (Point::new(a.x + self.hsv.s * a.width, a.y + (1.0 - self.hsv.v) * a.height), self.color(self.hsv)),
            Kind::Hue => (Point::new(a.x + self.hsv.h / 360.0 * a.width, a.center_y()), self.color(Hsv::new(self.hsv.h, 1.0, 1.0))),
        };
        let knob = Path::circle(at, 6.0);
        top.fill(&knob, fill);
        top.stroke(&Path::circle(at, 7.5), Stroke::default().with_width(1.0).with_color(self.c.fixed(Color::from_rgba(0.0, 0.0, 0.0, 0.45))));
        top.stroke(&knob, Stroke::default().with_width(2.0).with_color(self.c.fixed(Color::WHITE)));
        vec![cells, top.into_geometry()]
    }

    fn mouse_interaction(&self, state: &State, bounds: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
        if state.dragging {
            mouse::Interaction::Grabbing
        } else if cursor.is_over(bounds) {
            if self.kind == Kind::Square { mouse::Interaction::Crosshair } else { mouse::Interaction::Pointer }
        } else {
            mouse::Interaction::default()
        }
    }
}

impl Picker {
    /// The colours as small cells, each the colour at its centre; the square fades saturation
    /// left → right and value top → bottom, the strip runs through the hues.
    fn draw_cells(&self, frame: &mut Frame, a: Rectangle) {
        let cols = (a.width / CELL).ceil().max(1.0) as usize;
        let rows = if self.kind == Kind::Square { (a.height / CELL).ceil().max(1.0) as usize } else { 1 };
        let (w, h) = (a.width / cols as f32, a.height / rows as f32);
        for i in 0..cols {
            let x = (i as f32 + 0.5) / cols as f32;
            for j in 0..rows {
                let hsv = match self.kind {
                    Kind::Square => Hsv::new(self.hsv.h, x, 1.0 - (j as f32 + 0.5) / rows as f32),
                    Kind::Hue => Hsv::new((x * 360.0).min(359.9), 1.0, 1.0),
                };
                // A hair wider than the cell, so no seam shows between neighbours.
                frame.fill_rectangle(Point::new(a.x + i as f32 * w, a.y + j as f32 * h), Size::new(w + 0.5, h + 0.5), self.color(hsv));
            }
        }
        let radius = if self.kind == Kind::Square { RADIUS } else { BAR / 2.0 };
        // Paints the corners outside the rounded shape in the card's colour (and the hair of
        // overlap past the right and bottom edges).
        let mask = Path::new(|p| {
            p.rectangle(a.position(), Size::new(a.width + 1.0, a.height + 1.0));
            p.rounded_rectangle(a.position(), a.size(), radius.into());
        });
        frame.fill(&mask, canvas::Fill { style: canvas::Style::Solid(self.backdrop), rule: fill::Rule::EvenOdd });
    }
}
