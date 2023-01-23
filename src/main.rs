use std::io::prelude::*;
use std::io::{SeekFrom};
use std::fs::File;
use std::path::PathBuf;

use clap::{arg, command, value_parser};


fn main() {
    let args = command!()
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
            .value_parser(value_parser!(u64)),
        )
        .arg(
            arg!(-s --skip <skip> "number of bytes to skip (default: 0)")
            .required(false)
            .value_parser(value_parser!(u64)),
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

    let output = args.get_one::<PathBuf>("output");
    match output {
        Some(output) => {
            let mut output =
                File::create(output)
                .expect("error creating output file!");
            slice(bytes, skip, &mut input, &mut output).unwrap();
        },
        None => {
            slice(bytes, skip, &mut input, &mut std::io::stdout()).unwrap();
        },
    };
}

fn slice<R: Read + Seek, W: Write>
(bytes: u64, skip: u64, input: &mut R, output: &mut W)
-> std::io::Result<()> {
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
