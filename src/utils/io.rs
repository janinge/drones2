use std::io;
use std::path::Path;

use clap::Parser;

#[derive(Parser)]
pub struct Args {
    /// Path to a directory containing problem files, or a base path for problem files
    #[arg(short, long)]
    prefix: Option<String>,

    /// Path to one or more problem files
    #[arg(short, long)]
    file: Option<Vec<String>>,

    /// Number of runs to perform with equal parameters
    #[arg(short, long, default_value_t = 1)]
    pub runs: u32,

    /// Maximum running time in seconds
    #[arg(short, long)]
    pub time_limit: Option<u32>,

    /// Temperature at start
    #[arg(short, long)]
    pub t0: Option<f32>,

    /// Temperature at end
    #[arg(short, long, default_value_t = 10.0)]
    pub t_final: f32,
}

pub fn enumerate_input_files(args: &Args) -> io::Result<Vec<std::path::PathBuf>> {
    if let Some(files) = &args.file {
        if let Some(prefix) = &args.prefix {
            Ok(files.iter()
                .map(|f| Path::new(prefix).join(f))
                .collect())
        } else {
            Ok(files.iter()
                .map(|f| Path::new(f).to_path_buf())
                .collect())
        }
    } else if let Some(prefix) = &args.prefix {
        let dir_entries = std::fs::read_dir(prefix)?;
        let mut files = Vec::new();
        for entry in dir_entries {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                files.push(path);
            }
        }

        files.sort_by(|a, b| {
            fn split_parts(s: &str) -> Vec<Result<u64, String>> {
                let mut parts = Vec::new();
                let mut buf = String::new();
                let mut is_digit = None;

                for c in s.chars() {
                    let c = if c == '_' { ' ' } else { c.to_ascii_lowercase() };
                    let current_is_digit = c.is_ascii_digit();

                    match is_digit {
                        Some(prev) if prev != current_is_digit => {
                            if prev {
                                parts.push(Ok(buf.parse::<u64>().unwrap()));
                            } else {
                                parts.push(Err(buf.clone()));
                            }
                            buf.clear();
                        }
                        _ => {}
                    }
                    buf.push(c);
                    is_digit = Some(current_is_digit);
                }

                if !buf.is_empty() {
                    if is_digit == Some(true) {
                        parts.push(Ok(buf.parse::<u64>().unwrap()));
                    } else {
                        parts.push(Err(buf));
                    }
                }

                parts
            }

            let a_key = a.file_name().and_then(|n| n.to_str()).map(split_parts).unwrap_or_default();
            let b_key = b.file_name().and_then(|n| n.to_str()).map(split_parts).unwrap_or_default();

            a_key.cmp(&b_key)
        });

        Ok(files)
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Either --file and/or --prefix must be provided",
        ))
    }
}