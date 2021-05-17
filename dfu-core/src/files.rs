use std::{
    fs::File,
    io::{self, Read},
    path::Path,
};

pub fn file_contents_equal<P1: AsRef<Path>, P2: AsRef<Path>>(
    file1: P1,
    file2: P2,
) -> Result<bool, io::Error> {
    // If file lengths differ, the file contents differ.
    if file1.as_ref().metadata()?.len() != file2.as_ref().metadata()?.len() {
        return Ok(false);
    }

    let mut f1 = File::open(file1)?.bytes();
    let mut f2 = File::open(file2)?.bytes();

    loop {
        match (f1.next(), f2.next()) {
            // Both files have remaining bytes
            (Some(Ok(b1)), Some(Ok(b2))) => {
                if b1 != b2 {
                    return Ok(false);
                }
            }
            // One of the files is longer
            (Some(Ok(_)), None) => {
                return Ok(false);
            }
            (None, Some(Ok(_))) => {
                return Ok(false);
            }
            // One of the iterators yields an error
            (Some(Err(e)), _) => {
                return Err(e);
            }
            (_, Some(Err(e))) => {
                return Err(e);
            }
            // Both iterators end at the same point
            (None, None) => {
                return Ok(true);
            }
        }
    }
}
