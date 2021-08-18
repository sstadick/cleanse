use bstr::{ByteSlice, ByteVec};
use color_eyre::Report;
use std::path::PathBuf;
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

#[derive(StructOpt, Debug)]
#[structopt(name = "cleanse", author, global_setting(ColoredHelp))]
struct Opts {
    #[structopt(short, long, default_value = "\t")]
    delimiter: String,

    #[structopt(short, long)]
    output: PathBuf,

    #[structopt(name = "FILE", parse(from_os_str))]
    file: PathBuf,
}

fn main() -> Result<(), Report> {
    let opts = setup()?;
    if opts.delimiter.as_bytes().len() != 1 {
        return Err(Report::msg("Input delimiter may only be a single byte"));
    }
    info!("Delim is {:?}", opts.delimiter.as_bytes());
    let mut writer = csv::WriterBuilder::new()
        .delimiter(opts.delimiter.as_bytes()[0])
        .from_path(opts.output)?;

    let mut reader = csv::ReaderBuilder::new()
        .delimiter(opts.delimiter.as_bytes()[0])
        .from_path(opts.file)?;

    let delim = opts.delimiter.as_bytes()[0];
    for (record_number, record) in reader.byte_records().enumerate() {
        let record = record?;
        record
            .iter()
            .enumerate()
            .map(|(field_number, field)| cleanse_field(field, delim, record_number, field_number))
            .try_for_each(|field| writer.write_field(field))?;
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
