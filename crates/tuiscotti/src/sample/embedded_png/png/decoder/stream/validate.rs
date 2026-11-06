use super::*;

impl Info<'_> {
    pub(super) fn validate_default_image(&self, fc: &FrameControl) -> Result<(), DecodingError> {
        // https://www.w3.org/TR/png-3/#fcTL-chunk says that:
        //
        // > The fcTL chunk corresponding to the default image, if it exists, has these
        // > restrictions:
        // >
        // > * The x_offset and y_offset fields must be 0.
        // > * The width and height fields must equal
        // >   the corresponding fields from the IHDR chunk.
        if fc.x_offset != 0
            || fc.y_offset != 0
            || fc.width != self.width
            || fc.height != self.height
        {
            return Err(DecodingError::Format(
                FormatErrorInner::BadSubFrameBounds {}.into(),
            ));
        }
        Ok(())
    }

    pub(super) fn validate(&self, fc: &FrameControl) -> Result<(), DecodingError> {
        if fc.width == 0 || fc.height == 0 {
            return Err(DecodingError::Format(
                FormatErrorInner::InvalidDimensions.into(),
            ));
        }

        // Validate mathematically: fc.width + fc.x_offset <= self.width
        let in_x_bounds = Some(fc.width) <= self.width.checked_sub(fc.x_offset);
        // Validate mathematically: fc.height + fc.y_offset <= self.height
        let in_y_bounds = Some(fc.height) <= self.height.checked_sub(fc.y_offset);

        if !in_x_bounds || !in_y_bounds {
            return Err(DecodingError::Format(
                // TODO: do we want to display the bad bounds?
                FormatErrorInner::BadSubFrameBounds {}.into(),
            ));
        }

        Ok(())
    }
}
