use fontdue::layout::TextStyle;
use glam::{Vec2, Vec4};
use std::rc::Rc;

use crate::{Area, Constraint, Element, Font, Rectangle, Scene};

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

    fn paint(&mut self, mut area: Area, scene: &mut Scene) {
        scene.rectangle(Rectangle {
            area,
            colour: self.colour,
            radius: self.radius,
        });
        area.origin += self.padding;
        area.size -= self.padding * 2.0;
        self.child.paint(area, scene);
    }
}

pub struct Text {
    pub text: String,
    pub font: Rc<Font>,
    pub font_size: f32,
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
        let offset = glyphs.first().map(|glyph| Vec2::new(glyph.x, glyph.y)).unwrap_or_default();
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

    fn paint(&mut self, area: Area, scene: &mut Scene) {
        scene.text(crate::Text {
            font: self.font.clone(),
            origin: area.origin,
            font_size: self.font_size,
            text: self.text.clone(),
        })
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

    fn paint(&mut self, area: Area, scene: &mut Scene) {
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
                child.paint(area, scene);
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

    fn paint(&mut self, area: Area, scene: &mut Scene) {
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
                child.paint(area, scene);
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

    fn paint(&mut self, mut area: Area, scene: &mut Scene) {
        area.origin += self.offset;
        area.size -= self.offset;
        self.child.paint(area, scene);
    }
}
