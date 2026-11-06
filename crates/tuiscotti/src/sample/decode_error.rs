use std::collections::TryReserveError;
use std::fmt::{self, Display, Formatter};

use super::{PngDecoderError, SampleError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "A bounded path within the decompressed binary message."]
pub struct WirePath {
    fields: [u8; 5],
    fields_len: u8,
    index: Option<u64>,
    index_after: u8,
    tag: Option<u32>,
}

impl WirePath {
    pub(super) const fn new() -> Self {
        Self {
            fields: [0; 5],
            fields_len: 0,
            index: None,
            index_after: 0,
            tag: None,
        }
    }
    pub(super) fn field(&mut self, id: u8) -> bool {
        let index = usize::from(self.fields_len);
        let Some(slot) = self.fields.get_mut(index) else {
            return false;
        };
        if self.index.is_some() && index < usize::from(self.index_after) {
            return false;
        }
        *slot = id;
        self.fields_len += 1;
        true
    }
    pub(super) fn index(&mut self, index: u64) -> bool {
        if self.index.is_some() || self.fields_len == 0 {
            return false;
        }
        self.index = Some(index);
        self.index_after = self.fields_len;
        true
    }
    pub(super) fn tag(&mut self, tag: u32) -> bool {
        if self.tag.is_some() {
            return false;
        }
        self.tag = Some(tag);
        true
    }
    pub(super) fn pop_index(&mut self) {
        self.index = None;
        self.index_after = 0;
    }
    pub(super) fn pop_field(&mut self) {
        if self.fields_len > 0 {
            self.fields_len -= 1;
        }
    }
    fn field_name(id: u8) -> &'static str {
        super::wire::field_name(id)
    }
    fn write_fields(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for index in 0..usize::from(self.fields_len) {
            if index != 0 {
                f.write_str(".")?;
            }
            f.write_str(Self::field_name(self.fields[index]))?;
            if self.index_after as usize == index + 1
                && let Some(value) = self.index
            {
                write!(f, "[{value}]")?;
            }
        }
        if let Some(tag) = self.tag {
            if self.fields_len != 0 {
                f.write_str(".")?;
            }
            write!(f, "#{tag}")?;
        }
        Ok(())
    }
}

impl Display for WirePath {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.write_fields(f)
    }
}

#[derive(Debug)]
#[doc = "A bounded Zstandard decoder error."]
pub struct ZstdError {
    pub(super) code: usize,
}
impl Display for ZstdError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(zstd_safe::get_error_name(self.code))
    }
}
impl std::error::Error for ZstdError {}
const _: () = assert!(std::mem::size_of::<ZstdError>() <= 4096);

