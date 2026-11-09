/// Decoder configuration options
#[derive(Clone)]
pub(crate) struct DecodeOptions {
    pub(super) ignore_adler32: bool,
    pub(super) ignore_crc: bool,
    pub(super) ignore_text_chunk: bool,
    pub(super) ignore_iccp_chunk: bool,
    pub(super) skip_ancillary_crc_failures: bool,
}

impl Default for DecodeOptions {
    fn default() -> Self {
        Self {
            ignore_adler32: true,
            ignore_crc: false,
            ignore_text_chunk: false,
            ignore_iccp_chunk: false,
            skip_ancillary_crc_failures: true,
        }
    }
}

impl DecodeOptions {
    /// When set, the decoder will not compute and verify the Adler-32 checksum.
    ///
    /// Defaults to `true`.
    pub(crate) fn set_ignore_adler32(&mut self, ignore_adler32: bool) {
        self.ignore_adler32 = ignore_adler32;
    }

    /// When set, the decoder will not compute and verify the CRC code.
    ///
    /// Defaults to `false`.
    pub(crate) fn set_ignore_crc(&mut self, ignore_crc: bool) {
        self.ignore_crc = ignore_crc;
    }

    /// Flag to ignore computing and verifying the Adler-32 checksum and CRC
    /// code.
    pub(crate) fn set_ignore_checksums(&mut self, ignore_checksums: bool) {
        self.ignore_adler32 = ignore_checksums;
        self.ignore_crc = ignore_checksums;
    }

    /// Ignore text chunks while decoding.
    ///
    /// Defaults to `false`.
    pub(crate) fn set_ignore_text_chunk(&mut self, ignore_text_chunk: bool) {
        self.ignore_text_chunk = ignore_text_chunk;
    }

    /// Ignore ICCP chunks while decoding.
    ///
    /// Defaults to `false`.
    pub(crate) fn set_ignore_iccp_chunk(&mut self, ignore_iccp_chunk: bool) {
        self.ignore_iccp_chunk = ignore_iccp_chunk;
    }

    /// Ignore ancillary chunks if CRC fails
    ///
    /// Defaults to `true`
    pub(crate) fn set_skip_ancillary_crc_failures(&mut self, skip_ancillary_crc_failures: bool) {
        self.skip_ancillary_crc_failures = skip_ancillary_crc_failures;
    }
}
