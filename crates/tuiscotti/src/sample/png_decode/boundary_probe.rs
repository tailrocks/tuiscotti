use super::*;
use std::error::Error;

#[derive(Default)]
struct Ledger {
    fail: Option<&'static str>,
    live: u64,
    peak: u64,
    events: Vec<(&'static str, u64, u64)>,
}
impl PngBudget for Ledger {
    type Token = u64;
    fn charge(&mut self, site: &'static str, bytes: u64) -> Result<u64, SampleError> {
        self.events.push((site, bytes, self.live));
        if self.fail == Some(site) {
            return Err(SampleError::ResourceLimit {
                resource: site,
                claimed: Some(bytes),
                limit: 0,
                source: None,
            });
        }
        self.live += bytes;
        self.peak = self.peak.max(self.live);
        Ok(bytes)
    }
    fn release(&mut self, token: u64) -> Result<(), SampleError> {
        self.live -= token;
        Ok(())
    }
}
#[test]
fn actual_png_boundary_charges_and_injected_limits() -> Result<(), Box<dyn Error>> {
    let bytes = include_bytes!("../../../tests/pngsuite/basn6a08.png");
    let mut ledger = Ledger::default();
    let pixels = decode_png(bytes, &mut ledger)?;
    eprintln!(
        "PNG input={} dimensions={}x{} output={} peak ledger={} events={:?}",
        bytes.len(),
        pixels.width,
        pixels.height,
        pixels.rgba.len(),
        ledger.peak,
        ledger.events
    );
    assert_eq!(ledger.events.len(), 3);
    assert_eq!(ledger.events[2].2, ledger.events[0].1 + ledger.events[1].1);
    assert_eq!(ledger.live, pixels.rgba.len() as u64);
    for site in [
        "PNG header and metadata",
        "PNG rows and transform",
        "PNG RGBA output",
    ] {
        let mut failed = Ledger {
            fail: Some(site),
            ..Ledger::default()
        };
        let Err(error) = decode_png(bytes, &mut failed) else {
            return Err(format!("{site} did not fail").into());
        };
        assert!(matches!(&error, SampleError::ResourceLimit { resource, .. } if *resource == site));
        assert_eq!(crate::approve::sample_code(&error), "RESOURCE_LIMIT");
        eprintln!("injected {site}: events={:?}", failed.events);
    }
    Ok(())
}
