fn scan(root: Span<'_>, root_kind: &'static str) -> Result<ScanSummary, WireFault> {
    let mut state = ScanState {
        stack: [None; 32],
        depth: 1,
        path: WirePath::new(),
    };
    state.stack[0] = Some(Frame {
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
    while state.depth > 0 {
        let index = state.depth - 1;
        let top = state.stack[index]
            .as_ref()
            .ok_or_else(|| fault(0, state.path, "missing scanner frame"))?;
        if top.cursor == top.span.end {
            if let Some(summary) = finish_scanned_frame(&mut state)? {
                return Ok(summary);
            }
            continue;
        }
        let child = {
            let top = state.stack[index]
                .as_mut()
                .ok_or_else(|| fault(0, state.path, "missing scanner frame"))?;
            scan_current_field(top, state.path, state.depth)?
        };
        if let Some((child, child_path)) = child {
            state.path = child_path;
            state.stack[state.depth] = Some(child);
            state.depth += 1;
        }
    }
    Err(fault(
        root.end,
        state.path,
        "scanner terminated without root",
    ))
}
fn finish_scanned_frame(state: &mut ScanState<'_>) -> Result<Option<ScanSummary>, WireFault> {
    let index = state.depth - 1;
    let top = state.stack[index]
        .as_mut()
        .ok_or_else(|| fault(0, state.path, "missing scanner frame"))?;
    finish_frame(top, state.path)?;
    let complete = state.stack[index]
        .take()
        .ok_or_else(|| fault(0, state.path, "missing completed frame"))?;
    state.depth -= 1;
    if state.depth == 0 {
        return Ok(Some(complete.summary));
    }
    if complete.indexed {
        state.path.pop_index();
    }
    state.path.pop_field();
    let parent = state.stack[state.depth - 1]
        .as_mut()
        .ok_or_else(|| fault(0, state.path, "missing parent frame"))?;
    merge_summary(parent, &complete.summary, complete.span.end, state.path)?;
    Ok(None)
}
fn merge_summary(
    parent: &mut Frame<'_>,
    child: &ScanSummary,
    end: usize,
    path: WirePath,
) -> Result<(), WireFault> {
    parent.summary.l = checked_sum(parent.summary.l, child.l, end, path)?;
    parent.summary.c = checked_sum(parent.summary.c, child.c, end, path)?;
    parent.summary.f = checked_sum(parent.summary.f, child.f, end, path)?;
    parent.summary.p = checked_sum(parent.summary.p, child.p, end, path)?;
    parent.summary.test_id = parent.summary.test_id.or(child.test_id);
    parent.summary.platform_key = parent.summary.platform_key.or(child.platform_key);
    parent.summary.grid = parent.summary.grid.or(child.grid);
    parent.summary.image = parent.summary.image.or(child.image);
    Ok(())
}
fn scan_current_field<'a>(
    top: &mut Frame<'a>,
    path: WirePath,
    depth: usize,
) -> Result<Option<(Frame<'a>, WirePath)>, WireFault> {
    let step = read_field_header(top, path)?;
    read_field_value(top, step, depth)
}
fn read_field_header(top: &mut Frame<'_>, path: WirePath) -> Result<FieldStep, WireFault> {
    let fields =
        fields_for(top.kind).ok_or_else(|| fault(top.cursor, path, "unknown message kind"))?;
    let offset = top.cursor;
    let key = read_varint(top.span.bytes, &mut top.cursor, top.span.end, path)?;
    let tag = u32::try_from(key >> 3).map_err(|_| fault(offset, path, "field tag overflow"))?;
    let wire = (key & 7) as u8;
    if tag == 0 || tag > 536_870_911 {
        return Err(fault(offset, path, "invalid field tag"));
    }
    let index = fields
        .iter()
        .position(|field| field.tag == tag)
        .ok_or_else(|| {
            let mut unknown = path;
            if unknown.tag(tag) {
                fault(offset, unknown, "unknown field")
            } else {
                fault(offset, path, "wire path capacity proof violated")
            }
        })?;
    if index >= 16 {
        return Err(fault(offset, path, "generated field counter overflow"));
    }
    if index >= 64 {
        return Err(fault(offset, path, "generated field mask overflow"));
    }
    let field = &fields[index];
    let mut field_path = path;
    if !field_path.field(field.id) {
        return Err(fault(offset, path, "wire path capacity proof violated"));
    }
    if field.repeated && !field_path.index(top.repeated[index]) {
        return Err(fault(offset, path, "wire path capacity proof violated"));
    }
    if !field.repeated && top.seen & (1u64 << index) != 0 {
        return Err(fault(offset, field_path, "duplicate singular field"));
    }
    top.seen |= 1u64 << index;
    if field.oneof.is_some() {
        if top.oneofs != 0 {
            return Err(fault(offset, field_path, "multiple oneof alternatives"));
        }
        top.oneofs = 1;
    }
    let expected = match field.kind {
        WireKind::Varint => 0,
        WireKind::Fixed64 => 1,
        WireKind::LengthDelimited => 2,
        WireKind::Fixed32 => 5,
    };
    if wire != expected {
        return Err(fault(offset, field_path, "wire type mismatch"));
    }
    if field.repeated {
        let next = checked_sum(top.repeated[index], 1, offset, field_path)?;
        if let Some(limit) = repeated_limit(top.kind, field.name)
            && next > limit
        {
            return Err(resource_fault(
                offset,
                field_path,
                "palette exceeds 256 entries",
                "palette entries",
                Some(next),
                limit,
            ));
        }
        top.repeated[index] = next;
    }
    Ok(FieldStep {
        index,
        tag,
        offset,
        path: field_path,
    })
}
fn read_field_value<'a>(
    top: &mut Frame<'a>,
    step: FieldStep,
    depth: usize,
) -> Result<Option<(Frame<'a>, WirePath)>, WireFault> {
    let fields = fields_for(top.kind)
        .ok_or_else(|| fault(step.offset, step.path, "unknown message kind"))?;
    let field = &fields[step.index];
    let mut child = None;
    match field.kind {
        WireKind::Varint => {
            let value = read_varint(top.span.bytes, &mut top.cursor, top.span.end, step.path)?;
            check_varint(field, value, step.offset, step.path)?;
            if step.tag <= 16 {
                top.values[step.tag as usize - 1] = value;
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
                .ok_or_else(|| fault(top.cursor, step.path, "fixed field overflow"))?;
            if finish > top.span.end {
                return Err(fault(top.cursor, step.path, "truncated fixed field"));
            }
            check_fixed(
                field,
                &top.span.bytes[top.cursor..finish],
                step.offset,
                step.path,
            )?;
            top.cursor = finish;
        }
        WireKind::LengthDelimited => {
            let (start, finish) =
                read_length(top.span.bytes, &mut top.cursor, top.span.end, step.path)?;
            if let Some(child_kind) = field.type_name {
                if depth == 32 {
                    return Err(resource_fault(
                        start,
                        step.path,
                        "message nesting exceeds 32",
                        "message nesting depth",
                        Some(33),
                        32,
                    ));
                }
                if child_kind == "Cell" {
                    top.summary.c = checked_sum(top.summary.c, 1, start, step.path)?;
                }
                if child_kind == "Font" {
                    top.summary.f = checked_sum(top.summary.f, 1, start, step.path)?;
                }
                child = Some(Frame {
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
                });
            } else {
                update_length_summary(top, field, start, finish, step.path)?;
            }
        }
    }
    if !field.repeated {
        top.repeated[step.index] = 1;
    }
    Ok(child.map(|child| (child, step.path)))
}
fn update_length_summary(
    top: &mut Frame<'_>,
    field: &FieldSpec,
    start: usize,
    finish: usize,
    path: WirePath,
) -> Result<(), WireFault> {
    let length = (finish - start) as u64;
    check_length(field, &top.span.bytes[start..finish], start, path)?;
    top.summary.l = checked_sum(top.summary.l, length, start, path)?;
    if field.message == "Image" && field.name == "png" {
        top.summary.p = checked_sum(top.summary.p, length, start, path)?;
    }
    if field.message == "Sample" && field.name == "test_id" {
        top.summary.test_id = Some((start, finish));
    }
    if field.message == "RenderContract" && field.name == "platform_key" {
        top.summary.platform_key = Some((start, finish));
    }
    Ok(())
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
