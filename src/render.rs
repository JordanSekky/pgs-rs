use std::collections::HashMap;

use yuv::{YuvPackedImage, YuvRange, YuvStandardMatrix};

use crate::{
    error::{PgsError, PgsResult},
    parse::{
        CompositionObject, CompositionState, ObjectDefinition, PaletteDefinition, Pgs,
        SegmentContents, Window,
    },
};

const PIXEL_SIZE: usize = 4;

// type MutableImage<'a> = YuvPackedImageMut<'a, u8>;
// type Image<'a> = YuvPackedImage<'a, u8>;

#[derive(Debug, PartialEq, Eq)]
pub struct DisplaySet<'a> {
    pub presentation_timestamp: u32,
    pub decoding_timestamp: u32,
    pub width: u16,
    pub height: u16,
    pub frame_rate: u8,
    pub composition_number: u16,
    pub composition_state: CompositionState,
    pub palette_update: bool,
    pub palette_id: u8,
    pub composition_objects: &'a [CompositionObject],
    pub windows: HashMap<u8, &'a Window>,
    pub palettes: HashMap<u8, &'a PaletteDefinition>,
    pub objects: HashMap<u16, &'a ObjectDefinition>,
}

impl<'a> DisplaySet<'a> {
    pub fn is_empty(&self) -> bool {
        self.composition_objects.is_empty()
    }
}

pub struct DisplaySetIterator<'a> {
    pgs: &'a Pgs,
    index: usize,
    windows: HashMap<u8, &'a Window>,
    palettes: HashMap<u8, &'a PaletteDefinition>,
    objects: HashMap<u16, &'a ObjectDefinition>,
}
impl<'a> DisplaySetIterator<'a> {
    pub fn new(pgs: &'a Pgs) -> Self {
        Self {
            pgs,
            index: 0,
            windows: HashMap::new(),
            palettes: HashMap::new(),
            objects: HashMap::new(),
        }
    }
}

impl<'a> Iterator for DisplaySetIterator<'a> {
    type Item = DisplaySet<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.pgs.segments.len() {
            return None;
        }
        let start_index = self.index;
        let presentation_timestamp = self.pgs.segments[start_index].pts;
        let decoding_timestamp = self.pgs.segments[start_index].dts;
        let presentation_composition = match &self.pgs.segments[start_index].contents {
            SegmentContents::PresentationComposition(presentation_composition) => {
                presentation_composition
            }
            _ => return None,
        };
        self.index += 1;

        if CompositionState::EpochStart == presentation_composition.composition_state {
            self.windows.clear();
            self.palettes.clear();
            self.objects.clear();
        }

        let mut display_set = DisplaySet {
            presentation_timestamp,
            decoding_timestamp,
            width: presentation_composition.width,
            height: presentation_composition.height,
            frame_rate: presentation_composition.frame_rate,
            composition_number: presentation_composition.composition_number,
            composition_state: presentation_composition.composition_state,
            palette_update: presentation_composition.palette_update,
            palette_id: presentation_composition.palette_id,
            composition_objects: &presentation_composition.composition_objects,
            windows: self.windows.clone(),
            palettes: self.palettes.clone(),
            objects: self.objects.clone(),
        };
        loop {
            if self.index >= self.pgs.segments.len() {
                return None;
            }
            if self.pgs.segments[self.index].pts != presentation_timestamp {
                panic!("Presentation timestamp mismatch");
            }
            if self.pgs.segments[self.index].dts != decoding_timestamp {
                panic!("Decoding timestamp mismatch");
            }
            match &self.pgs.segments[self.index].contents {
                SegmentContents::PresentationComposition(_) => {
                    panic!("Presentation composition in the middle of a display set")
                }
                SegmentContents::WindowDefinition(window_definition) => {
                    for window in &window_definition.windows {
                        display_set.windows.insert(window.id, window);
                    }
                }
                SegmentContents::PaletteDefinition(palette_definition) => {
                    display_set
                        .palettes
                        .insert(palette_definition.id, palette_definition);
                }
                SegmentContents::ObjectDefinition(object_definition) => {
                    display_set
                        .objects
                        .insert(object_definition.id, object_definition);
                }
                SegmentContents::End => {
                    self.index += 1;
                    return Some(display_set);
                }
            }
            self.index += 1;
        }
    }
}

