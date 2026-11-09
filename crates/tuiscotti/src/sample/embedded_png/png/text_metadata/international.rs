use super::*;

/// Struct encoding an iTXt chunk
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ITXtChunk {
    /// The keyword field. This needs to be between 1-79 bytes when encoded as Latin-1.
    pub(crate) keyword: String,
    /// Indicates whether the text will be (or was) compressed in the PNG.
    pub(crate) compressed: bool,
    /// A hyphen separated list of languages that the keyword is translated to. This is ASCII-7 encoded.
    pub(crate) language_tag: String,
    /// Translated keyword. This is UTF-8 encoded.
    pub(crate) translated_keyword: String,
    /// Text field of iTXt chunk. It is compressed by default, but can be uncompressed if necessary.
    text: OptCompressed,
}

use super::compressed::OptCompressed;
impl ITXtChunk {
    /// Constructs a new iTXt chunk. Leaves all but keyword and text to default values.
    pub(crate) fn new(keyword: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            keyword: keyword.into(),
            compressed: false,
            language_tag: "".to_string(),
            translated_keyword: "".to_string(),
            text: OptCompressed::Uncompressed(text.into()),
        }
    }

    pub(crate) fn decode(
        keyword_slice: &[u8],
        compression_flag: u8,
        compression_method: u8,
        language_tag_slice: &[u8],
        translated_keyword_slice: &[u8],
        text_slice: &[u8],
        limits: &mut Limits,
    ) -> Result<Self, DecodingError> {
        if keyword_slice.is_empty() || keyword_slice.len() > 79 {
            return Err(TextDecodingError::InvalidKeywordSize.into());
        }
        let keyword = decode_iso_8859_1(keyword_slice, "PNG iTXt keyword", limits)?;

        let compressed = match compression_flag {
            0 => false,
            1 => true,
            _ => return Err(TextDecodingError::InvalidCompressionFlag.into()),
        };

        if compressed && compression_method != 0 {
            return Err(TextDecodingError::InvalidCompressionMethod.into());
        }

        let language_tag = copy_text_string(
            decode_ascii(language_tag_slice)?,
            "PNG iTXt language tag",
            limits,
        )?;

        let translated_keyword = std::str::from_utf8(translated_keyword_slice)
            .map_err(|_| DecodingError::from(TextDecodingError::Unrepresentable))?;
        let translated_keyword =
            copy_text_string(translated_keyword, "PNG iTXt translated keyword", limits)?;
        let text = if compressed {
            OptCompressed::Compressed(copy_text_bytes(
                text_slice,
                "PNG iTXt compressed text",
                limits,
            )?)
        } else {
            let text = std::str::from_utf8(text_slice)
                .map_err(|_| DecodingError::from(TextDecodingError::Unrepresentable))?;
            OptCompressed::Uncompressed(copy_text_string(text, "PNG iTXt text", limits)?)
        };

        Ok(Self {
            keyword,
            compressed,
            language_tag,
            translated_keyword,
            text,
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
                        v, limit,
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
                self.text = OptCompressed::Uncompressed(
                    String::from_utf8(uncompressed_raw)
                        .map_err(|_| TextDecodingError::Unrepresentable)?,
                );
            }
            OptCompressed::Uncompressed(_) => {}
        };
        Ok(())
    }

    /// Decompresses the inner text, and returns it as a `String`.
    /// If decompression takes more than 2 MiB, try `decompress_text_with_limit` followed by this method.
    pub(crate) fn get_text(&self) -> Result<String, DecodingError> {
        match &self.text {
            OptCompressed::Compressed(v) => {
                let uncompressed_raw = crate::sample::embedded_png::fdeflate::decompress_to_vec(v)
                    .map_err(|_| DecodingError::from(TextDecodingError::InflationError))?;
                String::from_utf8(uncompressed_raw)
                    .map_err(|_| TextDecodingError::Unrepresentable.into())
            }
            OptCompressed::Uncompressed(s) => Ok(s.clone()),
        }
    }

    /// Compresses the inner text, mutating its own state.
    #[cfg(test)]
    pub(crate) fn compress_text(&mut self) -> Result<(), EncodingError> {
        match &self.text {
            OptCompressed::Uncompressed(s) => {
                let uncompressed_raw = s.as_bytes();
                let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
                encoder
                    .write_all(uncompressed_raw)
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
impl EncodableTextChunk for ITXtChunk {
    fn encode<W: Write>(&self, w: &mut W) -> Result<(), EncodingError> {
        // Keyword
        let mut data = encode_iso_8859_1(&self.keyword)?;

        if data.is_empty() || data.len() > 79 {
            return Err(TextEncodingError::InvalidKeywordSize.into());
        }

        // Null separator
        data.push(0);

        // Compression flag
        if self.compressed {
            data.push(1);
        } else {
            data.push(0);
        }

        // Compression method
        data.push(0);

        // Language tag
        if !self.language_tag.is_ascii() {
            return Err(EncodingError::from(TextEncodingError::Unrepresentable));
        }
        data.extend(self.language_tag.as_bytes());

        // Null separator
        data.push(0);

        // Translated keyword
        data.extend_from_slice(self.translated_keyword.as_bytes());

        // Null separator
        data.push(0);

        // Text
        if self.compressed {
            match &self.text {
                OptCompressed::Compressed(v) => {
                    data.extend_from_slice(&v[..]);
                }
                OptCompressed::Uncompressed(s) => {
                    let uncompressed_raw = s.as_bytes();
                    let mut encoder = ZlibEncoder::new(data, Compression::fast());
                    encoder
                        .write_all(uncompressed_raw)
                        .map_err(|_| EncodingError::from(TextEncodingError::CompressionError))?;
                    data = encoder
                        .finish()
                        .map_err(|_| EncodingError::from(TextEncodingError::CompressionError))?;
                }
            }
        } else {
            match &self.text {
                OptCompressed::Compressed(v) => {
                    let uncompressed_raw =
                        crate::sample::embedded_png::fdeflate::decompress_to_vec(v).map_err(
                            |_| EncodingError::from(TextEncodingError::CompressionError),
                        )?;
                    data.extend_from_slice(&uncompressed_raw[..]);
                }
                OptCompressed::Uncompressed(s) => {
                    data.extend_from_slice(s.as_bytes());
                }
            }
        }

        encoder::write_chunk(w, chunk::I_TXT, &data)
    }
}
