use std::collections::HashMap;

use struple::Struple;
use winnow::Result as PResult;
use winnow::binary::{be_u8, be_u16, be_u24, be_u32, length_and_then, length_repeat};
use winnow::combinator::{alt, dispatch, fail, repeat};
use winnow::error::{ContextError, ParseError};
use winnow::prelude::*;

#[derive(Debug, PartialEq, Eq, Struple)]
pub struct Pgs {
    pub segments: Vec<Segment>,
}

#[derive(Debug, PartialEq, Eq, Struple)]
pub struct Segment {
    pub pts: u32,
    pub dts: u32,
    pub contents: SegmentContents,
}

#[derive(Debug, PartialEq, Eq, Struple)]
pub struct PresentationComposition {
    pub width: u16,
    pub height: u16,
    pub frame_rate: u8,
    pub composition_number: u16,
    pub composition_state: CompositionState,
    pub palette_update: bool,
    pub palette_id: u8,
    pub composition_objects: Vec<CompositionObject>,
}

#[derive(Debug, PartialEq, Eq, Struple)]
pub struct WindowDefinition {
    pub windows: Vec<Window>,
}
#[derive(Debug, PartialEq, Eq, Struple)]
pub struct PaletteDefinition {
    pub id: u8,
    pub version: u8,
    pub entries: HashMap<u8, PaletteEntry>,
}

#[derive(Debug, PartialEq, Eq, Struple)]
pub struct ObjectDefinition {
    pub id: u16,
    pub version: u8,
    pub last_in_sequence: LastInSequence,
    pub width: u16,
    pub height: u16,
    pub data: RunLengthEncodedData,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SegmentContents {
    PresentationComposition(PresentationComposition),
    WindowDefinition(WindowDefinition),
    PaletteDefinition(PaletteDefinition),
    ObjectDefinition(ObjectDefinition),
    End,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum LastInSequence {
    Last,
    First,
    FirstAndLast,
}
#[derive(PartialEq, Eq)]
pub struct RunLengthEncodedData(pub Vec<RlEncodedPixels>);

impl std::fmt::Debug for RunLengthEncodedData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("RunLengthEncodedData")
            .field(&self.0.iter().map(|p| p.count as u64).sum::<u64>())
            .finish()
    }
}

#[derive(Debug, PartialEq, Eq, Struple)]
pub struct Window {
    pub id: u8,
    pub horizontal_position: u16,
    pub vertical_position: u16,
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, PartialEq, Eq, Struple)]
pub struct PaletteEntry {
    pub id: u8,
    pub luminance: u8,
    pub color_difference_red: u8,
    pub color_difference_blue: u8,
    pub alpha: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum CompositionState {
    Normal,
    AcquisitionPoint,
    EpochStart,
}

#[derive(Debug, PartialEq, Eq, Struple)]
pub struct CompositionObject {
    pub id: u16,
    pub window_id: u8,
    pub horizontal_position: u16,
    pub vertical_position: u16,
    pub cropped: Option<CropInfo>,
}

#[derive(Debug, PartialEq, Eq, Struple)]
pub struct CropInfo {
    pub horizontal_position: u16,
    pub vertical_position: u16,
    pub width: u16,
    pub height: u16,
}

pub fn parse_pgs<'a>(input: &'a mut [u8]) -> Result<Pgs, ParseError<&'a [u8], ContextError>> {
    let segments = repeat(1.., parse_segment).parse(input)?;
    Ok(Pgs { segments })
}

fn parse_segment(input: &mut &[u8]) -> PResult<Segment> {
    // Verify magic number "PG" is present.
    be_u16.verify(|&v| v == 0x5047).parse_next(input)?;
    let (pts, dts, contents) = (
        be_u32,
        be_u32,
        dispatch! {be_u8;
            0x14 => parse_palette_definition_segment.map(|v| SegmentContents::PaletteDefinition(v)),
            0x15 => parse_object_definition_segment.map(|v| SegmentContents::ObjectDefinition(v)),
            0x16 => parse_presentation_composition_segment.map(|v| SegmentContents::PresentationComposition(v)),
            0x17 => parse_window_definition_segment.map(|v| SegmentContents::WindowDefinition(v)),
            0x80 => parse_end_of_display_set_segment.map(|_| SegmentContents::End),
            _ => fail::<_, SegmentContents, _>,
        },
    )
        .parse_next(input)?;
    Ok(Segment { pts, dts, contents })
}

fn parse_end_of_display_set_segment(input: &mut &[u8]) -> PResult<()> {
    be_u16.verify(|&v| v == 0x0000).parse_next(input)?;
    Ok(())
}

fn parse_object_definition_segment(input: &mut &[u8]) -> PResult<ObjectDefinition> {
    let (id, version, last_in_sequence, (width, height, data)) = length_and_then(
        be_u16,
        (
            be_u16,
            be_u8,
            parse_last_in_sequence,
            // length_and_then(be_u24.map(|l| l + 4), (be_u16, be_u16, repeat(0.., be_u8))),
            length_and_then(be_u24, (be_u16, be_u16, parse_run_length_encoded_pixels)),
        ),
    )
    .parse_next(input)?;
    Ok(ObjectDefinition {
        id,
        version,
        last_in_sequence,
        width,
        height,
        data: RunLengthEncodedData(data),
    })
}

#[derive(PartialEq, Eq, Struple)]
pub struct RlEncodedPixels {
    pub count: u16,
    pub color: u8,
}
fn parse_run_length_encoded_pixels(input: &mut &[u8]) -> PResult<Vec<RlEncodedPixels>> {
    Ok(repeat(0.., parse_single_encoded_pixel).parse_next(input)?)
}

