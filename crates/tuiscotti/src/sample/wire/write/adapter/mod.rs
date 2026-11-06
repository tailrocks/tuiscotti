use super::super::super::{Gate, SAMPLE_SCHEMA, Sample, SampleError};
use super::super::generated::{EncodeSample, Gate as WireGate};

mod render;
mod snapshot;

use render::{DecodedImage, ImageAdapter, ObservationAdapter, RenderContractAdapter};
use snapshot::SnapshotAdapter;

pub(super) struct SampleAdapter<'a> {
    sample: &'a Sample,
    image: Option<DecodedImage<'a>>,
}

impl<'a> SampleAdapter<'a> {
    pub(super) fn new(sample: &'a Sample) -> Result<Self, SampleError> {
        let image = sample.image().map(DecodedImage::new).transpose()?;
        Ok(Self { sample, image })
    }
}

impl EncodeSample for SampleAdapter<'_> {
    fn schema(&self) -> &str {
        SAMPLE_SCHEMA
    }
    fn test_id(&self) -> &str {
        self.sample.test_id()
    }
    fn gate(&self) -> WireGate {
        match self.sample.gate() {
            Gate::State => WireGate::State,
            Gate::StateAndPixels => WireGate::StateAndPixels,
        }
    }
    type Snapshot<'a>
        = SnapshotAdapter<'a>
    where
        Self: 'a;
    fn snapshot(&self) -> Self::Snapshot<'_> {
        SnapshotAdapter(self.sample.snapshot())
    }
    type RenderContract<'a>
        = RenderContractAdapter<'a>
    where
        Self: 'a;
    fn render_contract(&self) -> Option<Self::RenderContract<'_>> {
        self.sample.render_contract().map(RenderContractAdapter)
    }
    type Image<'a>
        = ImageAdapter<'a>
    where
        Self: 'a;
    fn image(&self) -> Option<Self::Image<'_>> {
        self.image.as_ref().map(|image| ImageAdapter {
            image: image.image,
            png: &image.png,
        })
    }
    type Observation<'a>
        = ObservationAdapter<'a>
    where
        Self: 'a;
    fn observation(&self) -> Self::Observation<'_> {
        ObservationAdapter(self.sample.observation())
    }
}
