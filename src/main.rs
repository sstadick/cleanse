use bstr::{ByteSlice, ByteVec};
use color_eyre::Report;
use csv::ByteRecord;
use std::fs::File;
use std::io;
use std::io::{BufReader, BufWriter, Read, Write};
use std::iter::FromIterator;
use std::path::PathBuf;
use std::process::exit;
use structopt::{clap::AppSettings::ColoredHelp, StructOpt};
use tracing::info;
use tracing_subscriber::EnvFilter;

#[derive(Debug)]
enum CleanseChanges {
    DelimiterReplacement,
    TerminatorReplacement,
    FixedEncoding,
}

fn cleanse_field(bytes: &[u8], delim: u8, record_number: usize, field_number: usize) -> String {
    // Replace any delimiter or terminator characters
    let mut changes = vec![];
    let delim_fixed = bytes.replace((delim as char).to_string(), " ");
    if delim_fixed != bytes {
        changes.push(CleanseChanges::DelimiterReplacement);
    }
    let term_fixed = delim_fixed.replace("\n", " ");
    if term_fixed != delim_fixed {
        changes.push(CleanseChanges::TerminatorReplacement);
    }
    // Fix encoding
    let str = match term_fixed.into_string() {
        Ok(new_string) => new_string,
        Err(e @ bstr::FromUtf8Error { .. }) => {
            changes.push(CleanseChanges::FixedEncoding);
            e.into_vec().into_string_lossy()
        }
    };
    if !changes.is_empty() {
        info!(
            "Record number {}, field number {}: {:?}",
            record_number, field_number, changes
        );
    }
    str
}

fn get_input(path: PathBuf) -> Result<Box<dyn Read>, Report> {
    let reader: Box<dyn Read> = if path.as_os_str() == "-" {
        Box::new(BufReader::new(io::stdin()))
    } else {
        Box::new(BufReader::new(File::open(path)?))
    };
    Ok(reader)
}

fn get_output(path: PathBuf) -> Result<Box<dyn Write>, Report> {
    let writer: Box<dyn Write> = if path.as_os_str() == "-" {
        Box::new(BufWriter::new(io::stdout()))
    } else {
        Box::new(BufWriter::new(File::create(path)?))
    };
    Ok(writer)
}

/// Check if err is a broken pipe.
#[inline]
fn is_broken_pipe(err: &Report) -> bool {
    if let Some(io_err) = err.root_cause().downcast_ref::<io::Error>() {
        if io_err.kind() == io::ErrorKind::BrokenPipe {
            return true;
        }
    }
    false
}

/// A small program to do clean up delimited data.
///
/// For each field in each record this will do the following:
///
/// 1. Remove the delimiter from inside any quoted fields
/// 2. Remove the terminator from inside any quoted fields
/// 3. Fix any non-UTF8 encodings
#[derive(StructOpt, Debug)]
#[structopt(name = "cleanse", author, global_setting(ColoredHelp))]
struct Opts {
    /// Delimiter to use for parsing the file, must be a single byte.
    #[structopt(short, long, default_value = "\t")]
    delimiter: String,

    /// Output path to write to, "-" to write to stdout
    #[structopt(short, long)]
    output: PathBuf,

    /// Input file to read from, "-" to read from stdin
    #[structopt(name = "FILE", parse(from_os_str))]
    file: PathBuf,
}

fn main() -> Result<(), Report> {
    let opts = setup()?;
    if opts.delimiter.as_bytes().len() != 1 {
        return Err(Report::msg("Input delimiter may only be a single byte"));
    }

    if let Err(err) = run(
        get_input(opts.file)?,
        get_output(opts.output)?,
        opts.delimiter.as_bytes()[0],
    ) {
        if is_broken_pipe(&err) {
            exit(0)
        }
        return Err(err);
    }
    Ok(())
}

/// Run the program, returning any found errors
fn run<R, W>(input: R, output: W, delimiter: u8) -> Result<(), Report>
where
    R: Read,
    W: Write,
{
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(delimiter)
        .from_reader(input);

    let mut writer = csv::WriterBuilder::new()
        .has_headers(false)
        .delimiter(delimiter)
        .from_writer(output);

    for (record_number, record) in reader.byte_records().enumerate() {
        let record = record?;
        let record = record
            .into_iter()
            .enumerate()
            .map(|(field_number, field)| {
                cleanse_field(field, delimiter, record_number, field_number)
            })
            .collect();
        writer.write_byte_record(&record)?;
    }
    Ok(())
}

/// Parse args and set up logging / tracing
fn setup() -> Result<Opts, Report> {
    if std::env::var("RUST_LIB_BACKTRACE").is_err() {
        std::env::set_var("RUST_LIB_BACKTRACE", "1")
    }
    color_eyre::install()?;

    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info")
    }
    tracing_subscriber::fmt::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    Ok(Opts::from_args())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_simple() {
        let mut input = b"\
        a,b,c,d\n\
        1,\"2,3\",4,5\n\
        this,is,\"a\n\
        very gross\",li\xffe\n"
            .to_vec();

        let expected = String::from(
            "\
        a,b,c,d\n\
        1,2 3,4,5\n\
        this,is,a very gross,lie\n",
        );

        let mut writer = vec![];
        run(input.as_slice(), &mut writer, b',').unwrap();
        assert_eq!(expected, writer.into_string().unwrap());
    }
}
