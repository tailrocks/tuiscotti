use super::*;

/// Struct representing a zTXt chunk
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ZTXtChunk {
    /// Keyword field of the tEXt chunk. Needs to be between 1-79 bytes when encoded as Latin-1.
    pub(crate) keyword: String,
    /// Text field of zTXt chunk. It is compressed by default, but can be uncompressed if necessary.
    text: OptCompressed,
}

impl ZTXtChunk {
    /// Creates a new ZTXt chunk.
    pub(crate) fn new(keyword: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            keyword: keyword.into(),
            text: OptCompressed::Uncompressed(text.into()),
        }
    }

    pub(crate) fn decode(
        keyword_slice: &[u8],
        compression_method: u8,
        text_slice: &[u8],
        limits: &mut Limits,
    ) -> Result<Self, DecodingError> {
        if keyword_slice.is_empty() || keyword_slice.len() > 79 {
            return Err(TextDecodingError::InvalidKeywordSize.into());
        }

        if compression_method != 0 {
            return Err(TextDecodingError::InvalidCompressionMethod.into());
        }

        Ok(Self {
            keyword: decode_iso_8859_1(keyword_slice, "PNG zTXt keyword", limits)?,
            text: OptCompressed::Compressed(copy_text_bytes(text_slice, "PNG zTXt value", limits)?),
        })
    }

    /// Decompresses the inner text, mutating its own state. Can only handle decompressed text up to `DECOMPRESSION_LIMIT` bytes.
    pub(crate) fn decompress_text(&mut self) -> Result<(), DecodingError> {
        self.decompress_text_with_limit(DECOMPRESSION_LIMIT)
    }

    /// Decompresses the inner text, mutating its own state. Can only handle decompressed text up to `limit` bytes.
    pub(crate) fn decompress_text_with_limit(&mut self, limit: usize) -> Result<(), DecodingError> {
        match &self.text {
            OptCompressed::Compressed(v) => {
                let uncompressed_raw =
                    match crate::sample::embedded_png::fdeflate::decompress_to_vec_bounded_unmetered(
                        &v[..],
                        limit,
                    ) {
                        Ok(s) => s,
                        Err(BoundedDecompressionError::OutputTooLarge { .. }) => {
                            return Err(DecodingError::from(
                                TextDecodingError::OutOfDecompressionSpace,
                            ));
                        }
                        Err(BoundedDecompressionError::Allocation {
                            site,
                            requested,
                            source,
                        }) => return Err(DecodingError::allocation(site, requested, source)),
                        Err(BoundedDecompressionError::LimitsExceeded) => {
                            return Err(DecodingError::LimitsExceeded);
                        }
                        Err(BoundedDecompressionError::DecompressionError { .. }) => {
                            return Err(DecodingError::from(TextDecodingError::InflationError));
                        }
                    };
                let mut limits = unmetered_text_limits();
                self.text = OptCompressed::Uncompressed(decode_iso_8859_1(
                    &uncompressed_raw,
                    "PNG zTXt decompressed text",
                    &mut limits,
                )?);
            }
            OptCompressed::Uncompressed(_) => {}
        };
        Ok(())
    }

    /// Decompresses the inner text, and returns it as a `String`.
    /// If decompression uses more the 2MiB, first call decompress with limit, and then this method.
    pub(crate) fn get_text(&self) -> Result<String, DecodingError> {
        match &self.text {
            OptCompressed::Compressed(v) => {
                let uncompressed_raw = crate::sample::embedded_png::fdeflate::decompress_to_vec(v)
                    .map_err(|_| DecodingError::from(TextDecodingError::InflationError))?;
                let mut limits = unmetered_text_limits();
                decode_iso_8859_1(&uncompressed_raw, "PNG zTXt returned text", &mut limits)
            }
            OptCompressed::Uncompressed(s) => Ok(s.clone()),
        }
    }

    /// Compresses the inner text, mutating its own state.
    #[cfg(test)]
    pub(crate) fn compress_text(&mut self) -> Result<(), EncodingError> {
        match &self.text {
            OptCompressed::Uncompressed(s) => {
                let uncompressed_raw = encode_iso_8859_1(s)?;
                let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
                encoder
                    .write_all(&uncompressed_raw)
                    .map_err(|_| EncodingError::from(TextEncodingError::CompressionError))?;
                self.text = OptCompressed::Compressed(
                    encoder
                        .finish()
                        .map_err(|_| EncodingError::from(TextEncodingError::CompressionError))?,
                );
            }
            OptCompressed::Compressed(_) => {}
        }

        Ok(())
    }
}

#[cfg(test)]
impl EncodableTextChunk for ZTXtChunk {
    fn encode<W: Write>(&self, w: &mut W) -> Result<(), EncodingError> {
        let mut data = encode_iso_8859_1(&self.keyword)?;

        if data.is_empty() || data.len() > 79 {
            return Err(TextEncodingError::InvalidKeywordSize.into());
        }

        // Null separator
        data.push(0);

        // Compression method: the only valid value is 0, as of 2021.
        data.push(0);

        match &self.text {
            OptCompressed::Compressed(v) => {
                data.extend_from_slice(&v[..]);
            }
            OptCompressed::Uncompressed(s) => {
                // This code may have a bug. Check for correctness.
                let uncompressed_raw = encode_iso_8859_1(s)?;
                let mut encoder = ZlibEncoder::new(data, Compression::fast());
                encoder
                    .write_all(&uncompressed_raw)
                    .map_err(|_| EncodingError::from(TextEncodingError::CompressionError))?;
                data = encoder
                    .finish()
                    .map_err(|_| EncodingError::from(TextEncodingError::CompressionError))?;
            }
        };

        encoder::write_chunk(w, chunk::Z_TXT, &data)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum OptCompressed {
    /// Compressed version of text field. Can be at most 2GB.
    Compressed(Vec<u8>),
    /// Uncompressed text field.
    Uncompressed(String),
}
