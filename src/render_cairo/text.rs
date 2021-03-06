// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::f64;

// external
use cairo;
use pango::{
    self,
    LayoutExt,
    ContextExt,
};
use pangocairo::functions as pc;
use usvg::tree::prelude::*;

// self
use utils;
use geom::*;
use super::{
    fill,
    stroke,
};
use {
    Options
};


const PANGO_SCALE_64: f64 = pango::SCALE as f64;


pub struct PangoData {
    pub layout: pango::Layout,
    pub context: pango::Context,
    pub font: pango::FontDescription,
}

pub fn draw(
    node: &tree::Node,
    opt: &Options,
    cr: &cairo::Context,
) -> Rect {
    draw_tspan(node, opt, cr,
        |tspan, x, y, w, d| _draw_tspan(node, tspan, opt, x, y, w, d, cr))
}

pub fn draw_tspan<DrawAt>(
    node: &tree::Node,
    opt: &Options,
    cr: &cairo::Context,
    mut draw_at: DrawAt,
) -> Rect
    where DrawAt: FnMut(&tree::TSpan, f64, f64, f64, &PangoData)
{
    let mut bbox = Rect::new_bbox();
    let mut pc_list = Vec::new();
    let mut tspan_w_list = Vec::new();
    for chunk_node in node.children() {
        pc_list.clear();
        tspan_w_list.clear();
        let mut chunk_width = 0.0;

        if let tree::NodeKind::TextChunk(ref chunk) = *chunk_node.kind() {
            for tspan_node in chunk_node.children() {
                if let tree::NodeKind::TSpan(ref tspan) = *tspan_node.kind() {
                    let context = pc::create_context(cr).unwrap();
                    pc::update_context(cr, &context);
                    pc::context_set_resolution(&context, opt.usvg.dpi);

                    let font = init_font(&tspan.font, opt.usvg.dpi);

                    let layout = pango::Layout::new(&context);
                    layout.set_font_description(Some(&font));
                    layout.set_text(&tspan.text);
                    let tspan_width = layout.get_size().0 as f64 / PANGO_SCALE_64;

                    let mut layout_iter = layout.get_iter().unwrap();
                    let ascent = (layout_iter.get_baseline() / pango::SCALE) as f64;
                    let text_h = layout.get_size().1 as f64 / PANGO_SCALE_64;

                    pc_list.push(PangoData {
                        layout,
                        context,
                        font,
                    });
                    chunk_width += tspan_width;
                    tspan_w_list.push((tspan_width, ascent));

                    bbox.expand(Rect::from_xywh(chunk.x, chunk.y - ascent, chunk_width, text_h));
                }
            }

            let mut x = utils::process_text_anchor(chunk.x, chunk.anchor, chunk_width);

            for (idx, tspan_node) in chunk_node.children().enumerate() {
                if let tree::NodeKind::TSpan(ref tspan) = *tspan_node.kind() {
                    let (width, ascent) = tspan_w_list[idx];
                    let pc = &pc_list[idx];

                    draw_at(tspan, x, chunk.y - ascent, width, pc);
                    x += width;
                }
            }
        }
    }

    if bbox.x() == f64::MAX { bbox.origin.x = 0.0; }
    if bbox.y() == f64::MAX { bbox.origin.y = 0.0; }

    bbox
}

fn _draw_tspan(
    node: &tree::Node,
    tspan: &tree::TSpan,
    opt: &Options,
    x: f64,
    y: f64,
    width: f64,
    pd: &PangoData,
    cr: &cairo::Context,
) {
    let font_metrics = pd.context.get_metrics(Some(&pd.font), None).unwrap();

    let mut layout_iter = pd.layout.get_iter().unwrap();
    let baseline_offset = (layout_iter.get_baseline() / pango::SCALE) as f64;

    // Contains only characters path bounding box,
    // so spaces around text are ignored.
    let bbox = calc_layout_bbox(&pd.layout, x, y);

    let mut line_rect = Rect::from_xywh(
        x,
        0.0,
        width,
        font_metrics.get_underline_thickness() as f64 / PANGO_SCALE_64,
    );

    // Draw underline.
    //
    // Should be drawn before/under text.
    if let Some(ref style) = tspan.decoration.underline {
        line_rect.origin.y = y + baseline_offset
                             - font_metrics.get_underline_position() as f64 / PANGO_SCALE_64;
        draw_line(&node.tree(), line_rect, &style.fill, &style.stroke, opt, cr);
    }

    // Draw overline.
    //
    // Should be drawn before/under text.
    if let Some(ref style) = tspan.decoration.overline {
        line_rect.origin.y = y + font_metrics.get_underline_thickness() as f64 / PANGO_SCALE_64;
        draw_line(&node.tree(), line_rect, &style.fill, &style.stroke, opt, cr);
    }

    // Draw text.
    cr.move_to(x, y);

    fill::apply(&node.tree(), &tspan.fill, opt, bbox, cr);
    pc::update_layout(cr, &pd.layout);
    pc::show_layout(cr, &pd.layout);

    stroke::apply(&node.tree(), &tspan.stroke, opt, bbox, cr);
    pc::layout_path(cr, &pd.layout);
    cr.stroke();

    cr.move_to(-x, -y);

    // Draw line-through.
    //
    // Should be drawn after/over text.
    if let Some(ref style) = tspan.decoration.line_through {
        line_rect.origin.y = y + baseline_offset
                             - font_metrics.get_strikethrough_position() as f64 / PANGO_SCALE_64;
        line_rect.size.height = font_metrics.get_strikethrough_thickness() as f64 / PANGO_SCALE_64;
        draw_line(&node.tree(), line_rect, &style.fill, &style.stroke, opt, cr);
    }
}

