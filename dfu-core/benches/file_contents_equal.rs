use std::io::Write;
use std::fs::File;
use std::path::PathBuf;

use criterion::{criterion_main, criterion_group, Criterion};

use dfu_core::files::file_contents_equal;

fn create_tmp_file() -> Result<(File, PathBuf), tempfile::PersistError> {
    let mut f = tempfile::NamedTempFile::new().expect("failed creating temporary file");
    let mut payload: Vec<u8> = (0..=255).cycle().take(1024*1024).collect();
    f.write_all(&mut payload).expect("failed to write payload to temporary file");

    f.keep()
}

fn bench_file_contents_equal(c: &mut Criterion) {
    let (_handle, path) = create_tmp_file().expect("failed creating 1M temporary file");

    c.bench_function("file_contents_equal", |b| b.iter(|| {
        file_contents_equal(&path, &path).unwrap();
    }));
}

criterion_group!(benches, bench_file_contents_equal);
criterion_main!(benches);