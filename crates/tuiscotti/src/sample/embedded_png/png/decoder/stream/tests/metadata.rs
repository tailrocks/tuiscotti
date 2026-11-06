use super::*;

#[test]
fn image_gamma() -> Result<(), Box<dyn std::error::Error>> {
    fn trial(path: &str, expected: Option<ScaledFloat>) -> Result<(), Box<dyn std::error::Error>> {
        let decoder =
            crate::sample::embedded_png::png::Decoder::new(BufReader::new(File::open(path)?));
        let reader = decoder.read_info()?;
        let actual: Option<ScaledFloat> = reader.info()?.gamma();
        assert_eq!(actual, expected);
        Ok(())
    }
    trial("tests/pngsuite/f00n0g08.png", None)?;
    trial("tests/pngsuite/f00n2c08.png", None)?;
    trial("tests/pngsuite/f01n0g08.png", None)?;
    trial("tests/pngsuite/f01n2c08.png", None)?;
    trial("tests/pngsuite/f02n0g08.png", None)?;
    trial("tests/pngsuite/f02n2c08.png", None)?;
    trial("tests/pngsuite/f03n0g08.png", None)?;
    trial("tests/pngsuite/f03n2c08.png", None)?;
    trial("tests/pngsuite/f04n0g08.png", None)?;
    trial("tests/pngsuite/f04n2c08.png", None)?;
    trial("tests/pngsuite/f99n0g04.png", None)?;
    trial("tests/pngsuite/tm3n3p02.png", None)?;
    trial("tests/pngsuite/g03n0g16.png", Some(ScaledFloat::new(0.35)))?;
    trial("tests/pngsuite/g03n2c08.png", Some(ScaledFloat::new(0.35)))?;
    trial("tests/pngsuite/g03n3p04.png", Some(ScaledFloat::new(0.35)))?;
    trial("tests/pngsuite/g04n0g16.png", Some(ScaledFloat::new(0.45)))?;
    trial("tests/pngsuite/g04n2c08.png", Some(ScaledFloat::new(0.45)))?;
    trial("tests/pngsuite/g04n3p04.png", Some(ScaledFloat::new(0.45)))?;
    trial("tests/pngsuite/g05n0g16.png", Some(ScaledFloat::new(0.55)))?;
    trial("tests/pngsuite/g05n2c08.png", Some(ScaledFloat::new(0.55)))?;
    trial("tests/pngsuite/g05n3p04.png", Some(ScaledFloat::new(0.55)))?;
    trial("tests/pngsuite/g07n0g16.png", Some(ScaledFloat::new(0.7)))?;
    trial("tests/pngsuite/g07n2c08.png", Some(ScaledFloat::new(0.7)))?;
    trial("tests/pngsuite/g07n3p04.png", Some(ScaledFloat::new(0.7)))?;
    trial("tests/pngsuite/g10n0g16.png", Some(ScaledFloat::new(1.0)))?;
    trial("tests/pngsuite/g10n2c08.png", Some(ScaledFloat::new(1.0)))?;
    trial("tests/pngsuite/g10n3p04.png", Some(ScaledFloat::new(1.0)))?;
    trial("tests/pngsuite/g25n0g16.png", Some(ScaledFloat::new(2.5)))?;
    trial("tests/pngsuite/g25n2c08.png", Some(ScaledFloat::new(2.5)))?;
    trial("tests/pngsuite/g25n3p04.png", Some(ScaledFloat::new(2.5)))?;
    Ok(())
}

