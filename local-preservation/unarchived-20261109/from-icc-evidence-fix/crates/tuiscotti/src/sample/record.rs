use base64::Engine as _;
use tui_test::TerminalSnapshot;

use crate::render_profile::{Profile, platform_key};
use crate::report::atomic_write;

use super::hash::sha256_hex;
use super::image::decode_png_bounded;
use super::png_decode::{
    DynamicBudget, DynamicCharge, PngBudget, PngPixels, copy_text, decode_base64, decode_png,
};
use super::validation::{check_grid_image, check_grid_limits, check_image_dims, validate_test_id};
use super::{
    CONTRACT_VERSION, Gate, ImageFormat, MAX_CELL_TEXT_BYTES, Observation, RenderContract,
    SampleError, SampleImage, SampleSchema,
};

#[derive(Debug, Clone, PartialEq)]
#[doc = "One versioned visual sample: a snapshot bound to its visual evidence."]
pub struct Sample {
    #[doc = "The schema identifier (always `SAMPLE_SCHEMA`)."]
    pub(super) schema: SampleSchema,
    #[doc = "The test ID this sample was constructed for."]
    pub(super) test_id: String,
    #[doc = "The comparison policy this sample carries."]
    pub(super) gate: Gate,
    #[doc = "The embedded snapshot."]
    pub(super) snapshot: TerminalSnapshot,
    #[doc = "The render contract, present only for `state-and-pixels`."]
    pub(super) render_contract: Option<RenderContract>,
    #[doc = "The image evidence, present only for `state-and-pixels`."]
    pub(super) image: Option<SampleImage>,
    #[doc = "The observation metadata, excluded from the compared state."]
    pub(super) observation: Observation,
}

impl Sample {
    #[doc = "Build a state sample: serialize the snapshot without rendering.\n\n# Errors\n\nReturns [`SampleError`] when the test ID breaks its shape rules."]
    pub fn state(
        test_id: &str,
        snapshot: TerminalSnapshot,
        observation: Observation,
    ) -> Result<Self, SampleError> {
        validate_test_id(test_id, Gate::State, platform_key().as_str())?;
        Ok(Self {
            schema: SampleSchema,
            test_id: test_id.to_string(),
            gate: Gate::State,
            snapshot,
            render_contract: None,
            image: None,
            observation,
        })
    }

    #[doc = "Build a state-and-pixels sample: render the immutable snapshot through the profile, then validate the image before returning.\n\n# Errors\n\nReturns [`SampleError`] when the test ID breaks its shape rules, when the upstream renderer refuses the snapshot, or when the returned image fails validation."]
    pub fn with_pixels(
        test_id: &str,
        snapshot: &TerminalSnapshot,
        observation: Observation,
        profile: &Profile,
    ) -> Result<Self, SampleError> {
        validate_test_id(test_id, Gate::StateAndPixels, platform_key().as_str())?;
        let rendered = tui_test::render_snapshot_png(snapshot, &profile.render_options())?;
        let contract = RenderContract::from_rendered(profile, &rendered)?;
        let png = rendered.png().to_vec();
        let (width, height, pixels) = decode_png_bounded(&png)?;
        if (width, height) != (rendered.width(), rendered.height()) {
            return Err(SampleError::DimensionMismatch {
                detail: format!(
                    "renderer reported {}x{} but the PNG decodes to {}x{}",
                    rendered.width(),
                    rendered.height(),
                    width,
                    height
                ),
            });
        }
        check_grid_image(snapshot, width, height)?;
        let image = SampleImage {
            format: ImageFormat::Png,
            width,
            height,
            png_sha256: sha256_hex(&png),
            rgba_sha256: sha256_hex(&pixels),
            png_base64: base64::engine::general_purpose::STANDARD.encode(&png),
        };
        let sample = Self {
            schema: SampleSchema,
            test_id: test_id.to_string(),
            gate: Gate::StateAndPixels,
            snapshot: snapshot.clone(),
            render_contract: Some(contract),
            image: Some(image),
            observation,
        };
        sample.validate_structure()?;
        Ok(sample)
    }

