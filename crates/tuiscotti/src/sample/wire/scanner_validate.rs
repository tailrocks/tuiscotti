fn check_varint(
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
    if field.constraints.contains(&"ENUM_NONZERO_KNOWN") {
        let number =
            i32::try_from(value).map_err(|_| fault(offset, path, "enum value exceeds i32"))?;
        if number == 0 || !enum_accepts(field.type_name.unwrap_or(""), number) {
            return Err(fault(offset, path, "unknown enum value"));
        }
    }
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
    if field.message == "Image" && (field.name == "png_sha256" || field.name == "rgba_sha256") {
        if bytes.len() != 64
            || !bytes
                .iter()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
        {
            return Err(fault(
                offset,
                path,
                "digest must be 64 lowercase hexadecimal bytes",
            ));
        }
    }
    Ok(())
}
fn check_test_id(bytes: &[u8], offset: usize, path: WirePath) -> Result<(), WireFault> {
    if bytes.is_empty() || bytes[0] == b'/' || bytes.contains(&b'\\') || !bytes.is_ascii() {
        return Err(fault(offset, path, "invalid test ID"));
    }
    let mut start = 0usize;
    for end in 0..=bytes.len() {
        if end != bytes.len() && bytes[end] != b'/' {
            continue;
        }
        let segment = &bytes[start..end];
        if segment.is_empty() || segment.len() > 64 || segment == b"." || segment == b".." {
            return Err(fault(offset, path, "invalid test ID segment"));
        }
        if !segment.iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'_' || *byte == b'-'
        }) {
            return Err(fault(offset, path, "invalid test ID segment shape"));
        }
        let stem = segment
            .split(|byte| *byte == b'.')
            .next()
            .unwrap_or(segment);
        if matches!(stem, b"con" | b"prn" | b"aux" | b"nul")
            || (stem.len() == 4
                && (&stem[..3] == b"com" || &stem[..3] == b"lpt")
                && matches!(stem[3], b'1'..=b'9'))
        {
            return Err(fault(offset, path, "reserved test ID segment"));
        }
        start = end + 1;
    }
    Ok(())
}
fn finish_frame(frame: &mut Frame<'_>, path: WirePath) -> Result<(), WireFault> {
    let fields = fields_for(frame.kind)
        .ok_or_else(|| fault(frame.span.start, path, "unknown message kind"))?;
    for (index, field) in fields.iter().enumerate() {
        if field.required && frame.seen & (1u64 << index) == 0 {
            let mut p = path;
            let _ = p.field(field.name);
            return Err(fault(frame.span.end, p, "missing required field"));
        }
    }
    if fields.iter().any(|field| field.oneof.is_some()) && frame.oneofs != 1 {
        return Err(fault(
            frame.span.end,
            path,
            "oneof must select exactly one branch",
        ));
    }
    for (index, field) in fields.iter().enumerate() {
        if let Some(expected) = repeated_exact_count(frame.kind, field.name) {
            if frame.repeated[index] != expected {
                let mut field_path = path;
                let _ = field_path.field(field.name);
                return Err(WireFault {
                    offset: frame.span.end,
                    path: field_path,
                    detail: "repeated field count differs from exact policy",
                    kind: FaultKind::InvalidSnapshot,
                });
            }
        }
    }
    if frame.kind == "Snapshot" {
        let cells = frame.repeated[5];
        let grid = frame.values[1]
            .checked_mul(frame.values[2])
            .ok_or_else(|| {
                resource_fault(
                    frame.span.start,
                    path,
                    "grid product overflow",
                    "grid cells",
                    None,
                    1_000_000,
                )
            })?;
        if grid > 1_000_000 {
            return Err(resource_fault(
                frame.span.end,
                path,
                "grid cells exceed 1000000",
                "grid cells",
                Some(grid),
                1_000_000,
            ));
        }
        if cells != grid {
            return Err(WireFault {
                offset: frame.span.end,
                path,
                detail: "grid cell count mismatch",
                kind: FaultKind::InvalidSnapshot,
            });
        }
        frame.summary.grid = Some((frame.values[1] as u32, frame.values[2] as u32));
    }
    if frame.kind == "Image" {
        let rgba = frame.values[1]
            .checked_mul(frame.values[2])
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or_else(|| {
                resource_fault(
                    frame.span.start,
                    path,
                    "RGBA size overflow",
                    "decoded RGBA bytes",
                    None,
                    268_435_456,
                )
            })?;
        if rgba > 268_435_456 {
            return Err(resource_fault(
                frame.span.end,
                path,
                "RGBA bytes exceed limit",
                "decoded RGBA bytes",
                Some(rgba),
                268_435_456,
            ));
        }
        frame.summary.image = Some((frame.values[1] as u32, frame.values[2] as u32));
    }
    if frame.kind == "Sample" {
        let gate = frame.values[2];
        let contract = frame.seen & (1 << 4) != 0;
        let image = frame.seen & (1 << 5) != 0;
        if (gate == 1 && (contract || image)) || (gate == 2 && (!contract || !image)) {
            return Err(WireFault {
                offset: frame.span.end,
                path,
                detail: "gate evidence pair mismatch",
                kind: FaultKind::GateMismatch,
            });
        }
        let (id_start, id_end) = frame
            .summary
            .test_id
            .ok_or_else(|| fault(frame.span.end, path, "missing test ID summary"))?;
        let id = &frame.span.bytes[id_start..id_end];
        if gate == 1
            && id
                .rsplit(|byte| *byte == b'/')
                .next()
                .is_some_and(|tail| tail.starts_with(b"pixels-"))
        {
            return Err(WireFault {
                offset: id_start,
                path,
                detail: "state test ID has a pixels suffix",
                kind: FaultKind::GateMismatch,
            });
        }
        if gate == 2 {
            let (platform_start, platform_end) = frame
                .summary
                .platform_key
                .ok_or_else(|| fault(frame.span.end, path, "missing platform key summary"))?;
            let platform = &frame.span.bytes[platform_start..platform_end];
            let suffix_len = platform.len().checked_add(8).ok_or_else(|| {
                resource_fault(
                    id_start,
                    path,
                    "pixels test ID suffix overflow",
                    "test ID suffix",
                    None,
                    u64::MAX,
                )
            })?;
            let tail_start = id.len().checked_sub(suffix_len).ok_or_else(|| WireFault {
                offset: id_start,
                path,
                detail: "pixels test ID suffix mismatch",
                kind: FaultKind::GateMismatch,
            })?;
            if &id[tail_start..tail_start + 8] != b"/pixels-" || &id[tail_start + 8..] != platform {
                return Err(WireFault {
                    offset: id_start,
                    path,
                    detail: "pixels test ID suffix mismatch",
                    kind: FaultKind::GateMismatch,
                });
            }
        }
        if image {
            if let (
                Some((cols, rows)),
                Some((image_width, image_height)),
                Some((cell_width, cell_height)),
            ) = (frame.summary.grid, frame.summary.image, image_grid_scale())
            {
                let expected_width = cols.checked_mul(cell_width).ok_or_else(|| {
                    resource_fault(
                        frame.span.end,
                        path,
                        "grid image width overflow",
                        "expected image width",
                        None,
                        u64::from(u32::MAX),
                    )
                })?;
                let expected_height = rows.checked_mul(cell_height).ok_or_else(|| {
                    resource_fault(
                        frame.span.end,
                        path,
                        "grid image height overflow",
                        "expected image height",
                        None,
                        u64::from(u32::MAX),
                    )
                })?;
                if (image_width, image_height) != (expected_width, expected_height) {
                    return Err(WireFault {
                        offset: frame.span.end,
                        path,
                        detail: "image pixel dimensions differ from rendered snapshot grid",
                        kind: FaultKind::DimensionMismatch,
                    });
                }
            }
        }
        if frame.summary.c > 1_000_000 {
            return Err(resource_fault(
                frame.span.end,
                path,
                "aggregate cell count exceeds limit",
                "cells",
                Some(frame.summary.c),
                1_000_000,
            ));
        }
        let font_limit = (frame.span.end - frame.span.start) as u64 / 2;
        if frame.summary.f > font_limit {
            return Err(resource_fault(
                frame.span.end,
                path,
                "aggregate font count exceeds limit",
                "fonts",
                Some(frame.summary.f),
                font_limit,
            ));
        }
    }
    Ok(())
}
