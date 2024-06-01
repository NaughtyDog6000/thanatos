use fontdue::layout::TextStyle;
use glam::{Vec2, Vec4};
use std::rc::Rc;

use crate::{
    clicked, right_clicked, Area, Constraint, Element, Event, Font, Rectangle, Scene, Signal,
    Signals,
};

pub struct Container<T: Element> {
    pub padding: f32,
    pub colour: Vec4,
    pub radius: f32,
    pub child: T,
}

impl<T: Element> Element for Container<T> {
    fn layout(&mut self, mut constraint: Constraint<Vec2>) -> Vec2 {
        constraint.max -= self.padding * 2.0;
        let size = self.child.layout(constraint);
        size + self.padding * 2.0
    }

    fn paint(
        &mut self,
        mut area: Area,
        scene: &mut Scene,
        events: &[Event],
        signals: &mut Signals,
    ) {
        scene.rectangle(Rectangle {
            area,
            colour: self.colour,
            radius: self.radius,
        });
        area.origin += self.padding;
        area.size -= self.padding * 2.0;
        self.child.paint(area, scene, events, signals);
    }
}

pub struct Text {
    pub text: String,
    pub font: Rc<Font>,
    pub font_size: f32,
    pub colour: Vec4,
}

impl Element for Text {
    fn layout(&mut self, _constraint: Constraint<Vec2>) -> Vec2 {
        let mut layout =
            fontdue::layout::Layout::new(fontdue::layout::CoordinateSystem::PositiveYDown);
        layout.append(
            &[self.font.clone()],
            &TextStyle::new(&self.text, self.font_size, 0),
        );
        let glyphs = layout.glyphs();
        let offset = glyphs
            .first()
            .map(|glyph| Vec2::new(glyph.x, glyph.y))
            .unwrap_or_default();
        let width = glyphs
            .iter()
            .map(|glyph| (glyph.x - offset.x) + glyph.width as f32)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or_default();
        let height = glyphs
            .iter()
            .map(|glyph| (glyph.y - offset.y) + glyph.height as f32)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or_default();

        Vec2::new(width, height)
    }

    fn paint(&mut self, area: Area, scene: &mut Scene, _: &[Event], _: &mut Signals) {
        scene.text(crate::Text {
            font: self.font.clone(),
            origin: area.origin,
            font_size: self.font_size,
            text: self.text.clone(),
            colour: self.colour,
        })
    }
}

pub fn text<T: ToString>(text: T, font_size: f32, font: Rc<Font>) -> Text {
    Text {
        text: text.to_string(),
        font_size,
        font,
        colour: Vec4::ONE,
    }
}

pub enum VAlign {
    Top,
    Center,
    Bottom,
}

pub struct VGroup {
    children: Vec<Box<dyn Element>>,
    alignment: VAlign,
    spacing: f32,
    sizes: Vec<Vec2>,
}

impl VGroup {
    pub fn new(alignment: VAlign, spacing: f32) -> Self {
        Self {
            children: Vec::new(),
            alignment,
            spacing,
            sizes: Vec::new(),
        }
    }

    pub fn add<T: Element + 'static>(mut self, child: T) -> Self {
        self.children.push(Box::new(child));
        self
    }
}

impl Element for VGroup {
    fn layout(&mut self, mut constraint: Constraint<Vec2>) -> Vec2 {
        self.sizes = self
            .children
            .iter_mut()
            .map(|child| {
                let size = child.layout(constraint);
                constraint.max.x -= size.x + self.spacing;
                size
            })
            .collect::<Vec<Vec2>>();
        let width = self.sizes.iter().map(|size| size.x).sum::<f32>()
            + (self.sizes.len() as f32 - 1.0) * self.spacing;
        let height = self
            .sizes
            .iter()
            .map(|size| size.y)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or_default();

        Vec2::new(width, height)
    }

    fn paint(&mut self, area: Area, scene: &mut Scene, events: &[Event], signals: &mut Signals) {
        let mut x = area.origin.x;
        self.children
            .iter_mut()
            .zip(&self.sizes)
            .for_each(|(child, &size)| {
                let y = match self.alignment {
                    VAlign::Top => area.origin.y,
                    VAlign::Center => area.origin.y + (area.size.y - size.y) / 2.0,
                    VAlign::Bottom => area.origin.y + (area.size.y - size.y),
                };

                let area = Area {
                    origin: Vec2::new(x, y),
                    size,
                };
                child.paint(area, scene, events, signals);
                x += size.x + self.spacing;
            });
    }
}

pub enum HAlign {
    Left,
    Center,
    Right,
}

pub struct HGroup {
    children: Vec<Box<dyn Element>>,
    alignment: HAlign,
    spacing: f32,
    sizes: Vec<Vec2>,
}

impl HGroup {
    pub fn new(alignment: HAlign, spacing: f32) -> Self {
        Self {
            children: Vec::new(),
            alignment,
            spacing,
            sizes: Vec::new(),
        }
    }

