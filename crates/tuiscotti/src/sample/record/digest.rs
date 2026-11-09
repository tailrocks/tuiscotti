use sha2::{Digest as _, Sha256};

use super::{DynamicBudget, Gate, PngBudget, Sample, SampleError, copy_text};

const DIGITS: &[u8; 16] = b"0123456789abcdef";

struct HashWriter<'a>(&'a mut Sha256);

impl std::io::Write for HashWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Sample {
    #[doc = "Digest over the compared state only: gate, snapshot without its observation metadata, render contract, and decoded RGBA pixels.\n\n# Errors\n\nReturns [`SampleError`] when the embedded image fails to decode."]
    pub fn state_digest(&self) -> Result<String, SampleError> {
        let mut budget = DynamicBudget::bounded()?;
        let mut hasher = Sha256::new();
        hasher.update(b"tuiscotti-state-v1");
        hasher.update([u8::from(self.gate == Gate::StateAndPixels)]);
        crate::sample::digest::write_snapshot_projection(&self.snapshot, HashWriter(&mut hasher))
            .map_err(|source| SampleError::InvalidFormat { source })?;
        if let Some(contract) = &self.render_contract {
            crate::sample::digest::write_contract_projection(contract, HashWriter(&mut hasher))
                .map_err(|source| SampleError::InvalidFormat { source })?;
        }
        if let Some(image) = &self.image {
            let pixels = Self::decoded_pixels(image, &mut budget)?;
            hasher.update(&pixels.rgba);
            drop(pixels.rgba);
            budget.release(pixels.charge)?;
        }
        let digest = hasher.finalize();
        let mut hex = [0_u8; 64];
        for (index, byte) in digest.iter().copied().enumerate() {
            hex[index * 2] = DIGITS[usize::from(byte >> 4)];
            hex[index * 2 + 1] = DIGITS[usize::from(byte & 15)];
        }
        let text = std::str::from_utf8(&hex).map_err(|_| SampleError::DecodeInvariant {
            detail: "hex digest is not UTF-8",
        })?;
        copy_text(text, &mut budget, "state digest")
    }
}