    #[doc = "The schema identifier (always `SAMPLE_SCHEMA`)."]
    #[must_use]
    pub const fn schema(&self) -> SampleSchema {
        self.schema
    }

    #[doc = "The test ID this sample was constructed for."]
    #[must_use]
    pub fn test_id(&self) -> &str {
        &self.test_id
    }

    #[doc = "The comparison policy this sample carries."]
    #[must_use]
    pub const fn gate(&self) -> Gate {
        self.gate
    }

    #[doc = "The embedded snapshot."]
    #[must_use]
    pub fn snapshot(&self) -> &TerminalSnapshot {
        &self.snapshot
    }

    #[doc = "The render contract, present only for `state-and-pixels`."]
    #[must_use]
    pub fn render_contract(&self) -> Option<&RenderContract> {
        self.render_contract.as_ref()
    }

    #[doc = "The image evidence, present only for `state-and-pixels`."]
    #[must_use]
    pub fn image(&self) -> Option<&SampleImage> {
        self.image.as_ref()
    }

    #[doc = "The observation metadata, excluded from the compared state."]
    #[must_use]
    pub const fn observation(&self) -> &Observation {
        &self.observation
    }

    #[doc = "Serialize to canonical file bytes: declaration field order, stable upstream snapshot order, row-major cells.\n\n# Errors\n\nReturns [`SampleError`] when the sample fails validation, when serialization fails, or when the bytes pass the file limit."]
    pub fn to_bytes(&self) -> Result<Vec<u8>, SampleError> {
        self.validate_structure()?;
        self.validate_image_bytes()?;
        // Trial-only setting: level 3 is unmeasured and must not ship.
        super::wire::encode_sample(self, 3)
    }

    #[doc = "Parse and validate a binary sample.\n\n# Errors\n\nReturns [`SampleError`] for rejected input."]
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SampleError> {
        super::wire::decode_sample(bytes).map_err(Into::into)
    }

    pub(crate) fn from_owned_buffer(bytes: &Vec<u8>) -> Result<Self, SampleError> {
        super::wire::decode_sample_with_input_allocation(bytes, bytes.capacity() as u64)
            .map_err(Into::into)
    }

    #[doc = "Read a sample file with a bounded read: bytes past the file limit are never loaded.\n\n# Errors\n\nReturns [`SampleError`] when the file is unreadable, too large, or fails [`Sample::from_bytes`] validation."]
    pub fn from_file(path: &std::path::Path) -> Result<Self, SampleError> {
        super::wire::decode_file(path)
    }

