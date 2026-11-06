use super::helpers::*;

#[test]
fn absent_sample_and_snapshot_optionals_remain_absent() {
    let sample = state_only_sample();
    let actual = super::super::encode_sample(&sample).expect("fixture maps completely");
    assert_eq!(top_level_tags(&actual), [1, 2, 3, 4, 7]);
}
