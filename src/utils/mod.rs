mod interval_tree;
pub mod matrix;
mod io;

pub use interval_tree::IntervalTree;
pub use matrix::Matrix2;
pub use matrix::Matrix3;

pub use io::{Args, enumerate_input_files};
pub use clap::Parser;
