use super::*;
use crate::sample::embedded_png::png::chunk;
#[cfg(test)]
use crate::sample::embedded_png::png::encoder;
#[cfg(test)]
use std::io::Write;

/// Frame control information
#[derive(Clone, Copy, Debug)]
pub(crate) struct FrameControl {
    /// Sequence number of the animation chunk, starting from 0
    pub(crate) sequence_number: u32,
    /// Width of the following frame
    pub(crate) width: u32,
    /// Height of the following frame
    pub(crate) height: u32,
    /// X position at which to render the following frame
    pub(crate) x_offset: u32,
    /// Y position at which to render the following frame
    pub(crate) y_offset: u32,
    /// Frame delay fraction numerator
    pub(crate) delay_num: u16,
    /// Frame delay fraction denominator
    pub(crate) delay_den: u16,
    /// Type of frame area disposal to be done after rendering this frame
    pub(crate) dispose_op: DisposeOp,
    /// Type of frame area rendering for this frame
    pub(crate) blend_op: BlendOp,
}

impl Default for FrameControl {
    fn default() -> FrameControl {
        FrameControl {
            sequence_number: 0,
            width: 0,
            height: 0,
            x_offset: 0,
            y_offset: 0,
            delay_num: 1,
            delay_den: 30,
            dispose_op: DisposeOp::None,
            blend_op: BlendOp::Source,
        }
    }
}

impl FrameControl {
    pub(crate) fn set_seq_num(&mut self, s: u32) {
        self.sequence_number = s;
    }

    pub(crate) fn inc_seq_num(&mut self, i: u32) {
        self.sequence_number += i;
    }

    #[cfg(test)]
    pub(crate) fn encode<W: Write>(self, w: &mut W) -> encoder::Result<()> {
        let mut data = [0u8; 26];
        data[..4].copy_from_slice(&self.sequence_number.to_be_bytes());
        data[4..8].copy_from_slice(&self.width.to_be_bytes());
        data[8..12].copy_from_slice(&self.height.to_be_bytes());
        data[12..16].copy_from_slice(&self.x_offset.to_be_bytes());
        data[16..20].copy_from_slice(&self.y_offset.to_be_bytes());
        data[20..22].copy_from_slice(&self.delay_num.to_be_bytes());
        data[22..24].copy_from_slice(&self.delay_den.to_be_bytes());
        data[24] = self.dispose_op as u8;
        data[25] = self.blend_op as u8;

        encoder::write_chunk(w, chunk::FC_TL, &data)
    }
}

/// Animation control information
#[derive(Clone, Copy, Debug)]
pub(crate) struct AnimationControl {
    /// Number of frames
    pub(crate) num_frames: u32,
    /// Number of times to loop this APNG.  0 indicates infinite looping.
    pub(crate) num_plays: u32,
}

impl AnimationControl {
    #[cfg(test)]
    pub(crate) fn encode<W: Write>(self, w: &mut W) -> encoder::Result<()> {
        let mut data = [0; 8];
        data[..4].copy_from_slice(&self.num_frames.to_be_bytes());
        data[4..].copy_from_slice(&self.num_plays.to_be_bytes());
        encoder::write_chunk(w, chunk::AC_TL, &data)
    }
}
