use anyhow::Error;
use image::{ImageFormat, ImageReader, RgbImage};
use std::env::args;
use std::fmt::Write as WriteFmt;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

fn usage_err(name: &str, msg: &str) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "USAGE: {name} <-e|-d> <in.file> [out.file]");
    let _ = writeln!(s, "Modes:");
    let _ = writeln!(s, "-e: Encode bytes as color to png");
    let _ = writeln!(s, "-d: Decode png data back to bytes");
    let _ = writeln!(s, "ERROR: {msg}");
    s
}

enum Mode {
    Encode,
    Decode,
}

fn main() -> anyhow::Result<()> {
    macro_rules! fail {
        ($name:expr, $msg:expr) => {
            return Err(anyhow::Error::msg(usage_err($name, $msg)))
        };
    }
    let mut args = args();
    let name = args.next().unwrap();
    let mode = match args.next() {
        Some(e) if e == "-e" => Mode::Encode,
        Some(d) if d == "-d" => Mode::Decode,
        Some(p) => fail!(&name, &format!("invalid mode {p}")),
        _ => fail!(&name, "expected a mode and an input file.")
    };
    let Some(in_path) = args.next().map(PathBuf::from) else { fail!(&name, "missing input file.") };
    let out_path = args.next().map_or_else(|| {
        match mode {
            Mode::Encode => in_path.with_extension("png"),
            Mode::Decode => in_path.with_extension("bin"),
        }
    }, PathBuf::from);
    match mode {
        Mode::Encode => {
            let mut buffer = Vec::new();
            File::open(&in_path)?.read_to_end(&mut buffer)?;
            Ok(encode(&buffer)?.write_to(&mut File::create(out_path)?, ImageFormat::Png)?)
        }
        Mode::Decode => {
            let img = RgbImage::from(ImageReader::open(in_path)?.decode()?);
            let bytes = img.pixels().flat_map(|p|p.0).collect::<Vec<_>>();
            Ok(File::create(out_path)?.write_all(&bytes)?)
        }
    }
}

fn encode(bytes: &[u8]) -> anyhow::Result<RgbImage> {
    let mut buf = bytes.to_owned();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let w = f64::from(u32::try_from(buf.len())?).sqrt().ceil() as u32;
    let width = w + (3 - w % 3);
    let height = width / 3 - 1;
    buf.resize((width * height * 3) as usize, 0);
    let img = RgbImage::from_vec(width, height, buf).ok_or(Error::msg("buffer too small"))?;
    Ok(img)
}