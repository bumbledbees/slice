use std::io::prelude::*;
use std::io::SeekFrom;
use std::ffi::OsStr;
use std::fs::File;
use std::path::PathBuf;

use clap::{arg, command, value_parser, Arg, Command};
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
        let num = num.replace("_", "");
        let prefix = if num.len() < 2 { "" } else { &num[0..2] };
        let (num, base) = {
            match prefix {
                "0x" => (num.trim_start_matches("0x"), 16),
                "0o" => (num.trim_start_matches("0o"), 8),
                "0b" => (num.trim_start_matches("0b"), 2),
                _ => (num.as_str(), 10),
            }
        };
        match u64::from_str_radix(&num, base) {
            Err(e) => Err(Error::raw(ErrorKind::InvalidValue, e.to_string())),
            Ok(o) => Ok(o),
        }
    }
}

fn main() {
    let args = command!()
        .after_help("no reading from stdin... for now")
        .arg(
            arg!([input] "file to read")
            .required(true)
            .value_parser(value_parser!(PathBuf))
        )
        .arg(
            arg!(-o --output <output> "file to output to (default: stdout)")
            .required(false)
            .value_parser(value_parser!(PathBuf))
        )
        .arg(
            arg!(-n --bytes <bytes> "number of bytes to read (default: all)")
            .required(false)
            .value_parser(ValueParser::new(PrefixedU64ValueParser))
        )
        .arg(
            arg!(-s --skip <skip> "number of bytes to skip (default: 0)")
            .required(false)
            .value_parser(ValueParser::new(PrefixedU64ValueParser))
        )
        .arg(
            arg!(-e --end <end> "byte to stop reading on")
            .required(false)
            .value_parser(ValueParser::new(PrefixedU64ValueParser))
        )
        .get_matches();

    // input is required, unwrap shouldn't fail
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

    let (skip, bytes) = {
        let input_len =
            input.metadata().expect("error reading file metadata!").len();

        // grab all the relevant option values from clap,
        // toss out all the options that weren't specified,
        // unwrap the rest into tuples of form (option_name, option_value)
        let mut opt_stack: Vec<(&str, u64)> =
            ["skip", "bytes", "end"]
            .into_iter()
            .map(|o| (o, args.get_one::<u64>(o)))
            .filter(|(_, v)| v.is_some())
            .map(|(k, v)| (k, *v.unwrap()))
            .collect();

        {
            // the spicy fold checks for the presence of the skip and bytes
            // flags. it effectively has two accumulators, one (a_n) to hold
            // the number of values it finds, and one (a_v) to hold the value
            // of skip + bytes, should it read both opts (i.e. if a_n == 2)
            let (opt_count, skip_bytes) =
                opt_stack
                .iter()
                .fold(
                    (0, 0),
                    |(a_n, a_v), (k, v)| {
                        if *k == "skip" || *k == "bytes" { (a_n + 1, a_v + v) }
                        else { (a_n, a_v) }
                    }
                );
            if opt_count == 2 {
                opt_stack.insert(2, ("skip + bytes", skip_bytes));
            }
        }

        opt_stack.push(("input file size", input_len));
        opt_stack.sort_by(|(_, a), (_, b)| a.cmp(b));
        if let Some((k, _)) = opt_stack.pop() {
            if k != "input file size" { 
                panic!("value of {k} cannot exceed input file size!");
            }
        };

        // if "end" is not specified, we read to EOF. if it is, we read up to
        // (but not including) the specified location. this means that a valid
        // "end" value would become the new effective input length.
        let input_len = {
            if let Some(_) = opt_stack.iter().find(|(k, _)| *k == "end") {
                match opt_stack.pop().unwrap() {
                    (k, _) if k != "end" => {
                        panic!("value of {k} cannot exceed value of end!");
                    },
                    (_, v) => v,
                }
            } else { input_len }
        };

        match opt_stack.pop() {
            Some((k, _)) if k == "skip + bytes" => {
                match opt_stack[0..2] {
                    [(k, skip), (_, bytes)] if k == "skip" => (skip, bytes),
                    [(k, bytes), (_, skip)] if k == "bytes" => (skip, bytes),
                    _ => panic!("forbidden error! pls file a bug report!"),
                }
            },
            Some((k, skip)) if k == "skip" => (skip, input_len - skip),
            Some((k, bytes)) if k == "bytes" => (0, bytes),
            _ => (0, input_len),
        }
    };

    let mut output: Box<dyn Write> = {
        match args.get_one::<PathBuf>("output") {
            Some(output_path) => {
                let output_path = output_path.as_path();
                match output_path.to_str() {
                    Some(s) if s == "-" => Box::new(std::io::stdout()),
                    Some(s) => Box::new(File::create(s).unwrap()),
                    None => panic!("invalid UTF-8 in output file!"),
                }
            },
            None => Box::new(std::io::stdout()),
        }
    };
    
    slice(bytes, skip, &mut input, &mut output).unwrap();
}

fn slice<R: Read + Seek, W: Write>(
    bytes: u64, skip: u64, input: &mut R, output: &mut W
)-> std::io::Result<()> {
    let mut data = Vec::with_capacity(bytes as usize);
    {
        if skip > 0 { input.seek(SeekFrom::Start(skip))?; }
        input.take(bytes).read_to_end(&mut data)?;
    }
    output.write_all(&data)?;
    Ok(())
}
