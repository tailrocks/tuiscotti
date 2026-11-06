fn scan(root: Span<'_>, root_kind: &'static str) -> Result<ScanSummary, WireFault> {
    let mut stack: [Option<Frame<'_>>; 32] = [None; 32];
    let mut depth = 1usize;
    stack[0] = Some(Frame {
        kind: root_kind,
        span: root,
        cursor: root.start,
        seen: 0,
        oneofs: 0,
        repeated: [0; 16],
        values: [0; 16],
        summary: ScanSummary::default(),
        indexed: false,
    });
    let mut path = WirePath::new();
    while depth > 0 {
        let mut pending_child = None;
        let top_index = depth - 1;
        let top = stack[top_index]
            .as_mut()
            .ok_or_else(|| fault(0, path, "missing scanner frame"))?;
        if top.cursor == top.span.end {
            finish_frame(top, path)?;
            let complete = stack[top_index]
                .take()
                .ok_or_else(|| fault(0, path, "missing completed frame"))?;
            depth -= 1;
            if depth == 0 {
                return Ok(complete.summary);
            }
            if complete.indexed {
                path.pop();
            }
            path.pop();
            let parent = stack[depth - 1]
                .as_mut()
                .ok_or_else(|| fault(0, path, "missing parent frame"))?;
            parent.summary.l = checked_sum(
                parent.summary.l,
                complete.summary.l,
                complete.span.end,
                path,
            )?;
            parent.summary.c = checked_sum(
                parent.summary.c,
                complete.summary.c,
                complete.span.end,
                path,
            )?;
            parent.summary.f = checked_sum(
                parent.summary.f,
                complete.summary.f,
                complete.span.end,
                path,
            )?;
            parent.summary.p = checked_sum(
                parent.summary.p,
                complete.summary.p,
                complete.span.end,
                path,
            )?;
            parent.summary.test_id = parent.summary.test_id.or(complete.summary.test_id);
            parent.summary.platform_key = parent
                .summary
                .platform_key
                .or(complete.summary.platform_key);
            parent.summary.grid = parent.summary.grid.or(complete.summary.grid);
            parent.summary.image = parent.summary.image.or(complete.summary.image);
            continue;
        }
        let fields =
            fields_for(top.kind).ok_or_else(|| fault(top.cursor, path, "unknown message kind"))?;
        let field_offset = top.cursor;
        let key = read_varint(top.span.bytes, &mut top.cursor, top.span.end, path)?;
        let tag =
            u32::try_from(key >> 3).map_err(|_| fault(field_offset, path, "field tag overflow"))?;
        let wire = (key & 7) as u8;
        if tag == 0 || tag > 536_870_911 {
            return Err(fault(field_offset, path, "invalid field tag"));
        }
        let field_index = fields
            .iter()
            .position(|field| field.tag == tag)
            .ok_or_else(|| {
                let mut unknown = path;
                let _ = unknown.tag(tag);
                fault(field_offset, unknown, "unknown field")
            })?;
        if field_index >= 16 {
            return Err(fault(
                field_offset,
                path,
                "generated field counter overflow",
            ));
        }
        if field_index >= 64 {
            return Err(fault(field_offset, path, "generated field mask overflow"));
        }
        let field = &fields[field_index];
        let mut field_path = path;
        if !field_path.field(field.name) {
            return Err(fault(field_offset, path, "field path exceeds bound"));
        }
        if field.repeated && !field_path.index(top.repeated[field_index]) {
            return Err(fault(field_offset, field_path, "field path exceeds bound"));
        }
        if !field.repeated && top.seen & (1u64 << field_index) != 0 {
            return Err(fault(field_offset, field_path, "duplicate singular field"));
        }
        top.seen |= 1u64 << field_index;
        if field.oneof.is_some() {
            if top.oneofs != 0 {
                return Err(fault(
                    field_offset,
                    field_path,
                    "multiple oneof alternatives",
                ));
            }
            top.oneofs = 1;
        }
        let expected_wire = match field.kind {
            WireKind::Varint => 0,
            WireKind::Fixed64 => 1,
            WireKind::LengthDelimited => 2,
            WireKind::Fixed32 => 5,
        };
        if wire != expected_wire {
            return Err(fault(field_offset, field_path, "wire type mismatch"));
        }
        if field.repeated {
            let next = checked_sum(top.repeated[field_index], 1, field_offset, field_path)?;
            if let Some(limit) = repeated_limit(top.kind, field.name) {
                if next > limit {
                    return Err(resource_fault(
                        field_offset,
                        field_path,
                        "palette exceeds 256 entries",
                        "palette entries",
                        Some(next),
                        limit,
                    ));
                }
            }
            top.repeated[field_index] = next;
        }
        match field.kind {
            WireKind::Varint => {
                let value = read_varint(top.span.bytes, &mut top.cursor, top.span.end, field_path)?;
                check_varint(field, value, field_offset, field_path)?;
                if tag <= 16 {
                    top.values[tag as usize - 1] = value;
                }
            }
            WireKind::Fixed64 | WireKind::Fixed32 => {
                let width = if field.kind == WireKind::Fixed64 {
                    8
                } else {
                    4
                };
                let finish = top
                    .cursor
                    .checked_add(width)
                    .ok_or_else(|| fault(top.cursor, field_path, "fixed field overflow"))?;
                if finish > top.span.end {
                    return Err(fault(top.cursor, field_path, "truncated fixed field"));
                }
                top.cursor = finish;
            }
            WireKind::LengthDelimited => {
                let (start, finish) =
                    read_length(top.span.bytes, &mut top.cursor, top.span.end, field_path)?;
                if field.type_name.is_some() {
                    let child_kind = field
                        .type_name
                        .ok_or_else(|| fault(start, field_path, "missing child kind"))?;
                    if depth == 32 {
                        return Err(resource_fault(
                            start,
                            field_path,
                            "message nesting exceeds 32",
                            "message nesting depth",
                            Some(33),
                            32,
                        ));
                    }
                    if child_kind == "Cell" {
                        top.summary.c = checked_sum(top.summary.c, 1, start, field_path)?;
                    }
                    if child_kind == "Font" {
                        top.summary.f = checked_sum(top.summary.f, 1, start, field_path)?;
                    }
                    let child = Frame {
                        kind: child_kind,
                        span: Span {
                            bytes: top.span.bytes,
                            start,
                            end: finish,
                        },
                        cursor: start,
                        seen: 0,
                        oneofs: 0,
                        repeated: [0; 16],
                        values: [0; 16],
                        summary: ScanSummary::default(),
                        indexed: field.repeated,
                    };
                    path = field_path;
                    pending_child = Some(child);
                } else {
                    check_length(field, &top.span.bytes[start..finish], start, field_path)?;
                    top.summary.l =
                        checked_sum(top.summary.l, (finish - start) as u64, start, field_path)?;
                    if field.message == "Image" && field.name == "png" {
                        top.summary.p =
                            checked_sum(top.summary.p, (finish - start) as u64, start, field_path)?;
                    }
                    if field.message == "Sample" && field.name == "test_id" {
                        top.summary.test_id = Some((start, finish));
                    }
                    if field.message == "RenderContract" && field.name == "platform_key" {
                        top.summary.platform_key = Some((start, finish));
                    }
                }
            }
        }
        if !field.repeated {
            top.repeated[field_index] = 1;
        }
        if let Some(child) = pending_child {
            stack[depth] = Some(child);
            depth += 1;
        }
    }
    Err(fault(root.end, path, "scanner terminated without root"))
}
fn checked_sum(left: u64, right: u64, offset: usize, path: WirePath) -> Result<u64, WireFault> {
    left.checked_add(right).ok_or_else(|| {
        resource_fault(
            offset,
            path,
            "summary overflow",
            "decoded summary",
            None,
            u64::MAX,
        )
    })
}