fn init_font(dom_font: &tree::Font, dpi: f64) -> pango::FontDescription {
    let mut font = pango::FontDescription::new();

    font.set_family(&dom_font.family);

    let font_style = match dom_font.style {
        tree::FontStyle::Normal => pango::Style::Normal,
        tree::FontStyle::Italic => pango::Style::Italic,
        tree::FontStyle::Oblique => pango::Style::Oblique,
    };
    font.set_style(font_style);

    let font_variant = match dom_font.variant {
        tree::FontVariant::Normal => pango::Variant::Normal,
        tree::FontVariant::SmallCaps => pango::Variant::SmallCaps,
    };
    font.set_variant(font_variant);

    let font_weight = match dom_font.weight {
        tree::FontWeight::W100       => pango::Weight::Thin,
        tree::FontWeight::W200       => pango::Weight::Ultralight,
        tree::FontWeight::W300       => pango::Weight::Light,
        tree::FontWeight::W400       => pango::Weight::Normal,
        tree::FontWeight::W500       => pango::Weight::Medium,
        tree::FontWeight::W600       => pango::Weight::Semibold,
        tree::FontWeight::W700       => pango::Weight::Bold,
        tree::FontWeight::W800       => pango::Weight::Ultrabold,
        tree::FontWeight::W900       => pango::Weight::Heavy,
    };
    font.set_weight(font_weight);

    let font_stretch = match dom_font.stretch {
        tree::FontStretch::Normal         => pango::Stretch::Normal,
        tree::FontStretch::Narrower |
        tree::FontStretch::Condensed      => pango::Stretch::Condensed,
        tree::FontStretch::UltraCondensed => pango::Stretch::UltraCondensed,
        tree::FontStretch::ExtraCondensed => pango::Stretch::ExtraCondensed,
        tree::FontStretch::SemiCondensed  => pango::Stretch::SemiCondensed,
        tree::FontStretch::SemiExpanded   => pango::Stretch::SemiExpanded,
        tree::FontStretch::Wider |
        tree::FontStretch::Expanded       => pango::Stretch::Expanded,
        tree::FontStretch::ExtraExpanded  => pango::Stretch::ExtraExpanded,
        tree::FontStretch::UltraExpanded  => pango::Stretch::UltraExpanded,
    };
    font.set_stretch(font_stretch);

    // a-font-size-001.svg
    let font_size = dom_font.size * PANGO_SCALE_64 / dpi * 72.0;
    font.set_size(font_size as i32);

    font
}

pub fn calc_layout_bbox(layout: &pango::Layout, x: f64, y: f64) -> Rect {
    let (ink_rect, _) = layout.get_extents();

    Rect::from_xywh(
        x + ink_rect.x  as f64 / PANGO_SCALE_64,
        y + ink_rect.y  as f64 / PANGO_SCALE_64,
        ink_rect.width  as f64 / PANGO_SCALE_64,
        ink_rect.height as f64 / PANGO_SCALE_64,
    )
}

fn draw_line(
    tree: &tree::Tree,
    line_bbox: Rect,
    fill: &Option<tree::Fill>,
    stroke: &Option<tree::Stroke>,
    opt: &Options,
    cr: &cairo::Context,
) {
    // TODO: to rect
    cr.new_sub_path();
    cr.move_to(line_bbox.x(), line_bbox.y());
    cr.rel_line_to(line_bbox.width(), 0.0);
    cr.rel_line_to(0.0, line_bbox.height());
    cr.rel_line_to(-line_bbox.width(), 0.0);
    cr.close_path();

    fill::apply(tree, fill, opt, line_bbox, cr);
    if stroke.is_some() {
        cr.fill_preserve();

        stroke::apply(tree, &stroke, opt, line_bbox, cr);
        cr.stroke();
    } else {
        cr.fill();
    }
}
