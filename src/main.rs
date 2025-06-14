use std::io::prelude::*;
use std::io::SeekFrom;
use std::ffi::OsStr;
use std::fs::File;
use std::path::PathBuf;

use clap::{Arg, Command, CommandFactory, Parser};
use clap::builder::{StringValueParser, TypedValueParser};
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

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
#[command(after_help = "no reading from stdin... for now")]
struct Args {
    /// path of the file to read
    input: PathBuf,

    /// file to output to. default: stdout
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// number of bytes to read. default: all
    #[arg(short = 'n', long, value_parser = PrefixedU64ValueParser)]
    bytes: Option<u64>,

    /// byte to start reading at (inclusive). default: 0
    #[arg(short, long, value_parser = PrefixedU64ValueParser)]
    start: Option<u64>,

    /// byte to stop reading at (exclusive). default: last byte
    #[arg(short, long, value_parser = PrefixedU64ValueParser)]
    end: Option<u64>,
}

fn main() {
    let args = Args::command().get_matches();

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

    let (start, bytes) = {
        let input_len =
            input.metadata().expect("error reading file metadata!").len();

        // grab all the relevant option values from clap,
        // toss out all the options that weren't specified,
        // unwrap the rest into tuples of form (option_name, option_value)
        let mut opt_stack: Vec<(&str, u64)> =
            ["start", "bytes", "end"]
            .into_iter()
            .map(|o| (o, args.get_one::<u64>(o)))
            .filter(|(_, v)| v.is_some())
            .map(|(k, v)| (k, *v.unwrap()))
            .collect();

        {
            // the spicy fold checks for the presence of the start and bytes
            // flags. it effectively has two accumulators, one (a_n) to hold
            // the number of values it finds, and one (a_v) to hold the value
            // of start + bytes, should it read both opts (i.e. if a_n == 2)
            let (opt_count, start_bytes) =
                opt_stack
                .iter()
                .fold(
                    (0, 0),
                    |(a_n, a_v), (k, v)| {
                        if *k == "start" || *k == "bytes" { (a_n + 1, a_v + v) }
                        else { (a_n, a_v) }
                    }
                );
            if opt_count == 2 {
                opt_stack.insert(2, ("start + bytes", start_bytes));
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
            Some((k, _)) if k == "start + bytes" => {
                match opt_stack[0..2] {
                    [(k, start), (_, bytes)] if k == "start" => (start, bytes),
                    [(k, bytes), (_, start)] if k == "bytes" => (start, bytes),
                    _ => panic!("forbidden error! pls file a bug report!"),
                }
            },
            Some((k, start)) if k == "start" => (start, input_len - start),
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

    slice(bytes, start, &mut input, &mut output).unwrap();
}

/// Reads `bytes` bytes from the stream `input`, starting at byte `start` into
/// the stream `output`.
fn slice<R: Read + Seek, W: Write>(
    bytes: u64, start: u64, input: &mut R, output: &mut W
)-> std::io::Result<()> {
    let mut data = Vec::with_capacity(bytes as usize);
    {
        if start > 0 { input.seek(SeekFrom::Start(start))?; }
        input.take(bytes).read_to_end(&mut data)?;
    }
    output.write_all(&data)?;
    Ok(())
}
