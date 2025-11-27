use std::collections::HashMap;

// use yuv::{YuvPackedImage, YuvPackedImageMut};

use crate::parse::{
    CompositionObject, CompositionState, ObjectDefinition, PaletteDefinition, Pgs, SegmentContents,
    Window,
};

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
