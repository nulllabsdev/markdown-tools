//! Markdown formatting tools. The table aligner (`format_str` /
//! `format_directory`) lines up the columns of GitHub-style pipe tables; the
//! prose wrapper (`wrap_str` / `wrap_directory`) reflows paragraphs to a
//! display width; the graph aligner (`align_graph_str` /
//! `align_graph_directory`) normalizes ASCII flowcharts in fenced `text`
//! blocks. See `README.md` for the full specification.

mod align_graph;
mod common;
mod table;
mod wrap;

pub use align_graph::{align_graph_directory, align_graph_str};
pub use table::{format_directory, format_str};
pub use wrap::{wrap_directory, wrap_str};