    pub fn add<T: Element + 'static>(mut self, child: T) -> Self {
        self.children.push(Box::new(child));
        self
    }
}

impl Element for HGroup {
    fn layout(&mut self, mut constraint: Constraint<Vec2>) -> Vec2 {
        self.sizes = self
            .children
            .iter_mut()
            .map(|child| {
                let size = child.layout(constraint);
                constraint.max.y -= size.y + self.spacing;
                size
            })
            .collect::<Vec<Vec2>>();
        let height = self.sizes.iter().map(|size| size.y).sum::<f32>()
            + (self.sizes.len() as f32 - 1.0) * self.spacing;
        let width = self
            .sizes
            .iter()
            .map(|size| size.x)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or_default();

        Vec2::new(width, height)
    }

    fn paint(&mut self, area: Area, scene: &mut Scene, events: &[Event], signals: &mut Signals) {
        let mut y = area.origin.y;
        self.children
            .iter_mut()
            .zip(&self.sizes)
            .for_each(|(child, &size)| {
                let x = match self.alignment {
                    HAlign::Left => area.origin.x,
                    HAlign::Center => area.origin.x + (area.size.x - size.x) / 2.0,
                    HAlign::Right => area.origin.x + (area.size.x - size.x),
                };

                let area = Area {
                    origin: Vec2::new(x, y),
                    size,
                };
                child.paint(area, scene, events, signals);
                y += size.y + self.spacing;
            });
    }
}

pub struct Offset<T: Element> {
    pub offset: Vec2,
    pub child: T,
}

impl<T: Element> Element for Offset<T> {
    fn layout(&mut self, mut constraint: Constraint<Vec2>) -> Vec2 {
        constraint.max -= self.offset;
        self.child.layout(constraint) + self.offset
    }

    fn paint(
        &mut self,
        mut area: Area,
        scene: &mut Scene,
        events: &[Event],
        signals: &mut Signals,
    ) {
        area.origin += self.offset;
        area.size -= self.offset;
        self.child.paint(area, scene, events, signals);
    }
}

pub struct Clicked<T: Element> {
    pub signal: Signal,
    pub child: T,
}

impl<T: Element> Element for Clicked<T> {
    fn layout(&mut self, constraint: Constraint<Vec2>) -> Vec2 {
        self.child.layout(constraint)
    }

    fn paint(&mut self, area: Area, scene: &mut Scene, events: &[Event], signals: &mut Signals) {
        if clicked(events, area) {
            signals.set(self.signal)
        }
        self.child.paint(area, scene, events, signals)
    }
}

pub struct RightClicked<T: Element> {
    pub signal: Signal,
    pub child: T,
}

impl<T: Element> Element for RightClicked<T> {
    fn layout(&mut self, constraint: Constraint<Vec2>) -> Vec2 {
        self.child.layout(constraint)
    }

    fn paint(&mut self, area: Area, scene: &mut Scene, events: &[Event], signals: &mut Signals) {
        if right_clicked(events, area) {
            signals.set(self.signal)
        }
        self.child.paint(area, scene, events, signals)
    }
}

pub enum Gap {
    Auto,
}

pub struct VPair<A: Element, B: Element> {
    pub left: A,
    pub right: B,
    pub sizes: (Vec2, Vec2),
    pub gap: Gap,
}

impl<A: Element, B: Element> VPair<A, B> {
    pub fn new(left: A, right: B, gap: Gap) -> Self {
        Self {
            left,
            right,
            gap,
            sizes: (Vec2::ZERO, Vec2::ZERO),
        }
    }
}

impl<A: Element, B: Element> Element for VPair<A, B> {
    fn layout(&mut self, constraint: Constraint<Vec2>) -> Vec2 {
        let left = self.left.layout(constraint);
        let right = self.right.layout(Constraint {
            min: constraint.min,
            max: Vec2::new(constraint.max.x - left.x, constraint.max.y),
        });
        self.sizes = (left, right);

        Vec2::new(constraint.max.x, left.y)
    }

    fn paint(&mut self, area: Area, scene: &mut Scene, events: &[Event], signals: &mut Signals) {
        self.left.paint(
            Area {
                origin: area.origin,
                size: self.sizes.0,
            },
            scene,
            events,
            signals,
        );
        self.right.paint(
            Area {
                origin: area
                    .origin
                    .with_x(area.origin.x + area.size.x - self.sizes.1.x),
                size: self.sizes.1,
            },
            scene,
            events,
            signals,
        );
    }
}

pub struct Constrain<T: Element> {
    pub child: T,
    pub constraint: Constraint<Vec2>,
}

impl<T: Element> Element for Constrain<T> {
    fn layout(&mut self, _: Constraint<Vec2>) -> Vec2 {
        self.child.layout(self.constraint)
    }

    fn paint(&mut self, area: Area, scene: &mut Scene, events: &[Event], signals: &mut Signals) {
        self.child.paint(area, scene, events, signals)
    }
}