fn parse_single_encoded_pixel(input: &mut &[u8]) -> PResult<RlEncodedPixels> {
    alt((
        // Single pixel
        be_u8
            .verify(|v| *v != 0x00)
            .map(|v| RlEncodedPixels { count: 1, color: v }),
        // Run of base color
        (be_u8, be_u8)
            .verify(|v| (v.1 >> 6) == 0)
            .map(|v| RlEncodedPixels {
                count: (v.1 & 0x3F) as u16,
                color: 0,
            }),
        // Long run of base color
        (be_u8, be_u8, be_u8)
            .verify(|v| (v.1 >> 6) == 1)
            .map(|v| RlEncodedPixels {
                count: (v.1 as u16 & 0x3F) << 8 | v.2 as u16,
                color: 0,
            }),
        // Run of other color
        (be_u8, be_u8, be_u8)
            .verify(|v| (v.1 >> 6) == 2)
            .map(|v| RlEncodedPixels {
                count: (v.1 & 0x3F) as u16,
                color: v.2,
            }),
        // Long run of other color
        (be_u8, be_u8, be_u8, be_u8)
            .verify(|v| (v.1 >> 6) == 3)
            .map(|v| RlEncodedPixels {
                count: (v.1 as u16 & 0x3F) << 8 | v.2 as u16,
                color: v.3,
            }),
        (be_u16)
            .verify(|v| *v == 0)
            .map(|_| RlEncodedPixels { count: 0, color: 0 }),
    ))
    .parse_next(input)
}

fn parse_last_in_sequence(input: &mut &[u8]) -> PResult<LastInSequence> {
    Ok(alt((
        be_u8.verify(|v| *v == 0x40).value(LastInSequence::Last),
        be_u8.verify(|v| *v == 0x80).value(LastInSequence::First),
        be_u8
            .verify(|v| *v == 0xC0)
            .value(LastInSequence::FirstAndLast),
    ))
    .parse_next(input)?)
}

fn parse_palette_definition_segment(input: &mut &[u8]) -> PResult<PaletteDefinition> {
    Ok(PaletteDefinition::from_tuple(
        length_and_then(be_u16, (be_u8, be_u8, parse_palette_entries)).parse_next(input)?,
    ))
}

fn parse_palette_entries(input: &mut &[u8]) -> PResult<HashMap<u8, PaletteEntry>> {
    let entries: Vec<PaletteEntry> = repeat(0.., parse_palette_entry).parse_next(input)?;
    let mut palette = HashMap::new();
    for entry in entries {
        palette.insert(entry.id, entry);
    }
    Ok(palette)
}

fn parse_palette_entry(input: &mut &[u8]) -> PResult<PaletteEntry> {
    Ok(PaletteEntry::from_tuple(
        (be_u8, be_u8, be_u8, be_u8, be_u8).parse_next(input)?,
    ))
}

fn parse_window_definition_segment(input: &mut &[u8]) -> PResult<WindowDefinition> {
    Ok(WindowDefinition {
        windows: (length_and_then(be_u16, length_repeat(be_u8, parse_window))).parse_next(input)?,
    })
}

fn parse_window(input: &mut &[u8]) -> PResult<Window> {
    Ok(Window::from_tuple(
        (be_u8, be_u16, be_u16, be_u16, be_u16).parse_next(input)?,
    ))
}

fn parse_presentation_composition_segment(input: &mut &[u8]) -> PResult<PresentationComposition> {
    Ok(PresentationComposition::from_tuple(
        length_and_then(
            be_u16,
            (
                be_u16,
                be_u16,
                be_u8,
                be_u16,
                parse_composition_state,
                parse_palette_update_flag,
                be_u8,
                length_repeat(be_u8, parse_composition_object),
            ),
        )
        .parse_next(input)?,
    ))
}

fn parse_composition_state(input: &mut &[u8]) -> PResult<CompositionState> {
    Ok(alt((
        be_u8.verify(|v| *v == 0x00).value(CompositionState::Normal),
        be_u8
            .verify(|v| *v == 0x40)
            .value(CompositionState::AcquisitionPoint),
        be_u8
            .verify(|v| *v == 0x80)
            .value(CompositionState::EpochStart),
    ))
    .parse_next(input)?)
}

fn parse_palette_update_flag(input: &mut &[u8]) -> PResult<bool> {
    Ok(alt((
        be_u8.verify(|v| *v == 0x00).value(false),
        be_u8.verify(|v| *v == 0x80).value(true),
    ))
    .parse_next(input)?)
}

fn parse_composition_object(input: &mut &[u8]) -> PResult<CompositionObject> {
    let (id, window_id, cropped, horizontal_position, vertical_position) =
        (be_u16, be_u8, parse_object_cropped_flag, be_u16, be_u16).parse_next(input)?;
    if !cropped {
        return Ok(CompositionObject {
            id,
            window_id,
            horizontal_position,
            vertical_position,
            cropped: None,
        });
    }
    let crop_info = CropInfo::from_tuple((be_u16, be_u16, be_u16, be_u16).parse_next(input)?);
    Ok(CompositionObject {
        id,
        window_id,
        horizontal_position,
        vertical_position,
        cropped: Some(crop_info),
    })
}

fn parse_object_cropped_flag(input: &mut &[u8]) -> PResult<bool> {
    let flag = be_u8.parse_next(input)?;
    Ok(flag == 0x40)
}