#[test]
fn image_source_chromaticities() -> Result<(), Box<dyn std::error::Error>> {
    fn trial(
        path: &str,
        expected: Option<SourceChromaticities>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let decoder =
            crate::sample::embedded_png::png::Decoder::new(BufReader::new(File::open(path)?));
        let reader = decoder.read_info()?;
        let actual: Option<SourceChromaticities> = reader.info()?.chromaticities();
        assert_eq!(actual, expected);
        Ok(())
    }
    trial(
        "tests/pngsuite/ccwn2c08.png",
        Some(SourceChromaticities::new(
            (0.3127, 0.3290),
            (0.64, 0.33),
            (0.30, 0.60),
            (0.15, 0.06),
        )),
    )?;
    trial(
        "tests/pngsuite/ccwn3p08.png",
        Some(SourceChromaticities::new(
            (0.3127, 0.3290),
            (0.64, 0.33),
            (0.30, 0.60),
            (0.15, 0.06),
        )),
    )?;
    trial("tests/pngsuite/basi0g01.png", None)?;
    trial("tests/pngsuite/basi0g02.png", None)?;
    trial("tests/pngsuite/basi0g04.png", None)?;
    trial("tests/pngsuite/basi0g08.png", None)?;
    trial("tests/pngsuite/basi0g16.png", None)?;
    trial("tests/pngsuite/basi2c08.png", None)?;
    trial("tests/pngsuite/basi2c16.png", None)?;
    trial("tests/pngsuite/basi3p01.png", None)?;
    trial("tests/pngsuite/basi3p02.png", None)?;
    trial("tests/pngsuite/basi3p04.png", None)?;
    trial("tests/pngsuite/basi3p08.png", None)?;
    trial("tests/pngsuite/basi4a08.png", None)?;
    trial("tests/pngsuite/basi4a16.png", None)?;
    trial("tests/pngsuite/basi6a08.png", None)?;
    trial("tests/pngsuite/basi6a16.png", None)?;
    trial("tests/pngsuite/basn0g01.png", None)?;
    trial("tests/pngsuite/basn0g02.png", None)?;
    trial("tests/pngsuite/basn0g04.png", None)?;
    trial("tests/pngsuite/basn0g08.png", None)?;
    trial("tests/pngsuite/basn0g16.png", None)?;
    trial("tests/pngsuite/basn2c08.png", None)?;
    trial("tests/pngsuite/basn2c16.png", None)?;
    trial("tests/pngsuite/basn3p01.png", None)?;
    trial("tests/pngsuite/basn3p02.png", None)?;
    trial("tests/pngsuite/basn3p04.png", None)?;
    trial("tests/pngsuite/basn3p08.png", None)?;
    trial("tests/pngsuite/basn4a08.png", None)?;
    trial("tests/pngsuite/basn4a16.png", None)?;
    trial("tests/pngsuite/basn6a08.png", None)?;
    trial("tests/pngsuite/basn6a16.png", None)?;
    trial("tests/pngsuite/bgai4a08.png", None)?;
    trial("tests/pngsuite/bgai4a16.png", None)?;
    trial("tests/pngsuite/bgan6a08.png", None)?;
    trial("tests/pngsuite/bgan6a16.png", None)?;
    trial("tests/pngsuite/bgbn4a08.png", None)?;
    trial("tests/pngsuite/bggn4a16.png", None)?;
    trial("tests/pngsuite/bgwn6a08.png", None)?;
    trial("tests/pngsuite/bgyn6a16.png", None)?;
    trial("tests/pngsuite/cdfn2c08.png", None)?;
    trial("tests/pngsuite/cdhn2c08.png", None)?;
    trial("tests/pngsuite/cdsn2c08.png", None)?;
    trial("tests/pngsuite/cdun2c08.png", None)?;
    trial("tests/pngsuite/ch1n3p04.png", None)?;
    trial("tests/pngsuite/ch2n3p08.png", None)?;
    trial("tests/pngsuite/cm0n0g04.png", None)?;
    trial("tests/pngsuite/cm7n0g04.png", None)?;
    trial("tests/pngsuite/cm9n0g04.png", None)?;
    trial("tests/pngsuite/cs3n2c16.png", None)?;
    trial("tests/pngsuite/cs3n3p08.png", None)?;
    trial("tests/pngsuite/cs5n2c08.png", None)?;
    trial("tests/pngsuite/cs5n3p08.png", None)?;
    trial("tests/pngsuite/cs8n2c08.png", None)?;
    trial("tests/pngsuite/cs8n3p08.png", None)?;
    trial("tests/pngsuite/ct0n0g04.png", None)?;
    trial("tests/pngsuite/ct1n0g04.png", None)?;
    trial("tests/pngsuite/cten0g04.png", None)?;
    trial("tests/pngsuite/ctfn0g04.png", None)?;
    trial("tests/pngsuite/ctgn0g04.png", None)?;
    trial("tests/pngsuite/cthn0g04.png", None)?;
    trial("tests/pngsuite/ctjn0g04.png", None)?;
    trial("tests/pngsuite/ctzn0g04.png", None)?;
    trial("tests/pngsuite/f00n0g08.png", None)?;
    trial("tests/pngsuite/f00n2c08.png", None)?;
    trial("tests/pngsuite/f01n0g08.png", None)?;
    trial("tests/pngsuite/f01n2c08.png", None)?;
    trial("tests/pngsuite/f02n0g08.png", None)?;
    trial("tests/pngsuite/f02n2c08.png", None)?;
    trial("tests/pngsuite/f03n0g08.png", None)?;
    trial("tests/pngsuite/f03n2c08.png", None)?;
    trial("tests/pngsuite/f04n0g08.png", None)?;
    trial("tests/pngsuite/f04n2c08.png", None)?;
    trial("tests/pngsuite/f99n0g04.png", None)?;
    trial("tests/pngsuite/g03n0g16.png", None)?;
    trial("tests/pngsuite/g03n2c08.png", None)?;
    trial("tests/pngsuite/g03n3p04.png", None)?;
    trial("tests/pngsuite/g04n0g16.png", None)?;
    trial("tests/pngsuite/g04n2c08.png", None)?;
    trial("tests/pngsuite/g04n3p04.png", None)?;
    trial("tests/pngsuite/g05n0g16.png", None)?;
    trial("tests/pngsuite/g05n2c08.png", None)?;
    trial("tests/pngsuite/g05n3p04.png", None)?;
    trial("tests/pngsuite/g07n0g16.png", None)?;
    trial("tests/pngsuite/g07n2c08.png", None)?;
    trial("tests/pngsuite/g07n3p04.png", None)?;
    trial("tests/pngsuite/g10n0g16.png", None)?;
    trial("tests/pngsuite/g10n2c08.png", None)?;
    trial("tests/pngsuite/g10n3p04.png", None)?;
    trial("tests/pngsuite/g25n0g16.png", None)?;
    trial("tests/pngsuite/g25n2c08.png", None)?;
    trial("tests/pngsuite/g25n3p04.png", None)?;
    trial("tests/pngsuite/oi1n0g16.png", None)?;
    trial("tests/pngsuite/oi1n2c16.png", None)?;
    trial("tests/pngsuite/oi2n0g16.png", None)?;
    trial("tests/pngsuite/oi2n2c16.png", None)?;
    trial("tests/pngsuite/oi4n0g16.png", None)?;
    trial("tests/pngsuite/oi4n2c16.png", None)?;
    trial("tests/pngsuite/oi9n0g16.png", None)?;
    trial("tests/pngsuite/oi9n2c16.png", None)?;
    trial("tests/pngsuite/PngSuite.png", None)?;
    trial("tests/pngsuite/pp0n2c16.png", None)?;
    trial("tests/pngsuite/pp0n6a08.png", None)?;
    trial("tests/pngsuite/ps1n0g08.png", None)?;
    trial("tests/pngsuite/ps1n2c16.png", None)?;
    trial("tests/pngsuite/ps2n0g08.png", None)?;
    trial("tests/pngsuite/ps2n2c16.png", None)?;
    trial("tests/pngsuite/s01i3p01.png", None)?;
    trial("tests/pngsuite/s01n3p01.png", None)?;
    trial("tests/pngsuite/s02i3p01.png", None)?;
    trial("tests/pngsuite/s02n3p01.png", None)?;
    trial("tests/pngsuite/s03i3p01.png", None)?;
    trial("tests/pngsuite/s03n3p01.png", None)?;
    trial("tests/pngsuite/s04i3p01.png", None)?;
    trial("tests/pngsuite/s04n3p01.png", None)?;
    trial("tests/pngsuite/s05i3p02.png", None)?;
    trial("tests/pngsuite/s05n3p02.png", None)?;
    trial("tests/pngsuite/s06i3p02.png", None)?;
    trial("tests/pngsuite/s06n3p02.png", None)?;
    trial("tests/pngsuite/s07i3p02.png", None)?;
    trial("tests/pngsuite/s07n3p02.png", None)?;
    trial("tests/pngsuite/s08i3p02.png", None)?;
    trial("tests/pngsuite/s08n3p02.png", None)?;
    trial("tests/pngsuite/s09i3p02.png", None)?;
    trial("tests/pngsuite/s09n3p02.png", None)?;
    trial("tests/pngsuite/s32i3p04.png", None)?;
    trial("tests/pngsuite/s32n3p04.png", None)?;
    trial("tests/pngsuite/s33i3p04.png", None)?;
    trial("tests/pngsuite/s33n3p04.png", None)?;
    trial("tests/pngsuite/s34i3p04.png", None)?;
    trial("tests/pngsuite/s34n3p04.png", None)?;
    trial("tests/pngsuite/s35i3p04.png", None)?;
    trial("tests/pngsuite/s35n3p04.png", None)?;
    trial("tests/pngsuite/s36i3p04.png", None)?;
    trial("tests/pngsuite/s36n3p04.png", None)?;
    trial("tests/pngsuite/s37i3p04.png", None)?;
    trial("tests/pngsuite/s37n3p04.png", None)?;
    trial("tests/pngsuite/s38i3p04.png", None)?;
    trial("tests/pngsuite/s38n3p04.png", None)?;
    trial("tests/pngsuite/s39i3p04.png", None)?;
    trial("tests/pngsuite/s39n3p04.png", None)?;
    trial("tests/pngsuite/s40i3p04.png", None)?;
    trial("tests/pngsuite/s40n3p04.png", None)?;
    trial("tests/pngsuite/tbbn0g04.png", None)?;
    trial("tests/pngsuite/tbbn2c16.png", None)?;
    trial("tests/pngsuite/tbbn3p08.png", None)?;
    trial("tests/pngsuite/tbgn2c16.png", None)?;
    trial("tests/pngsuite/tbgn3p08.png", None)?;
    trial("tests/pngsuite/tbrn2c08.png", None)?;
    trial("tests/pngsuite/tbwn0g16.png", None)?;
    trial("tests/pngsuite/tbwn3p08.png", None)?;
    trial("tests/pngsuite/tbyn3p08.png", None)?;
    trial("tests/pngsuite/tm3n3p02.png", None)?;
    trial("tests/pngsuite/tp0n0g08.png", None)?;
    trial("tests/pngsuite/tp0n2c08.png", None)?;
    trial("tests/pngsuite/tp0n3p08.png", None)?;
    trial("tests/pngsuite/tp1n3p08.png", None)?;
    trial("tests/pngsuite/z00n2c08.png", None)?;
    trial("tests/pngsuite/z03n2c08.png", None)?;
    trial("tests/pngsuite/z06n2c08.png", None)?;
    Ok(())
}
