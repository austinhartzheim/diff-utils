use std::{
    cmp::Ordering,
    io,
    iter::Peekable,
    path::{Path, PathBuf},
};
use walkdir::{DirEntry, WalkDir};

#[derive(Error, Debug)]
pub enum DirError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("failed while walking directories: {0}")]
    Walk(#[from] walkdir::Error),
}

/// Finds the length of the longest identifier (including `path`) in `path`'s descendants.
pub fn longest_identifier<P: AsRef<Path>>(path: P) -> Result<usize, walkdir::Error> {
    WalkDir::new(path)
        .max_depth(1)
        .into_iter()
        .try_fold(0, |acc, entry_res| {
            entry_res.map(|entry| std::cmp::max(acc, entry.path().as_os_str().len()))
        })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TreeDiff {
    Left(PathBuf),
    Right(PathBuf),
    Matches(PathBuf, PathBuf),
    Differs(PathBuf, PathBuf),
}

pub fn diff_trees<P1: AsRef<Path>, P2: AsRef<Path>>(
    dir1: &P1,
    dir2: &P2,
    depth: usize,
) -> TreeDiffIter<impl Iterator<Item=Result<DirEntry, walkdir::Error>>> {
    let walker1 = WalkDir::new(dir1)
        .min_depth(depth)
        .max_depth(depth)
        .sort_by(file_name_cmp)
        .into_iter()
        .peekable();
    let walker2 = WalkDir::new(dir1)
        .min_depth(depth)
        .max_depth(depth)
        .sort_by(file_name_cmp)
        .into_iter()
        .peekable();

    TreeDiffIter { walker1, walker2 }
}

// # Warning: Error handling
// If this iterator yields an error, future calls may yield incorrect results.
//
// For example, if `dir1/a` cannot be accessed but `dir2/a` can, this iterator will yield an error
// indicating that `dir1/a` cannot be accessed, followed by a record indicating that `dir2/a` only
// exists in `dir2` (despite the fact that it may exist in `dir1`, but we don't have permission to
// access it).
pub struct TreeDiffIter<I: Iterator> {
    walker1: Peekable<I>,
    walker2: Peekable<I>,
}
impl<I> Iterator for TreeDiffIter<I>
where
    I: Iterator<Item = Result<DirEntry, walkdir::Error>>,
{
    type Item = Result<TreeDiff, DirError>;

    fn next(&mut self) -> Option<Self::Item> {
        match (self.walker1.peek(), self.walker2.peek()) {
            // Both iterators yield entries. Because the iterators are sorted, we can first check
            // for missing entries (i.e., names are not equal). If the names are equal, then we
            // need to scan the directories for equality.
            (Some(Ok(de1)), Some(Ok(de2))) => {
                match file_name_cmp(de1, de2) {
                    // Names are equal. We need to scan the contents of both directories.
                    Ordering::Equal => match diff_dirs(de1.path(), de2.path()) {
                        Ok(DiffResult::Equal) => {
                            Some(Ok(TreeDiff::Matches(self.walker1.next().unwrap().unwrap().into_path(), self.walker2.next().unwrap().unwrap().into_path())))
                        },
                        Ok(DiffResult::NotEqual) => {
                            Some(Ok(TreeDiff::Differs(self.walker1.next().unwrap().unwrap().into_path(), self.walker2.next().unwrap().unwrap().into_path())))
                        },
                        Err(e) => Some(Err(e))
                    }
                    // `de1` is later than `de2`. This means that `de2` was not found in `dir1`.
                    Ordering::Greater => Some(Ok(TreeDiff::Left(
                        self.walker2.next().unwrap().unwrap().into_path(),
                    ))),
                    // `de2` is later than `de1`. This means that `de1` was not found in `dir2`.
                    Ordering::Less => Some(Ok(TreeDiff::Left(
                        self.walker1.next().unwrap().unwrap().into_path(),
                    ))),
                }
            }

            // One iterator has completed while the other continues to yield entries. In this
            // case, mark the entry as only existing in either the left or right tree.
            (Some(Ok(_de1)), None) => Some(Ok(TreeDiff::Left(
                self.walker1.next().unwrap().unwrap().into_path(),
            ))),
            (None, Some(Ok(_de2))) => Some(Ok(TreeDiff::Left(
                self.walker2.next().unwrap().unwrap().into_path(),
            ))),

            // If either iterator yields an error, we yield that error.
            (Some(Err(_e)), _) => {
                Some(Err(self.walker1.next().unwrap().unwrap_err().into()))
            }
            (_, Some(Err(_e))) => {
                Some(Err(self.walker2.next().unwrap().unwrap_err().into()))
            }

            // Both iterators are complete.
            (None, None) => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DiffResult {
    Equal,
    NotEqual,
}

pub fn diff_dirs<P1: AsRef<Path>, P2: AsRef<Path>>(dir1: P1, dir2: P2) -> Result<DiffResult, DirError> {
    let mut walker1 = WalkDir::new(dir1)
        .sort_by(file_name_cmp)
        .into_iter()
        .peekable();
    let mut walker2 = WalkDir::new(dir2)
        .sort_by(file_name_cmp)
        .into_iter()
        .peekable();
    
    loop {
        match (walker1.peek(), walker2.peek()) {
            // Both iterators yield entries. Because the iterators are sorted, we can first check
            // for missing entries (i.e., names are not equal). If the names are equal, then we
            // need to scan the directories for equality.
            (Some(Ok(de1)), Some(Ok(de2))) => {
                match file_name_cmp(de1, de2) {
                    // Names are equal. We need to scan the contents of both directories.
                    Ordering::Equal => {
                        if de1.file_type() != de2.file_type() {
                            return Ok(DiffResult::NotEqual);
                        }
                        if de1.file_type().is_file() && !crate::files::file_contents_equal(de1.path(), de2.path())? {
                            return Ok(DiffResult::NotEqual);
                        }
                        walker1.next();
                        walker2.next();
                    }
                    // `de1` is later than `de2`. This means that `de2` was not found in `dir1`.
                    Ordering::Greater => {return Ok(DiffResult::NotEqual);}
                    // `de2` is later than `de1`. This means that `de1` was not found in `dir2`.
                    Ordering::Less => {return Ok(DiffResult::NotEqual);}
                }
            }

            // One iterator has completed while the other continues to yield entries. In this
            // case, mark the entry as only existing in either the left or right tree.
            (Some(Ok(_de1)), None) => {return Ok(DiffResult::NotEqual);}
            (None, Some(Ok(_de2))) => {return Ok(DiffResult::NotEqual);}

            // If either iterator yields an error, we yield that error.
            (Some(Err(_e)), _) => {
                walker1.next().unwrap()?;
            }
            (_, Some(Err(_e))) => {
                walker2.next().unwrap()?;
            }

            // Both iterators are complete.
            (None, None) => return Ok(DiffResult::Equal),
        }
    }
}

fn file_name_cmp(a: &DirEntry, b: &DirEntry) -> Ordering {
    a.file_name().cmp(&b.file_name())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_trees_sample() {
        for res in diff_trees(&"./", &"./", 1) {
            match res {
                Ok(td) => println!("{:?}", td),
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
}