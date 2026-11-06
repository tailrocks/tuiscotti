use super::*;

impl TEXtChunk {
    /// Constructs a new TEXtChunk.
    /// Not sure whether it should take &str or String.
    pub(crate) fn new(keyword: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            keyword: keyword.into(),
            text: text.into(),
        }
    }

    /// Decodes a slice of bytes to a String using Latin-1 decoding.
    /// The decoder runs in strict mode, and any decoding errors are passed along to the caller.
    pub(crate) fn decode(
        keyword_slice: &[u8],
        text_slice: &[u8],
        limits: &mut Limits,
    ) -> Result<Self, DecodingError> {
        if keyword_slice.is_empty() || keyword_slice.len() > 79 {
            return Err(TextDecodingError::InvalidKeywordSize.into());
        }

        Ok(Self {
            keyword: decode_iso_8859_1(keyword_slice, "PNG tEXt keyword", limits)?,
            text: decode_iso_8859_1(text_slice, "PNG tEXt value", limits)?,
        })
    }
}

#[cfg(test)]
impl EncodableTextChunk for TEXtChunk {
    /// Encodes TEXtChunk to a Writer. The keyword and text are separated by a byte of zeroes.
    fn encode<W: Write>(&self, w: &mut W) -> Result<(), EncodingError> {
        let mut data = encode_iso_8859_1(&self.keyword)?;

        if data.is_empty() || data.len() > 79 {
            return Err(TextEncodingError::InvalidKeywordSize.into());
        }

        data.push(0);

        encode_iso_8859_1_into(&mut data, &self.text)?;

        encoder::write_chunk(w, chunk::T_EXT, &data)
    }
}
