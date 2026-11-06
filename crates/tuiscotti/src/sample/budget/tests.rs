use super::{DecodeBudget, ERROR_FIXED, Phase};
use std::error::Error;

fn checked<T>(result: Result<T, super::DecodeError>) -> Result<T, Box<dyn Error>> {
    result.map_err(|error| format!("{error:?}").into())
}

#[test]
fn output_finish_rejects_foreign_ledgers_without_mutating_totals() -> Result<(), Box<dyn Error>> {
    let mut first = checked(DecodeBudget::new())?;
    let mut second = checked(DecodeBudget::new())?;
    let domain = checked(first.charge("domain", 17))?;
    let base64 = checked(second.charge("base64", 9))?;
    let before = first.total();
    assert!(first.retain_outputs(domain, base64).is_err());
    assert_eq!(first.total(), before);
    assert_eq!(first.charges, 2);
    Ok(())
}

#[test]
fn output_finish_waits_for_all_non_output_charges_and_preserves_output_total()
-> Result<(), Box<dyn Error>> {
    let mut budget = checked(DecodeBudget::new())?;
    checked(budget.reserve_message_error_envelope(17))?;
    checked(budget.set_phase(Phase::Codec))?;
    checked(budget.set_phase(Phase::Scan))?;
    let domain = checked(budget.charge("domain", 17))?;
    let base64 = checked(budget.charge("base64", 9))?;
    checked(budget.retain_outputs(domain, base64))?;
    let scratch = checked(budget.charge("scratch", 5))?;
    let retained_total = budget.total() - 5 - ERROR_FIXED - 32 * 17;
    assert!(budget.finish_output().is_err());
    assert_eq!(budget.total(), retained_total + 5 + ERROR_FIXED + 32 * 17);
    checked(budget.release(scratch))?;
    assert_eq!(budget.total(), retained_total + ERROR_FIXED + 32 * 17);
    checked(budget.set_phase(Phase::Mapping))?;
    checked(budget.finish_output())?;
    assert_eq!(budget.total(), retained_total);
    assert!(budget.charge("late", 1).is_err());
    assert!(budget.finish_output().is_err());
    Ok(())
}

#[test]
fn error_envelope_is_precharged_in_peak_and_near_limit_claim_is_exact() -> Result<(), Box<dyn Error>>
{
    let mut budget = checked(DecodeBudget::new())?;
    let identity = std::mem::size_of::<super::IdentityLayout>() as u64;
    assert_eq!(budget.total(), identity + ERROR_FIXED);
    assert_eq!(budget.peak(), budget.total());

    checked(budget.reserve_message_error_envelope(17))?;
    assert_eq!(budget.total(), identity + ERROR_FIXED + 32 * 17);
    assert_eq!(budget.peak(), budget.total());
    let remaining = budget.limit() - budget.total();
    let _near = checked(budget.charge("near envelope boundary", remaining))?;
    let before = budget.total();
    let Err(error) = budget.charge("one byte over", 1) else {
        return Err("charge beyond the decoder budget unexpectedly succeeded".into());
    };
    assert!(matches!(
        error,
        super::DecodeError::Resource { claimed: Some(claimed), limit, .. }
            if claimed == before + 1 && limit == budget.limit()
    ));
    assert_eq!(budget.total(), before);
    Ok(())
}

#[test]
fn error_envelope_peak_includes_input_and_message_lifetimes() -> Result<(), Box<dyn Error>> {
    let mut budget = checked(DecodeBudget::new())?;
    checked(budget.reserve_message_error_envelope(100))?;
    let input = checked(budget.charge("input", 19))?;
    let message = checked(budget.charge("message", 100))?;
    let expected =
        std::mem::size_of::<super::IdentityLayout>() as u64 + ERROR_FIXED + 32 * 100 + 19 + 100;
    assert_eq!(budget.total(), expected);
    assert_eq!(budget.peak(), expected);
    checked(budget.release(message))?;
    checked(budget.release(input))?;
    assert_eq!(budget.peak(), expected);
    Ok(())
}

#[test]
fn release_rejects_foreign_lease_without_mutating_either_ledger() -> Result<(), Box<dyn Error>> {
    let mut first = checked(DecodeBudget::new())?;
    let mut second = checked(DecodeBudget::new())?;
    let lease = checked(second.charge("foreign", 23))?;
    let first_before = first.total();
    let second_before = second.total();
    assert!(first.release(lease).is_err());
    assert_eq!(first.total(), first_before);
    assert_eq!(second.total(), second_before);
    Ok(())
}
