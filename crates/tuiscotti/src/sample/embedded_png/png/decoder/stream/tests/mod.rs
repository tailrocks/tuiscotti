use crate::sample::embedded_png::png::test_utils::*;
use crate::sample::embedded_png::png::{
    Decoder, DecodingError, Reader, ScaledFloat, SourceChromaticities, SrgbRenderingIntent, Unit,
};
use approx::assert_relative_eq;
use byteorder::WriteBytesExt;
use std::borrow::Cow;
use std::cell::RefCell;
use std::fs::File;
use std::io::{BufRead, BufReader, Cursor, ErrorKind, Read, Seek, Write};
use std::rc::Rc;

mod allocation;
mod animation;
mod animation_edge_cases;
mod chunks;
mod fdat;
mod helpers;
mod metadata;
mod streaming;
