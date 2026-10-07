//! Locate the embedded read-only filesystem inside a portable ELF bundle without executing it.
//!
//! The payload begins after the ELF binary extent (section headers, segments, etc.). We mirror
//! the practical rule used by libappimage / AppImageKit (`appimage_get_elf_size`) and then
//! validate candidate offsets against a known payload magic.
//!
//! Two payload containers are recognised:
//!
//! * **SquashFS** (`hsqs`) — classic Type 2 AppImages built by AppImageKit.
//! * **DwarFS** (`DWARFS`) — images built by `pkgforge-dev/appimagetool` and run by
//!   `VHSgunzo/uruntime`. Same appended-payload shape, different container.
//!
//! The offset arithmetic is identical for both; only the magic differs.

use goblin::elf::{Elf, program_header::PT_LOAD};

use crate::error::AppImageError;

const SQUASHFS_MAGIC: &[u8] = b"hsqs";
const DWARFS_MAGIC: &[u8] = b"DWARFS";

/// Upper bound on how far past the computed ELF extent we look for a payload magic.
/// Runtimes pad to varying alignments; a bounded scan keeps exotic layouts working
/// without turning this into an unbounded search over the whole file.
const SCAN_LIMIT_BYTES: u64 = 1024 * 1024;
const SCAN_STEP_BYTES: u64 = 4096;

/// Container format of the filesystem appended to the runtime ELF.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PayloadFormat {
    SquashFs,
    DwarFs,
}

impl PayloadFormat {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::SquashFs => "squashfs",
            Self::DwarFs => "dwarfs",
        }
    }

    fn magic(self) -> &'static [u8] {
        match self {
            Self::SquashFs => SQUASHFS_MAGIC,
            Self::DwarFs => DWARFS_MAGIC,
        }
    }
}

/// Where the payload starts and what container it uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct PayloadLocation {
    pub offset: u64,
    pub format: PayloadFormat,
    /// `Some(2)` when the file carries the AppImage Type 2 magic, `None` for
    /// unmarked runtimes (some RunImage/AppBundle builds omit it).
    pub generation: Option<u8>,
}

/// Read the AppImage generation marker at bytes 8..11, if present.
#[must_use]
pub fn appimage_type_magic(bytes: &[u8]) -> Option<u8> {
    if bytes.len() < 11 {
        return None;
    }
    match &bytes[8..11] {
        [0x41, 0x49, 0x01] => Some(1),
        [0x41, 0x49, 0x02] => Some(2),
        _ => None,
    }
}

/// Locate the appended payload, accepting either SquashFS or DwarFS.
///
/// The AppImage Type 2 magic is treated as corroborating evidence rather than a
/// hard requirement: Type 1 is rejected outright (its ISO 9660 payload is not
/// readable by this crate), but an unmarked ELF is still accepted when a known
/// payload magic is found at a computed offset. That magic match is the real
/// safety check.
pub fn payload_location(bytes: &[u8]) -> Result<PayloadLocation, AppImageError> {
    if bytes.len() < 64 {
        return Err(AppImageError::TooSmall);
    }
    if bytes.get(..4) != Some(b"\x7fELF") {
        return Err(AppImageError::NotElf);
    }

    let generation = appimage_type_magic(bytes);
    if generation == Some(1) {
        return Err(AppImageError::TypeOneUnsupported);
    }

    let elf = Elf::parse(bytes).map_err(|err| AppImageError::ElfParse(err.to_string()))?;
    let end = elf_extent_end(&elf);

    for candidate in candidate_offsets(end, bytes.len() as u64) {
        for format in [PayloadFormat::SquashFs, PayloadFormat::DwarFs] {
            if magic_matches(bytes, candidate, format.magic()) {
                return Ok(PayloadLocation {
                    offset: candidate,
                    format,
                    generation,
                });
            }
        }
    }

    Err(AppImageError::PayloadNotFound)
}

/// Locate a SquashFS payload specifically.
///
/// Kept for callers that can only read SquashFS; a DwarFS payload is reported as
/// such rather than as "not found", so the operator gets a real answer.
pub fn squashfs_payload_offset(bytes: &[u8]) -> Result<u64, AppImageError> {
    let location = payload_location(bytes)?;
    match location.format {
        PayloadFormat::SquashFs => Ok(location.offset),
        PayloadFormat::DwarFs => Err(AppImageError::DwarfsPayload {
            offset: location.offset,
        }),
    }
}

