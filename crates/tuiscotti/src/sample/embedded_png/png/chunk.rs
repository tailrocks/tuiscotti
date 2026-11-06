//! Chunk types and functions
use core::fmt;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ChunkType(pub(crate) [u8; 4]);

// -- Critical chunks --

/// Image header
pub(crate) const IHDR: ChunkType = ChunkType(*b"IHDR");
/// Palette
pub(crate) const PLTE: ChunkType = ChunkType(*b"PLTE");
/// Image data
pub(crate) const IDAT: ChunkType = ChunkType(*b"IDAT");
/// Image trailer
pub(crate) const IEND: ChunkType = ChunkType(*b"IEND");

// -- Ancillary chunks --

/// Transparency
pub(crate) const T_RNS: ChunkType = ChunkType(*b"tRNS");
/// Background colour
pub(crate) const B_KGD: ChunkType = ChunkType(*b"bKGD");
/// Image last-modification time
pub(crate) const T_IME: ChunkType = ChunkType(*b"tIME");
/// Physical pixel dimensions
pub(crate) const P_HYS: ChunkType = ChunkType(*b"pHYs");
/// Source system's pixel chromaticities
pub(crate) const C_HRM: ChunkType = ChunkType(*b"cHRM");
/// Source system's gamma value
pub(crate) const G_AMA: ChunkType = ChunkType(*b"gAMA");
/// sRGB color space chunk
pub(crate) const S_RGB: ChunkType = ChunkType(*b"sRGB");
/// ICC profile chunk
pub(crate) const I_CCP: ChunkType = ChunkType(*b"iCCP");
/// Coding-independent code points for video signal type identification chunk
pub(crate) const C_ICP: ChunkType = ChunkType(*b"cICP");
/// Mastering Display Color Volume chunk
pub(crate) const M_DCV: ChunkType = ChunkType(*b"mDCV");
/// Content Light Level Information chunk
pub(crate) const C_LLI: ChunkType = ChunkType(*b"cLLI");
/// EXIF metadata chunk
pub(crate) const E_XIF: ChunkType = ChunkType(*b"eXIf");
/// Latin-1 uncompressed textual data
pub(crate) const T_EXT: ChunkType = ChunkType(*b"tEXt");
/// Latin-1 compressed textual data
pub(crate) const Z_TXT: ChunkType = ChunkType(*b"zTXt");
/// UTF-8 textual data
pub(crate) const I_TXT: ChunkType = ChunkType(*b"iTXt");
// Significant bits
pub(crate) const S_BIT: ChunkType = ChunkType(*b"sBIT");

// -- Extension chunks --

/// Animation control
pub(crate) const AC_TL: ChunkType = ChunkType(*b"acTL");
/// Frame control
pub(crate) const FC_TL: ChunkType = ChunkType(*b"fcTL");
/// Frame data
pub(crate) const FD_AT: ChunkType = ChunkType(*b"fdAT");

// -- Chunk type determination --

/// Returns true if the chunk is critical.
pub(crate) fn is_critical(ChunkType(type_): ChunkType) -> bool {
    type_[0] & 32 == 0
}

/// Returns true if the chunk is private.
pub(crate) fn is_private(ChunkType(type_): ChunkType) -> bool {
    type_[1] & 32 != 0
}

/// Checks whether the reserved bit of the chunk name is set.
/// If it is set the chunk name is invalid.
pub(crate) fn reserved_set(ChunkType(type_): ChunkType) -> bool {
    type_[2] & 32 != 0
}

/// Returns true if the chunk is safe to copy if unknown.
pub(crate) fn safe_to_copy(ChunkType(type_): ChunkType) -> bool {
    type_[3] & 32 != 0
}

impl fmt::Debug for ChunkType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        struct DebugType([u8; 4]);

        impl fmt::Debug for DebugType {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                for &c in &self.0[..] {
                    write!(f, "{}", char::from(c).escape_debug())?;
                }
                Ok(())
            }
        }

        f.debug_struct("ChunkType")
            .field("type", &DebugType(self.0))
            .field("critical", &is_critical(*self))
            .field("private", &is_private(*self))
            .field("reserved", &reserved_set(*self))
            .field("safecopy", &safe_to_copy(*self))
            .finish()
    }
}
