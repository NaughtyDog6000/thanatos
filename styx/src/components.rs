use std::rc::Rc;

use fontdue::{layout::TextStyle, Font};
use glam::{Vec2, Vec4};

use crate::{Area, Constraint, Element, Rectangle, Scene};

pub struct Box<T: Element> {
    pub padding: f32,
    pub colour: Vec4,
    pub radius: f32,
    pub child: T,
}

impl<T: Element> Element for Box<T> {
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
        layout.append(&[self.font.clone()], &TextStyle::new(&self.text, self.font_size, 0));
        let glyphs = layout.glyphs();
        let width = glyphs
            .iter()
            .map(|glyph| glyph.x + glyph.width as f32)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or_default();
        let height = glyphs
            .iter()
            .map(|glyph| glyph.y + glyph.height as f32)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or_default();

        Vec2::new(width, height)
    }

    fn paint(&mut self, area: Area, scene: &mut Scene) {
        scene.text(crate::Text {
            font: self.font.clone(),
            origin: area.origin,
            font_size: self.font_size,
            text: self.text.clone()
        })
    }
}