fn elf_extent_end(elf: &Elf<'_>) -> u64 {
    let mut end = elf
        .header
        .e_shoff
        .saturating_add(u64::from(elf.header.e_shentsize) * u64::from(elf.header.e_shnum));

    for ph in elf.program_headers.iter() {
        if ph.p_type == PT_LOAD {
            end = end.max(ph.p_offset.saturating_add(ph.p_filesz));
        }
    }

    for sh in elf.section_headers.iter() {
        end = end.max(sh.sh_offset.saturating_add(sh.sh_size));
    }

    end
}

/// Candidate payload offsets, cheapest and most likely first: the exact ELF
/// extent, then its common alignments, then a bounded 4 KiB-step scan.
fn candidate_offsets(end: u64, file_len: u64) -> Vec<u64> {
    let mut candidates = Vec::new();
    let mut push = |value: u64| {
        if value < file_len && !candidates.contains(&value) {
            candidates.push(value);
        }
    };

    push(end);
    push(align_up(end, 1024));
    push(align_up(end, 4096));

    let scan_start = align_up(end, SCAN_STEP_BYTES);
    let scan_end = scan_start.saturating_add(SCAN_LIMIT_BYTES).min(file_len);
    let mut offset = scan_start;
    while offset < scan_end {
        push(offset);
        offset = offset.saturating_add(SCAN_STEP_BYTES);
    }

    candidates
}

fn align_up(value: u64, alignment: u64) -> u64 {
    value.div_ceil(alignment).saturating_mul(alignment)
}

fn magic_matches(bytes: &[u8], offset: u64, magic: &[u8]) -> bool {
    let Ok(start) = usize::try_from(offset) else {
        return false;
    };
    let end = start.saturating_add(magic.len());
    bytes.get(start..end) == Some(magic)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn align_up_rounds_to_the_next_boundary() {
        assert_eq!(align_up(0, 4096), 0);
        assert_eq!(align_up(1, 4096), 4096);
        assert_eq!(align_up(4096, 4096), 4096);
        assert_eq!(align_up(4097, 4096), 8192);
    }

    #[test]
    fn candidate_offsets_stay_inside_the_file_and_are_unique() {
        let candidates = candidate_offsets(100, 20_000);
        assert_eq!(candidates.first(), Some(&100));
        assert!(candidates.contains(&1024));
        assert!(candidates.contains(&4096));
        assert!(candidates.iter().all(|offset| *offset < 20_000));

        let mut sorted = candidates.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), candidates.len(), "candidates must be unique");
    }

    #[test]
    fn candidate_offsets_are_bounded_by_the_scan_limit() {
        let candidates = candidate_offsets(0, u64::from(u32::MAX));
        let expected = (SCAN_LIMIT_BYTES / SCAN_STEP_BYTES) as usize + 2;
        assert!(
            candidates.len() <= expected,
            "scan must stay bounded, got {}",
            candidates.len()
        );
    }

    #[test]
    fn magic_matches_detects_both_containers() {
        let mut bytes = vec![0u8; 32];
        bytes[8..12].copy_from_slice(b"hsqs");
        assert!(magic_matches(&bytes, 8, SQUASHFS_MAGIC));
        assert!(!magic_matches(&bytes, 8, DWARFS_MAGIC));

        let mut bytes = vec![0u8; 32];
        bytes[16..22].copy_from_slice(b"DWARFS");
        assert!(magic_matches(&bytes, 16, DWARFS_MAGIC));
        assert!(!magic_matches(&bytes, 16, SQUASHFS_MAGIC));
    }

    #[test]
    fn magic_lookup_past_end_of_buffer_is_not_a_match() {
        let bytes = b"DWARF".to_vec();
        assert!(!magic_matches(&bytes, 0, DWARFS_MAGIC));
    }

    #[test]
    fn type_one_appimages_are_rejected_with_a_specific_error() {
        let mut bytes = vec![0u8; 128];
        bytes[..4].copy_from_slice(b"\x7fELF");
        bytes[8..11].copy_from_slice(&[0x41, 0x49, 0x01]);

        let error = payload_location(&bytes).expect_err("type 1 must be rejected");
        assert!(
            matches!(error, AppImageError::TypeOneUnsupported),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn non_elf_input_is_rejected_before_any_scan() {
        let bytes = vec![0u8; 128];
        let error = payload_location(&bytes).expect_err("non-ELF must be rejected");
        assert!(matches!(error, AppImageError::NotElf), "got {error}");
    }

    #[test]
    fn short_input_is_rejected() {
        let error = payload_location(b"\x7fELF").expect_err("short input must be rejected");
        assert!(matches!(error, AppImageError::TooSmall), "got {error}");
    }
}