pub fn get_display_sets<'a>(pgs: &'a Pgs) -> impl Iterator<Item = DisplaySet<'a>> {
    return DisplaySetIterator::new(pgs);
}

pub fn render_display_set(display_set: &DisplaySet) -> PgsResult<Vec<u8>> {
    let width = display_set.width as usize;
    let height = display_set.height as usize;
    let stride = width * PIXEL_SIZE;
    let mut buf = vec![0u8; stride * height];

    for composition_object in display_set.composition_objects {
        let Some(object) = display_set.objects.get(&composition_object.id) else {
            return Err(PgsError::ObjectNotFound {
                object_id: composition_object.id,
                display_set: format!("{:?}", display_set),
            });
        };
        let x = composition_object.horizontal_position as usize;
        let y = composition_object.vertical_position as usize;

        let mut pixel_offset = (y * width + x) * PIXEL_SIZE;

        for pixel in object.data.0.iter() {
            let Some(pixel_color) = display_set
                .palettes
                // TODO: Is multiple palettes allowed?
                .get(&0)
                .and_then(|palette| palette.entries.get(&pixel.color))
            else {
                return Err(PgsError::PaletteNotFound {
                    palette_id: 0,
                    entry_id: pixel.color,
                    display_set: format!("{:?}", display_set),
                });
            };
            for _ in 0..pixel.count {
                if !is_cropped(&pixel_offset, composition_object) {
                    buf[pixel_offset] = pixel_color.alpha;
                    buf[pixel_offset + 1] = pixel_color.luminance;
                    buf[pixel_offset + 2] = pixel_color.color_difference_blue;
                    buf[pixel_offset + 3] = pixel_color.color_difference_red;
                }
                move_one_pixel_forward(
                    &mut pixel_offset,
                    width,
                    composition_object.horizontal_position as usize,
                    object.width as usize,
                );
            }
        }
    }

    let image = YuvPackedImage {
        yuy: &buf,
        yuy_stride: stride as u32,
        width: display_set.width as u32,
        height: display_set.height as u32,
    };

    image.check_constraints444()?;

    let mut rgba = vec![0u8; stride * height];

    yuv::ayuv_to_rgba(
        &image,
        &mut rgba,
        stride as u32,
        YuvRange::Full,
        YuvStandardMatrix::Bt709,
        false,
    )?;

    Ok(rgba)
}

fn is_cropped(pixel_offset: &usize, object: &CompositionObject) -> bool {
    if let Some(cropped) = &object.cropped {
        return is_cropped_1(
            pixel_offset as &usize,
            cropped.horizontal_position as usize,
            cropped.vertical_position as usize,
            cropped.width as usize,
            cropped.height as usize,
        );
    }
    false
}

fn is_cropped_1(
    pixel_offset: &usize,
    top_left_x: usize,
    top_left_y: usize,
    width: usize,
    height: usize,
) -> bool {
    let x = (*pixel_offset / PIXEL_SIZE) % width;
    let y = (*pixel_offset / PIXEL_SIZE) / width;
    x < top_left_x || x >= top_left_x + width || y < top_left_y || y >= top_left_y + height
}

fn move_one_pixel_forward(
    pixel_offset: &mut usize,
    width: usize,
    horizontal_position: usize,
    object_width: usize,
) {
    *pixel_offset += PIXEL_SIZE;
    let x = (*pixel_offset / PIXEL_SIZE) % width;
    if x == (horizontal_position + object_width) {
        *pixel_offset += (width - object_width) * PIXEL_SIZE
    }
}