#[derive(Debug)]
#[non_exhaustive]
#[doc = "An invalid binary sample encoding."]
pub enum BinaryFormatError {
    #[doc = "An invalid container header."]
    Header {
        #[doc = "The file byte offset."]
        offset: usize,
        #[doc = "A bounded explanation."]
        detail: &'static str,
    },
    #[doc = "An invalid compressed frame."]
    Frame {
        #[doc = "The file byte offset."]
        offset: usize,
        #[doc = "A bounded explanation."]
        detail: &'static str,
    },
    #[doc = "An invalid message field."]
    Wire {
        #[doc = "The decompressed message byte offset."]
        offset: usize,
        #[doc = "The field path."]
        path: WirePath,
        #[doc = "A bounded explanation."]
        detail: &'static str,
    },
    #[doc = "The codec rejected the frame."]
    Codec {
        #[doc = "The codec error."]
        source: Box<ZstdError>,
    },
}
impl Display for BinaryFormatError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Header { offset, detail } => {
                write!(f, "invalid header at byte {offset}: {detail}")
            }
            Self::Frame { offset, detail } => write!(f, "invalid frame at byte {offset}: {detail}"),
            Self::Wire {
                offset,
                path,
                detail,
            } => write!(f, "invalid wire field {path} at byte {offset}: {detail}"),
            Self::Codec { source } => write!(f, "invalid compressed frame: {source}"),
        }
    }
}
impl std::error::Error for BinaryFormatError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Codec { source } => Some(source.as_ref()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::WirePath;

    #[test]
    fn compact_path_preserves_schema_fields_and_full_width_terminals() {
        let mut path = WirePath::new();
        for (id, expected) in [(63, "snapshot"), (72, "cells")] {
            assert!(path.field(id));
            assert_eq!(WirePath::field_name(id), expected);
        }
        assert!(path.index(u64::MAX));
        for (id, expected) in [(2, "fg"), (17, "rgb"), (57, "red")] {
            assert!(path.field(id));
            assert_eq!(WirePath::field_name(id), expected);
        }
        assert!(path.tag(u32::MAX));
        assert_eq!(
            path.to_string(),
            "snapshot.cells[18446744073709551615].fg.rgb.red.#4294967295"
        );
        assert!(std::mem::size_of::<WirePath>() <= 32);
    }

    #[test]
    fn every_failed_append_preserves_the_previous_path() {
        let mut full = WirePath::new();
        for id in 0..5 {
            assert!(full.field(id));
        }
        let before = full;
        assert!(!full.field(6));
        assert_eq!(full, before);

        let mut indexed = WirePath::new();
        let before = indexed;
        assert!(!indexed.index(u64::MAX));
        assert_eq!(indexed, before);
        assert!(indexed.field(60));
        assert!(indexed.index(u64::MAX));
        let before = indexed;
        assert!(!indexed.index(0));
        assert_eq!(indexed, before);

        let mut tagged = WirePath::new();
        assert!(tagged.tag(u32::MAX));
        let before = tagged;
        assert!(!tagged.tag(0));
        assert_eq!(tagged, before);
    }

    #[test]
    fn repeated_sibling_transition_pops_index_before_parent_field() {
        let mut path = WirePath::new();
        assert!(path.field(63));
        assert!(path.index(u64::MAX));
        assert!(path.field(57));
        path.pop_field();
        path.pop_index();
        assert_eq!(path.to_string(), "snapshot");
        assert!(path.index(0));
        assert!(path.field(58));
        assert_eq!(path.to_string(), "snapshot[0].green");
    }

    #[test]
    fn final_repeated_field_uses_its_separate_index_slot() {
        let mut path = WirePath::new();
        for id in 0..5 {
            assert!(path.field(id));
        }
        assert!(path.index(u64::MAX));
        assert_eq!(
            path.to_string(),
            "text.width.fg.bg.bold[18446744073709551615]"
        );
    }
}

#[derive(Debug)]
pub(super) enum DecodeError {
    CodecAllocation {
        source: ZstdError,
    },
    ContextAllocation {
        site: &'static str,
    },
    Invariant {
        detail: &'static str,
    },
    Format(BinaryFormatError),
    Resource {
        resource: &'static str,
        claimed: Option<u64>,
        limit: u64,
        source: Option<Box<PngDecoderError>>,
    },
    Allocation {
        site: &'static str,
        requested: u64,
        source: TryReserveError,
    },
    UnsupportedContainerVersion(u16),
    UnsupportedCodec(u16),
    UnsupportedMessageVersion(u32),
    UnsupportedSchema(String),
    Domain(SampleError),
}
impl DecodeError {
    pub(super) fn resource(resource: &'static str, claimed: Option<u64>, limit: u64) -> Self {
        Self::Resource {
            resource,
            claimed,
            limit,
            source: None,
        }
    }
}

#[derive(Debug)]
pub(super) enum EncodeError {
    Limit {
        resource: &'static str,
        claimed: Option<u64>,
        limit: u64,
    },
    Allocation {
        site: &'static str,
        requested: u64,
        source: TryReserveError,
    },
    ContextAllocation {
        site: &'static str,
    },
    Codec {
        source: ZstdError,
    },
    Invariant {
        detail: &'static str,
    },
}
