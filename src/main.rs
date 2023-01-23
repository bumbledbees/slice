use std::io::prelude::*;
use std::io::SeekFrom;
use std::ffi::OsStr;
use std::fs::File;
use std::path::{Path, PathBuf};

use clap::{arg, command, value_parser, Arg, Command, crate_authors};
use clap::builder::{ValueParser, StringValueParser, TypedValueParser};
use clap::error::{Error, ErrorKind};

#[derive(Clone)]
struct PrefixedU64ValueParser;

impl TypedValueParser for PrefixedU64ValueParser {
    type Value = u64;

    fn parse_ref(
        &self, cmd: &Command, arg: Option<&Arg>, value: &OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let inner = StringValueParser::new();
        let num = inner.parse_ref(cmd, arg, value)?;
        let prefix = &num[0..2];
        let (prefix, base) = match prefix {
            "0x" => (Some("0x"), 16),
            "0o" => (Some("0o"), 8),
            "0b" => (Some("0b"), 2),
            _ => (None, 10),
        };
        let num = match prefix {
            Some(p) => num.trim_start_matches(p),
            None => &num,
        };
        return match u64::from_str_radix(num, base) {
            Err(e) => {
                Err(Error::raw(ErrorKind::InvalidValue, e.to_string()))
            },
            Ok(t) => Ok(t),
        };
    }
}

fn main() {
    let args = command!()
        .author(crate_authors!())
        .after_help("no specifying a byte to stop on... for now")
        .arg(
            arg!([input] "file to read")
            .required(true)
            .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            arg!(-o --output <output> "file to output to (default: stdout)")
            .required(false)
            .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            arg!(-n --bytes <bytes> "number of bytes to read (default: all)")
            .required(false)
            .value_parser(ValueParser::new(PrefixedU64ValueParser)),
        )
        .arg(
            arg!(-s --skip <skip> "number of bytes to skip (default: 0)")
            .required(false)
            .value_parser(ValueParser::new(PrefixedU64ValueParser)),
        )
        .get_matches();

    let skip =
        if let Some(skip) = args.get_one::<u64>("skip") { *skip }
        else { 0 };

    let input = args.get_one::<PathBuf>("input").unwrap();
    {
        let input_path = input.as_path();
        if !input_path.exists() {
            let input_path = input_path.to_str();
            match input_path {
                Some(o) => panic!("invalid path {o}!"),
                None => panic!("specified path was invalid UTF-8!"),
            };
        }
    }

    let mut input = File::open(input).expect("error opening input file!");

    let bytes =
        if let Some(bytes) = args.get_one::<u64>("bytes") { *bytes }
        else {
            let input = input
                .metadata()
                .expect("error reading file metadata!")
                .len();
            input - skip
        };

    // type annotations for my sanity lol
    let mut output: Box<dyn Write>;
    let output_path: Option<&PathBuf> =
        args.get_one::<PathBuf>("output");
    match output_path {
        Some(output_path) => {
            let output_path: &Path = output_path.as_path();
            output = match output_path.to_str() {
                Some(s) => {
                    if s == "-" {
                        Box::new(std::io::stdout())
                    } else {
                        Box::new(
                            File::create(s)
                            .expect("error creating output file!")
                        )
                    }
                },
                None => panic!("invalid UTF-8 in output file!"),
            };
        },
        None => {
            output = Box::new(std::io::stdout());
        },
    };
    slice(bytes, skip, &mut input, &mut output)
        .expect("error slicing file :O!");
}

fn slice<R: Read + Seek, W: Write>(
    bytes: u64, skip: u64, input: &mut R, output: &mut W
)-> std::io::Result<()> {
    let mut data = Vec::with_capacity(bytes as usize);
    {
        if skip > 0 { input.seek(SeekFrom::Start(skip))?; }
        input.take(bytes).read_to_end(&mut data)?;
    }
    {
        output.write_all(&data)?;
    }
    Ok(())
}
