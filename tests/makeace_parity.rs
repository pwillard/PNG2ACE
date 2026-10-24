//! Byte-level parity checks against ACE files written by Kuju MakeACE v1.04.
//!
//! The fixtures in `tests/fixtures` were produced by the original `makeace.exe`
//! from the TGA files next to them:
//!   makeace solid.tga makeace_solid.ace
//!   makeace ramp.tga  makeace_alpha.ace
//!   makeace solid.tga makeace_solid_dxt.ace -dxt
//!   makeace ramp.tga  makeace_trans_dxt.ace -dxt -trans
//!
//! MakeACE point-samples its mipmaps while PNG2ACE box-filters them, so only the
//! container layout and the full-size image are expected to match exactly.

use std::path::PathBuf;

use png2ace::{AceFormat, ConversionOptions, convert_image_file};

const SIGNATURE: usize = 16;
const HEADER_FIELDS: usize = SIGNATURE + 7 * 4; // six header u32s plus the reserved u32
const CHANNEL_TABLE: usize = SIGNATURE + 152;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn convert(source: &str, format: AceFormat, tag: &str) -> Vec<u8> {
    let output =
        std::env::temp_dir().join(format!("png2ace_parity_{tag}_{}.ace", std::process::id()));
    convert_image_file(
        fixture(source),
        &output,
        ConversionOptions {
            mipmaps: true,
            format,
        },
    )
    .unwrap();
    let bytes = std::fs::read(&output).unwrap();
    std::fs::remove_file(&output).unwrap();
    bytes
}

fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

struct Layout {
    table_start: usize,
    table_end: usize,
}

fn layout(bytes: &[u8], table_entries: usize) -> Layout {
    let channels = u32_at(bytes, SIGNATURE + 20) as usize;
    let table_start = CHANNEL_TABLE + channels * 16;
    Layout {
        table_start,
        table_end: table_start + table_entries * 4,
    }
}

/// Header fields, channel table and offset table must be identical.
fn assert_container_matches(ours: &[u8], makeace: &[u8], table_entries: usize) {
    assert_eq!(
        &ours[..HEADER_FIELDS],
        &makeace[..HEADER_FIELDS],
        "header fields"
    );
    let ours_layout = layout(ours, table_entries);
    let ref_layout = layout(makeace, table_entries);
    assert_eq!(
        &ours[CHANNEL_TABLE..ours_layout.table_start],
        &makeace[CHANNEL_TABLE..ref_layout.table_start],
        "channel table"
    );
    assert_eq!(
        &ours[ours_layout.table_start..ours_layout.table_end],
        &makeace[ref_layout.table_start..ref_layout.table_end],
        "offset table"
    );
    assert_eq!(ours.len(), makeace.len(), "file size");
}

/// Full-size image rows (first 32 rows after the offset table) must be identical.
fn assert_level0_matches(ours: &[u8], makeace: &[u8], table_entries: usize) {
    let l = layout(ours, table_entries);
    let row1 = u32_at(ours, l.table_start + 4) as usize + SIGNATURE;
    let row0 = u32_at(ours, l.table_start) as usize + SIGNATURE;
    let level0_end = row0 + (row1 - row0) * 32;
    assert_eq!(
        row0, l.table_end,
        "first row starts right after the offset table"
    );
    assert_eq!(
        &ours[row0..level0_end],
        &makeace[row0..level0_end],
        "level 0 pixels"
    );
}

// 32x32 with mipmaps: 32+16+8+4+2+1 row offsets.
const STRUCTURED_ROWS: usize = 63;
// One offset per DXT mip level.
const DXT_LEVELS: usize = 6;

#[test]
fn structured_solid_matches_makeace() {
    let ours = convert("solid.tga", AceFormat::Structured, "solid");
    let makeace = std::fs::read(fixture("makeace_solid.ace")).unwrap();
    assert_container_matches(&ours, &makeace, STRUCTURED_ROWS);
    assert_level0_matches(&ours, &makeace, STRUCTURED_ROWS);
}

#[test]
fn structured_alpha_matches_makeace() {
    let ours = convert("ramp.tga", AceFormat::Structured, "alpha");
    let makeace = std::fs::read(fixture("makeace_alpha.ace")).unwrap();
    assert_container_matches(&ours, &makeace, STRUCTURED_ROWS);
    assert_level0_matches(&ours, &makeace, STRUCTURED_ROWS);
}

#[test]
fn dxt1_solid_matches_makeace_container() {
    let ours = convert("solid.tga", AceFormat::Dxt1, "dxt1");
    let makeace = std::fs::read(fixture("makeace_solid_dxt.ace")).unwrap();
    assert_container_matches(&ours, &makeace, DXT_LEVELS);
}

#[test]
fn dxt1_transparent_matches_makeace_container() {
    let ours = convert("ramp.tga", AceFormat::Dxt1, "dxt1t");
    let makeace = std::fs::read(fixture("makeace_trans_dxt.ace")).unwrap();
    assert_container_matches(&ours, &makeace, DXT_LEVELS);
}
