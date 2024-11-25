#![warn(clippy::pedantic)]

use anyhow::{bail, Error};
use flate2::bufread::{ZlibDecoder, ZlibEncoder};
use image::{GenericImageView, ImageFormat, Rgba, RgbaImage};
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
        _ => fail!(&name, "expected a mode and an input file."),
    };
    let Some(in_path) = args.next().map(PathBuf::from) else {
        fail!(&name, "missing input file.")
    };
    let out_path = args.next().map_or_else(
        || match mode {
            Mode::Encode => in_path.with_extension("png"),
            Mode::Decode => in_path.with_extension("bin"),
        },
        PathBuf::from,
    );
    match mode {
        Mode::Encode => {
            let mut buffer = Vec::new();
            File::open(&in_path)?.read_to_end(&mut buffer)?;
            Ok(encode(&buffer)?.write_to(&mut File::create(out_path)?, ImageFormat::Png)?)
        }
        Mode::Decode => {
            let bytes: Vec<u8> = image::open(in_path)?
                .pixels()
                .flat_map(|(_, _, Rgba(c))| c)
                .collect();

            File::create(out_path)?.write_all(&decode(&bytes)?)?;

            Ok(())
        }
    }
}

fn decode(bytes: &[u8]) -> anyhow::Result<Vec<u8>> {
    let is_compressed: bool = {
        if bytes.is_empty() {
            bail!("input file is empty")
        };
        bytes[0] != 0
    };
    let Some((length, data)) = bytes[1..].split_first_chunk() else {
        bail!("input file is invalid")
    };
    let length = usize::try_from(u64::from_le_bytes(*length))?;
    let buf = &data[..length];
    if is_compressed {
        Ok(ZlibDecoder::new(buf)
            .bytes()
            .collect::<Result<Vec<u8>, _>>()?)
    } else {
        Ok(buf.to_owned())
    }
}

fn encode(bytes: &[u8]) -> anyhow::Result<RgbaImage> {
    let is_compressed: bool;
    let compressed: Result<Vec<u8>, _> = ZlibEncoder::new(bytes, flate2::Compression::best())
        .bytes()
        .collect();
    let bytes = match compressed {
        Ok(compressed) => {
            is_compressed = true;
            compressed
        }
        Err(err) => {
            is_compressed = false;
            eprintln!("Compression failed: {err:?}. Encoding raw bytes...");
            bytes.to_owned()
        }
    };
    let mut buf = Vec::new();
    buf.push(is_compressed.into());
    buf.extend((bytes.len() as u64).to_le_bytes());
    buf.extend(bytes);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let w = f64::from(u32::try_from(buf.len())?).sqrt().ceil() as u32;
    let width = w + (4 - w % 4);
    let height = width / 4 - 1;
    buf.resize((width * height * 4) as usize, 0);
    let img =
        RgbaImage::from_vec(width / 2, height * 2, buf).ok_or(Error::msg("buffer too small"))?;
    Ok(img)
}
