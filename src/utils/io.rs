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

    /// Maximum running time in seconds per instance run
    #[arg(short, long)]
    pub time_limit: Option<u32>,

    /// Temperature at start (T0)
    #[arg(long)]
    pub t0: Option<f32>,

    /// Temperature at end (T_final)
    #[arg(long, default_value_t = 10.0)]
    pub t_final: f32,

    // Removal Operator Parameters
    /// Ratio of calls to select for removal
    #[arg(long, default_value_t = 0.95)]
    pub removal_selection_ratio: f32,

    /// Bias towards assignments with more calls (0.0 to 1.0)
    #[arg(long, default_value_t = 0.90)]
    pub removal_assignment_bias: f32,

    /// Minimum number of calls to remove
    #[arg(long, default_value_t = 1)]
    pub removal_min_removals: usize,

    /// Maximum number of calls to remove
    #[arg(long, default_value_t = 5)]
    pub removal_max_removals: usize,
    
    /// Optional delay in seconds to print the current best solution after it improved
    #[arg(long)]
    pub print_best_delay: Option<u32>,
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