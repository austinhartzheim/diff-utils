//! Diff directories of directories, finding shared directories at depth one.
//!
//! # Use Case
//! You have a backup directory containing a set of projects and you have a "live" directory
//! containing possibly-modified versions of the same projects. You want to know which projects
//! differ from the backup. Also, you want to know which projects are no longer present.
//!
//! ```
//! backup/
//!   project-1/
//!     README.txt
//!   project-2/
//!     README.txt
//!   project-3/
//!     README.txt
//! live/
//!   project-1/
//!     README.txt
//!   project-2/
//!     SOMETHING-ELSE.txt
//!   project-4/
//!     README.txt
//! ```
//! ```
//! $ diff-utils-dir-trees ./backup ./live
//! ./backup                  ./live
//! project-1     MATCHES     project-1
//! project-2     DIFFERS     project-2
//! project-3   < ONLY IN
//!               ONLY IN >   project-4
//! ```

use clap::{App, Arg};
use std::fmt;
use std::io::{self, Write};
use std::path::Path;

use dfu_core::directories::{self, TreeDiff};

fn main() {
    let args = App::new("diff-utils-dir-trees")
        .arg(Arg::with_name("dir1").required(true).takes_value(true))
        .arg(Arg::with_name("dir2").required(true).takes_value(true))
        .get_matches();
    let (path1, path2) = (
        Path::new(args.value_of("dir1").unwrap()),
        Path::new(args.value_of("dir2").unwrap()),
    );

    // For pretty table formatting, we need to know the name of the longest identifier.
    let col_width = std::cmp::max(
        directories::longest_identifier(path1).expect("failed to access dir1"),
        directories::longest_identifier(path2).expect("failed to access dir2"),
    );
    println!(
        "{1:0$}    -------    {2:0$}",
        col_width,
        path1.display(),
        path2.display()
    );

    for res in directories::diff_trees(&path1, &path2, 1) {
        match res {
            Ok(td) => println!("{}", display_tree_diff(&td, col_width)),
            // Once an error is encountered, scanning must be stopped to ensure accurate results.
            Err(e) => {
                println!("ERROR: Encountered an error while scanning directories:");
                println!("  {}", e);
                println!("Aborting directory scan.");
                break;
            }
        }
    }
}

fn display_tree_diff(td: &TreeDiff, width: usize) -> String {
    let empty = Path::new("").display();
    let (l, c, r) = match td {
        TreeDiff::Differs(l, r) => (l.display(), "  DIFFERS  ", r.display()),
        TreeDiff::Matches(l, r) => (l.display(), "  MATCHES  ", r.display()),
        TreeDiff::Left(l) => (l.display(), "< ONLY IN  ", empty),
        TreeDiff::Right(r) => (empty, "  ONLY IN >", r.display()),
    };
    format!("{1:0$}  {2}  {3:0$}", width, l, c, r)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_tree_diff_samples() {
        let cases = [
            (
                TreeDiff::Left("./test".into()),
                "./test  < ONLY IN          ",
            ),
            (
                TreeDiff::Right("./test".into()),
                "          ONLY IN >  ./test",
            ),
            (
                TreeDiff::Matches("./test".into(), "./test".into()),
                "./test    MATCHES    ./test",
            ),
            (
                TreeDiff::Differs("./test".into(), "./test".into()),
                "./test    DIFFERS    ./test",
            ),
        ];

        for (td, expected) in cases {
            assert_eq!(&display_tree_diff(&td, 6), expected);
        }
    }
}
