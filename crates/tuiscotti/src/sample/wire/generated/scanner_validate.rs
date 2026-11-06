fn check_varint(
    field: &FieldSpec,
    value: u64,
    offset: usize,
    path: WirePath,
) -> Result<(), WireFault> {
    check_small_integer(field, value, offset, path)?;
    check_dimensions(field, value, offset, path)?;
    check_enum_value(field, value, offset, path)?;
    check_schema_value(field, value, offset, path)
}
fn check_small_integer(
    field: &FieldSpec,
    value: u64,
    offset: usize,
    path: WirePath,
) -> Result<(), WireFault> {
    if field.constraints.contains(&"UINT32") && value > u64::from(u32::MAX) {
        return Err(fault(offset, path, "uint32 overflow"));
    }
    if field.constraints.contains(&"BOOL_0_1") && value > 1 {
        return Err(fault(offset, path, "bool is not zero or one"));
    }
    if field.constraints.contains(&"RGB_U8") && value > 255 {
        return Err(fault(offset, path, "RGB component exceeds 255"));
    }
    if field.constraints.contains(&"INDEX_U8") && value > 255 {
        return Err(fault(offset, path, "palette index exceeds 255"));
    }
    if field.constraints.contains(&"CELL_WIDTH_U16") && value > u64::from(u16::MAX) {
        return Err(fault(offset, path, "cell width exceeds u16"));
    }
    Ok(())
}
fn check_dimensions(
    field: &FieldSpec,
    value: u64,
    offset: usize,
    path: WirePath,
) -> Result<(), WireFault> {
    if field.constraints.contains(&"GRID_DIM_1_32768") && value == 0 {
        return Err(WireFault {
            offset,
            path,
            detail: "snapshot grid dimension is zero",
            kind: FaultKind::InvalidSnapshot,
        });
    }
    if field.constraints.contains(&"GRID_DIM_1_32768") && value > 32768 {
        return Err(resource_fault(
            offset,
            path,
            "grid dimension exceeds 32768",
            "grid dimension",
            Some(value),
            32768,
        ));
    }
    if field
        .constraints
        .contains(&"IMAGE_DIM_1_32768_RGBA_MAX_268435456")
        && value == 0
    {
        return Err(WireFault {
            offset,
            path,
            detail: "image dimensions must be nonzero",
            kind: FaultKind::DimensionMismatch,
        });
    }
    if field
        .constraints
        .contains(&"IMAGE_DIM_1_32768_RGBA_MAX_268435456")
        && value > 32768
    {
        return Err(resource_fault(
            offset,
            path,
            "image dimension exceeds 32768",
            "image dimension",
            Some(value),
            32768,
        ));
    }
    Ok(())
}
fn check_enum_value(
    field: &FieldSpec,
    value: u64,
    offset: usize,
    path: WirePath,
) -> Result<(), WireFault> {
    if field.constraints.contains(&"ENUM_NONZERO_KNOWN") {
        let number =
            i32::try_from(value).map_err(|_| fault(offset, path, "enum value exceeds i32"))?;
        if number == 0 || !enum_accepts(field.type_name.unwrap_or(""), number) {
            return Err(fault(offset, path, "unknown enum value"));
        }
    }
    Ok(())
}
fn check_schema_value(
    field: &FieldSpec,
    value: u64,
    offset: usize,
    path: WirePath,
) -> Result<(), WireFault> {
    if field.constraints.contains(&"SNAPSHOT_SCHEMA_EQ_1") && value != 1 {
        return Err(WireFault {
            offset,
            path,
            detail: "unsupported snapshot schema",
            kind: FaultKind::UnsupportedSchema(value),
        });
    }
    if field.constraints.contains(&"CONTRACT_VERSION_EXPECTED") && value != 1 {
        return Err(WireFault {
            offset,
            path,
            detail: "unsupported render contract version",
            kind: FaultKind::UnsupportedContract(
                u32::try_from(value)
                    .map_err(|_| fault(offset, path, "contract version exceeds u32"))?,
            ),
        });
    }
    if field.constraints.contains(&"USIZE_CHECKED") && usize::try_from(value).is_err() {
        return Err(fault(offset, path, "value does not fit usize"));
    }
    Ok(())
}
fn check_length(
    field: &FieldSpec,
    bytes: &[u8],
    offset: usize,
    path: WirePath,
) -> Result<(), WireFault> {
    if field.constraints.contains(&"UTF8") && std::str::from_utf8(bytes).is_err() {
        return Err(fault(offset, path, "invalid UTF-8"));
    }
    let limit = if field.constraints.contains(&"CELL_TEXT_UTF8_4096")
        || field.constraints.contains(&"LINK_UTF8_4096")
    {
        4096
    } else if field.constraints.contains(&"TITLE_UTF8_65536") {
        65536
    } else if field.constraints.contains(&"TEST_ID_GRAMMAR_240_64") {
        240
    } else {
        64 * 1024 * 1024
    };
    if bytes.len() > limit {
        return Err(resource_fault(
            offset,
            path,
            "field exceeds policy length",
            "field bytes",
            Some(bytes.len() as u64),
            limit as u64,
        ));
    }
    if field.constraints.contains(&"SAMPLE_SCHEMA_EXPECTED") && bytes != b"tuiscotti.sample.v1" {
        return Err(WireFault {
            offset,
            path,
            detail: "unsupported sample schema",
            kind: FaultKind::UnsupportedSampleSchema(offset, bytes.len()),
        });
    }
    if field.constraints.contains(&"TEST_ID_GRAMMAR_240_64") {
        check_test_id(bytes, offset, path)?;
    }
    if field.message == "Image"
        && (field.name == "png_sha256" || field.name == "rgba_sha256")
        && (bytes.len() != 64
            || !bytes
                .iter()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte)))
    {
        return Err(fault(
            offset,
            path,
            "digest must be 64 lowercase hexadecimal bytes",
        ));
    }
    Ok(())
}
