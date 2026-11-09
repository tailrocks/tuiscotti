mod allocation;
mod cell;
mod contract;
mod image;
mod snapshot;
mod snapshot_components;
#[cfg(test)]
mod tests;

use super::super::budget::{DecodeBudget, Phase};
use super::super::decode_error::DecodeError;
use super::super::{Sample, SampleSchema};
use super::generated::{
    DecodeSample, Gate as WireGate, ValidatedImage, ValidatedObservation, ValidatedRenderContract,
    ValidatedSample, ValidatedSnapshot,
};
use allocation::copied_string;
use contract::{gate, map_contract, map_observation};
use image::map_image;
use snapshot::map_snapshot;

#[derive(Debug, Default)]
enum FieldPresence<T> {
    #[default]
    Unset,
    Set(T),
}

impl<T> FieldPresence<T> {
    fn take(self, detail: &'static str) -> Result<T, DecodeError> {
        match self {
            Self::Unset => Err(DecodeError::Invariant { detail }),
            Self::Set(value) => Ok(value),
        }
    }
}

const _: () = assert!(std::mem::size_of::<tui_test::SnapshotCell>() <= 128);
const _: () = assert!(std::mem::size_of::<super::super::ContractFont>() <= 64);

struct SampleFields<'b> {
    test_id: Option<String>,
    gate: Option<super::super::Gate>,
    snapshot: Option<tui_test::TerminalSnapshot>,
    render_contract: FieldPresence<Option<super::super::RenderContract>>,
    image: FieldPresence<Option<super::super::SampleImage>>,
    observation: Option<super::super::Observation>,
    cell_capacity: usize,
    font_capacity: usize,
    budget: &'b mut DecodeBudget,
}

impl<'a> DecodeSample<'a> for SampleFields<'_> {
    type Error = DecodeError;
    fn schema(&mut self, value: &'a str) -> Result<(), Self::Error> {
        if value != super::super::SAMPLE_SCHEMA {
            return Err(DecodeError::Invariant {
                detail: "validated sample schema changed",
            });
        }
        Ok(())
    }
    fn test_id(&mut self, value: &'a str) -> Result<(), Self::Error> {
        self.test_id = Some(copied_string(value, "sample test ID")?);
        Ok(())
    }
    fn gate(&mut self, value: WireGate) -> Result<(), Self::Error> {
        self.gate = Some(gate(value));
        Ok(())
    }
    fn snapshot(&mut self, value: ValidatedSnapshot<'a>) -> Result<(), Self::Error> {
        self.snapshot = Some(map_snapshot(value, self.cell_capacity)?);
        Ok(())
    }
    fn render_contract(
        &mut self,
        value: Option<ValidatedRenderContract<'a>>,
    ) -> Result<(), Self::Error> {
        self.render_contract = FieldPresence::Set(
            value
                .map(|value| map_contract(value, self.font_capacity))
                .transpose()?,
        );
        Ok(())
    }
    fn image(&mut self, value: Option<ValidatedImage<'a>>) -> Result<(), Self::Error> {
        self.image = FieldPresence::Set(
            value
                .map(|value| map_image(value, self.budget))
                .transpose()?,
        );
        Ok(())
    }
    fn observation(&mut self, value: ValidatedObservation<'a>) -> Result<(), Self::Error> {
        self.observation = Some(map_observation(value)?);
        Ok(())
    }
}

pub(super) fn map_sample(
    root: ValidatedSample<'_>,
    budget: &mut DecodeBudget,
) -> Result<Sample, DecodeError> {
    let (payload_bytes, cells, fonts, png_bytes) = root.scan_claims();
    let domain_bytes = payload_bytes
        .checked_add(cells.checked_mul(128).ok_or_else(|| {
            DecodeError::resource("domain allocation bytes", None, budget.limit())
        })?)
        .and_then(|total| {
            fonts
                .checked_mul(64)
                .and_then(|font_bytes| total.checked_add(font_bytes))
        })
        .ok_or_else(|| DecodeError::resource("domain allocation bytes", None, budget.limit()))?;
    let base64_bytes = png_bytes
        .checked_add(2)
        .and_then(|length| length.checked_div(3))
        .and_then(|groups| groups.checked_mul(4))
        .ok_or_else(|| DecodeError::resource("base64 image bytes", None, budget.limit()))?;
    let domain_charge = budget.charge("mapped domain envelope", domain_bytes)?;
    let base64_charge = match budget.charge("base64 image envelope", base64_bytes) {
        Ok(charge) => charge,
        Err(error) => {
            budget.release(domain_charge)?;
            return Err(error);
        }
    };
    budget.retain_outputs(domain_charge, base64_charge)?;
    budget.set_phase(Phase::Mapping)?;
    let cell_capacity = usize::try_from(cells).map_err(|_| {
        DecodeError::resource("snapshot cell count", Some(cells), usize::MAX as u64)
    })?;
    let font_capacity = usize::try_from(fonts).map_err(|_| {
        DecodeError::resource("contract font count", Some(fonts), usize::MAX as u64)
    })?;
    let mut fields = SampleFields {
        test_id: None,
        gate: None,
        snapshot: None,
        render_contract: FieldPresence::Unset,
        image: FieldPresence::Unset,
        observation: None,
        cell_capacity,
        font_capacity,
        budget,
    };
    root.visit(&mut fields)?;
    let SampleFields {
        test_id,
        gate,
        snapshot,
        render_contract,
        image,
        observation,
        ..
    } = fields;
    let sample = Sample {
        schema: SampleSchema,
        test_id: test_id.ok_or(DecodeError::Invariant {
            detail: "validated sample omitted test ID",
        })?,
        gate: gate.ok_or(DecodeError::Invariant {
            detail: "validated sample omitted gate",
        })?,
        snapshot: snapshot.ok_or(DecodeError::Invariant {
            detail: "validated sample omitted snapshot",
        })?,
        render_contract: render_contract.take("validated sample omitted contract presence")?,
        image: image.take("validated sample omitted image presence")?,
        observation: observation.ok_or(DecodeError::Invariant {
            detail: "validated sample omitted observation",
        })?,
    };
    if let Some(image) = &sample.image {
        super::super::validation::check_grid_image(&sample.snapshot, image.width, image.height)
            .map_err(DecodeError::Domain)?;
    }
    Ok(sample)
}
