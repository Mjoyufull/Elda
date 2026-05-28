//! Locate the embedded SquashFS image inside a Type 2 AppImage without executing it.
//!
//! The payload begins immediately after the ELF binary extent (section headers, segments,
//! etc.). We mirror the practical rule used by libappimage / AppImageKit (`appimage_get_elf_size`)
//! and validate candidates against the SquashFS magic `hsqs`.

use goblin::elf::{Elf, program_header::PT_LOAD};

use crate::error::AppImageError;

const SQUASHFS_MAGIC: &[u8] = b"hsqs";

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

pub fn squashfs_payload_offset(bytes: &[u8]) -> Result<u64, AppImageError> {
    if bytes.len() < 64 {
        return Err(AppImageError::TooSmall);
    }
    if bytes.get(..4) != Some(b"\x7fELF") {
        return Err(AppImageError::NotElf);
    }
    match appimage_type_magic(bytes) {
        Some(2) => {}
        Some(_) => {
            return Err(AppImageError::UnsupportedGeneration);
        }
        None => {
            return Err(AppImageError::UnsupportedGeneration);
        }
    }

    let elf = Elf::parse(bytes).map_err(|err| AppImageError::ElfParse(err.to_string()))?;

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

    let aligned = align4096(end);

    for candidate in dedup_candidates([end, aligned]) {
        if squash_magic_matches(bytes, candidate) {
            return Ok(candidate);
        }
    }

    Err(AppImageError::SquashfsNotFound)
}

fn dedup_candidates<const N: usize>(vals: [u64; N]) -> Vec<u64> {
    let mut out = Vec::new();
    for v in vals {
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

fn align4096(n: u64) -> u64 {
    (n + 4095) & !4095
}

fn squash_magic_matches(bytes: &[u8], offset: u64) -> bool {
    let start = offset as usize;
    let end = start.saturating_add(SQUASHFS_MAGIC.len());
    bytes.get(start..end) == Some(SQUASHFS_MAGIC)
}