    #[doc = "Serialize and write one sample file through one atomic file write: a temporary file in the destination directory, flushed and closed before one atomic rename.\n\n# Errors\n\nReturns [`SampleError`] when serialization fails, when the sample path has no parent directory or file name, or when the write fails. A partial write is a failure, never a success value."]
    pub fn save(&self, path: &std::path::Path) -> Result<(), SampleError> {
        let bytes = self.to_bytes()?;
        let invalid = |what: &str| SampleError::Io {
            source: std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("sample path {} has {what}", path.display()),
            ),
        };
        let dir = path
            .parent()
            .ok_or_else(|| invalid("no parent directory"))?;
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| invalid("no file name"))?;
        atomic_write(dir, name, &bytes).map_err(|source| SampleError::Io { source })?;
        Ok(())
    }

    #[doc = "The candidate ID: SHA-256 over the sample file bytes."]
    #[must_use]
    pub fn candidate_id(bytes: &[u8]) -> String {
        sha256_hex(bytes)
    }

    #[doc = "Structural validation without font resolution: gate pairing, test ID, contract version, grid limits, and cell text limits.\n\n# Errors\n\nReturns [`SampleError`] for every structural rejection."]
    pub fn validate_structure(&self) -> Result<(), SampleError> {
        let current_platform;
        let pixels_platform = if let Some(contract) = &self.render_contract {
            contract.platform_key.as_str()
        } else {
            current_platform = platform_key();
            current_platform.as_str()
        };
        validate_test_id(&self.test_id, self.gate, pixels_platform)?;
        match self.gate {
            Gate::State => {
                if self.render_contract.is_some() || self.image.is_some() {
                    return Err(SampleError::GateMismatch {
                        detail: "state gate requires null render contract and image".to_string(),
                    });
                }
            }
            Gate::StateAndPixels => {
                let contract =
                    self.render_contract
                        .as_ref()
                        .ok_or_else(|| SampleError::GateMismatch {
                            detail: "state-and-pixels gate requires a render contract".to_string(),
                        })?;
                let image = self
                    .image
                    .as_ref()
                    .ok_or_else(|| SampleError::GateMismatch {
                        detail: "state-and-pixels gate requires an image".to_string(),
                    })?;
                if contract.contract_version != CONTRACT_VERSION {
                    return Err(SampleError::UnsupportedContract {
                        version: contract.contract_version,
                    });
                }
                if image.format != ImageFormat::Png {
                    return Err(SampleError::PngInvalid {
                        detail: "only PNG images are supported".to_string(),
                    });
                }
            }
        }
        check_grid_limits(
            self.snapshot.cols(),
            self.snapshot.rows(),
            self.snapshot.cells().len(),
        )?;
        for cell in self.snapshot.cells() {
            if cell.text().len() > MAX_CELL_TEXT_BYTES {
                return Err(SampleError::TooLarge {
                    what: "cell text",
                    bytes: cell.text().len() as u64,
                    limit: MAX_CELL_TEXT_BYTES as u64,
                });
            }
        }
        Ok(())
    }

    fn decoded_pixels(
        image: &SampleImage,
        budget: &mut DynamicBudget,
    ) -> Result<PngPixels<DynamicCharge>, SampleError> {
        check_image_dims(image.width, image.height)?;
        let (raw, raw_charge) = decode_base64(&image.png_base64, budget)?;
        let pixels = decode_png(&raw, budget)?;
        drop(raw);
        budget.release(raw_charge)?;
        if (pixels.width, pixels.height) != (image.width, image.height) {
            let actual = (pixels.width, pixels.height);
            drop(pixels.rgba);
            budget.release(pixels.charge)?;
            return Err(SampleError::ImageDimensionsMismatch {
                claimed: (image.width, image.height),
                actual,
            });
        }
        Ok(pixels)
    }

    fn validate_image_bytes(&self) -> Result<(), SampleError> {
        let Some(image) = &self.image else {
            return Ok(());
        };
        let raw = base64::engine::general_purpose::STANDARD
            .decode(image.png_base64.as_bytes())
            .map_err(|source| SampleError::Base64Invalid { source })?;
        let actual_png = sha256_hex(&raw);
        if actual_png != image.png_sha256 {
            return Err(SampleError::DigestMismatch {
                field: "png_sha256",
                expected: image.png_sha256.clone(),
                actual: actual_png,
            });
        }
        let mut budget = DynamicBudget::bounded()?;
        let pixels = Self::decoded_pixels(image, &mut budget)?;
        let actual_rgba = sha256_hex(&pixels.rgba);
        let (width, height) = (pixels.width, pixels.height);
        drop(pixels.rgba);
        budget.release(pixels.charge)?;
        if actual_rgba != image.rgba_sha256 {
            return Err(SampleError::DigestMismatch {
                field: "rgba_sha256",
                expected: image.rgba_sha256.clone(),
                actual: actual_rgba,
            });
        }
        check_grid_image(&self.snapshot, width, height)?;
        Ok(())
    }
}

mod digest;
