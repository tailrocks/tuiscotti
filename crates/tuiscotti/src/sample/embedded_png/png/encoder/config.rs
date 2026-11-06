use super::*;
use crate::sample::embedded_png::png::{SourceChromaticities, SrgbRenderingIntent};

/// PNG Encoder.
///
/// This configures the PNG format options such as animation chunks, palette use, color types,
/// auxiliary chunks etc.
///
/// FIXME: Configuring APNG might be easier (less individual errors) if we had an _adapter_ which
/// borrows this mutably but guarantees that `info.frame_control` is not `None`.

pub(crate) struct Encoder<'a, W: Write> {
    w: W,
    info: Info<'a>,
    options: Options,
}

/// Encoding options, internal type, forwarded to the Writer.
#[derive(Default)]
pub(super) struct Options {
    pub(super) filter: Filter,
    pub(super) sep_def_img: bool,
    pub(super) validate_sequence: bool,
    pub(super) compression: DeflateCompression,
}

impl<'a, W: Write> Encoder<'a, W> {
    pub(crate) fn new(w: W, width: u32, height: u32) -> Encoder<'static, W> {
        Encoder {
            w,
            info: Info::with_size(width, height),
            options: Options::default(),
        }
    }

    pub(crate) fn with_info(w: W, info: Info<'a>) -> Result<Encoder<'a, W>> {
        if info.animation_control.is_some() != info.frame_control.is_some() {
            return Err(EncodingError::Format(FormatErrorKind::NotAnimated.into()));
        }

        if let Some(actl) = info.animation_control {
            if actl.num_frames == 0 {
                return Err(EncodingError::Format(FormatErrorKind::ZeroFrames.into()));
            }
        }

        Ok(Encoder {
            w,
            info,
            options: Options::default(),
        })
    }

    /// Specify that the image is animated.
    ///
    /// `num_frames` controls how many frames the animation has, while
    /// `num_plays` controls how many times the animation should be
    /// repeated until it stops, if it's zero then it will repeat
    /// infinitely.
    ///
    /// When this method is returns successfully then the images written will be encoded as fdAT
    /// chunks, except for the first image that is still encoded as `IDAT`. You can control if the
    /// first frame should be treated as an animation frame with [`Encoder::set_sep_def_img()`].
    ///
    /// This method returns an error if `num_frames` is 0.
    pub(crate) fn set_animated(&mut self, num_frames: u32, num_plays: u32) -> Result<()> {
        if num_frames == 0 {
            return Err(EncodingError::Format(FormatErrorKind::ZeroFrames.into()));
        }

        let actl = AnimationControl {
            num_frames,
            num_plays,
        };

        let fctl = FrameControl {
            sequence_number: 0,
            width: self.info.width,
            height: self.info.height,
            ..Default::default()
        };

        self.info.animation_control = Some(actl);
        self.info.frame_control = Some(fctl);
        Ok(())
    }

    /// Mark the first animated frame as a 'separate default image'.
    ///
    /// In APNG each animated frame is preceded by a special control chunk, `fcTL`. It's up to the
    /// encoder to decide if the first image, the standard `IDAT` data, should be part of the
    /// animation by emitting this chunk or by not doing so. A default image that is _not_ part of
    /// the animation is often interpreted as a thumbnail.
    ///
    /// This method will return an error when animation control was not configured
    /// (which is done by calling [`Encoder::set_animated`]).
    pub(crate) fn set_sep_def_img(&mut self, sep_def_img: bool) -> Result<()> {
        if self.info.animation_control.is_some() {
            self.options.sep_def_img = sep_def_img;
            Ok(())
        } else {
            Err(EncodingError::Format(FormatErrorKind::NotAnimated.into()))
        }
    }

    /// Sets the raw byte contents of the PLTE chunk. This method accepts
    /// both borrowed and owned byte data.
    pub(crate) fn set_palette<T: Into<Cow<'a, [u8]>>>(&mut self, palette: T) {
        self.info.palette = Some(palette.into());
    }

    /// Sets the raw byte contents of the tRNS chunk. This method accepts
    /// both borrowed and owned byte data.
    pub(crate) fn set_trns<T: Into<Cow<'a, [u8]>>>(&mut self, trns: T) {
        self.info.trns = Some(trns.into());
    }

    /// Set the display gamma of the source system on which the image was generated or last edited.
    pub(crate) fn set_source_gamma(&mut self, source_gamma: ScaledFloat) {
        self.info.source_gamma = Some(source_gamma);
    }

    /// Set the chromaticities for the source system's display channels (red, green, blue) and the whitepoint
    /// of the source system on which the image was generated or last edited.
    pub(crate) fn set_source_chromaticities(
        &mut self,
        source_chromaticities: SourceChromaticities,
    ) {
        self.info.source_chromaticities = Some(source_chromaticities);
    }

    /// Mark the image data as conforming to the SRGB color space with the specified rendering intent.
    ///
    /// Any ICC profiles will be ignored.
    ///
    /// Source gamma and chromaticities will be written only if they're set to fallback
    /// values specified in [11.3.2.5](https://www.w3.org/TR/png-3/#sRGB-gAMA-cHRM).
    pub(crate) fn set_source_srgb(&mut self, rendering_intent: SrgbRenderingIntent) {
        self.info.set_source_srgb(rendering_intent);
    }

    /// Start encoding by writing the header data.
    ///
    /// The remaining data can be supplied by methods on the returned [`Writer`].
    pub(crate) fn write_header(self) -> Result<Writer<W>> {
        Writer::new(self.w, super::PartialInfo::new(&self.info), self.options).init(&self.info)
    }

    /// Set the color of the encoded image.
    ///
    /// These correspond to the color types in the png IHDR data that will be written. The length
    /// of the image data that is later supplied must match the color type, otherwise an error will
    /// be emitted.
    pub(crate) fn set_color(&mut self, color: ColorType) {
        self.info.color_type = color;
    }

    /// Set the indicated depth of the image data.
    pub(crate) fn set_depth(&mut self, depth: BitDepth) {
        self.info.bit_depth = depth;
    }

    /// Set compression parameters, see [`Compression`] for the available options.
    pub(crate) fn set_compression(&mut self, compression: Compression) {
        self.set_deflate_compression(DeflateCompression::from_simple(compression));
        self.set_filter(Filter::from_simple(compression));
    }

    /// Provides in-depth customization of DEFLATE compression options.
    ///
    /// For a simpler selection of compression options see [`set_compression`].
    ///
    /// [`set_compression`]: Self::set_compression
    pub(crate) fn set_deflate_compression(&mut self, compression: DeflateCompression) {
        self.options.compression = compression;
    }

    /// Set the used filter type.
    ///
    /// The default filter is [`Filter::Adaptive`] which automatically selects the best filter
    /// for each row of the image.
    ///
    /// You should only change this if you are after very fast compression,
    /// and either don't care about compression ratio or know exactly what works best for your images.
    pub(crate) fn set_filter(&mut self, filter: Filter) {
        self.options.filter = filter;
    }

    /// Set the fraction of time every frame is going to be displayed, in seconds.
    ///
    /// *Note that this parameter can be set for each individual frame after
    /// [`Encoder::write_header`] is called. (see [`Writer::set_frame_delay`])*
    ///
    /// If the denominator is 0, it is to be treated as if it were 100
    /// (that is, the numerator then specifies 1/100ths of a second).
    /// If the value of the numerator is 0 the decoder should render the next frame
    /// as quickly as possible, though viewers may impose a reasonable lower bound.
    ///
    /// The default value is 0 for both the numerator and denominator.
    ///
    /// This method will return an error if the image is not animated.
    /// (see [`set_animated`])
    ///
    /// [`write_header`]: Self::write_header
    /// [`set_animated`]: Self::set_animated
    pub(crate) fn set_frame_delay(&mut self, numerator: u16, denominator: u16) -> Result<()> {
        if let Some(ref mut fctl) = self.info.frame_control {
            fctl.delay_den = denominator;
            fctl.delay_num = numerator;
            Ok(())
        } else {
            Err(EncodingError::Format(FormatErrorKind::NotAnimated.into()))
        }
    }

    /// Set the blend operation for every frame.
    ///
    /// The blend operation specifies whether the frame is to be alpha blended
    /// into the current output buffer content, or whether it should completely
    /// replace its region in the output buffer.
    ///
    /// *Note that this parameter can be set for each individual frame after
    /// [`write_header`] is called. (see [`Writer::set_blend_op`])*
    ///
    /// See the [`BlendOp`] documentation for the possible values and their effects.
    ///
    /// *Note that for the first frame the two blend modes are functionally
    /// equivalent due to the clearing of the output buffer at the beginning
    /// of each play.*
    ///
    /// The default value is [`BlendOp::Source`].
    ///
    /// This method will return an error if the image is not animated.
    /// (see [`set_animated`])
    ///
    /// [`write_header`]: Self::write_header
    /// [`set_animated`]: Self::set_animated
    pub(crate) fn set_blend_op(&mut self, op: BlendOp) -> Result<()> {
        if let Some(ref mut fctl) = self.info.frame_control {
            fctl.blend_op = op;
            Ok(())
        } else {
            Err(EncodingError::Format(FormatErrorKind::NotAnimated.into()))
        }
    }

    /// Set the dispose operation for every frame.
    ///
    /// The dispose operation specifies how the output buffer should be changed
    /// at the end of the delay (before rendering the next frame)
    ///
    /// *Note that this parameter can be set for each individual frame after
    /// [`write_header`] is called (see [`Writer::set_dispose_op`])*
    ///
    /// See the [`DisposeOp`] documentation for the possible values and their effects.
    ///
    /// *Note that if the first frame uses [`DisposeOp::Previous`]
    /// it will be treated as [`DisposeOp::Background`].*
    ///
    /// The default value is [`DisposeOp::None`].
    ///
    /// This method will return an error if the image is not animated.
    /// (see [`set_animated`])
    ///
    /// [`set_animated`]: Self::set_animated
    /// [`write_header`]: Self::write_header
    pub(crate) fn set_dispose_op(&mut self, op: DisposeOp) -> Result<()> {
        if let Some(ref mut fctl) = self.info.frame_control {
            fctl.dispose_op = op;
            Ok(())
        } else {
            Err(EncodingError::Format(FormatErrorKind::NotAnimated.into()))
        }
    }
    pub(crate) fn set_pixel_dims(&mut self, pixel_dims: Option<PixelDimensions>) {
        self.info.pixel_dims = pixel_dims
    }
    /// Convenience function to add tEXt chunks to [`Info`] struct
    pub(crate) fn add_text_chunk(&mut self, keyword: String, text: String) -> Result<()> {
        let text_chunk = TEXtChunk::new(keyword, text);
        self.info.uncompressed_latin1_text.push(text_chunk);
        Ok(())
    }

    /// Convenience function to add zTXt chunks to [`Info`] struct
    pub(crate) fn add_ztxt_chunk(&mut self, keyword: String, text: String) -> Result<()> {
        let text_chunk = ZTXtChunk::new(keyword, text);
        self.info.compressed_latin1_text.push(text_chunk);
        Ok(())
    }

    /// Convenience function to add iTXt chunks to [`Info`] struct
    ///
    /// This function only sets the `keyword` and `text` field of the iTXt chunk.
    /// To set the other fields, create a [`ITXtChunk`] directly, and then encode it to the output stream.
    pub(crate) fn add_itxt_chunk(&mut self, keyword: String, text: String) -> Result<()> {
        let text_chunk = ITXtChunk::new(keyword, text);
        self.info.utf8_text.push(text_chunk);
        Ok(())
    }

    /// Validate the written image sequence.
    ///
    /// When validation is turned on (it's turned off by default) then attempts to write more than
    /// one `IDAT` image or images beyond the number of frames indicated in the animation control
    /// chunk will fail and return an error result instead. Attempts to [finish][finish] the image
    /// with missing frames will also return an error.
    ///
    /// [finish]: StreamWriter::finish
    ///
    /// (It's possible to circumvent these checks by writing raw chunks instead.)
    pub(crate) fn validate_sequence(&mut self, validate: bool) {
        self.options.validate_sequence = validate;
    }
}
