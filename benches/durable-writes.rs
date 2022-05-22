use std::{
    io::{Seek, SeekFrom, Write},
    os::unix::io::AsRawFd,
};

use criterion::{criterion_group, criterion_main, Criterion};
use tempfile::NamedTempFile;

const WRITE_SIZE: usize = 1024;
const PREALLOC_SIZE: usize = 16 * WRITE_SIZE;

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("writes");

    let data = vec![42; WRITE_SIZE];
    let zeroes = vec![0; PREALLOC_SIZE];
    group.bench_function("append", |b| {
        let mut file = NamedTempFile::new_in(".").unwrap();
        let file = file.as_file_mut();
        b.iter(|| {
            file.write_all(&data).unwrap();
            file.sync_data().unwrap();
        });
    });
    group.bench_function("preallocate", |b| {
        let mut file = NamedTempFile::new_in(".").unwrap();
        let file = file.as_file_mut();
        let mut file_length = 0;
        let mut allocated_length = 0;
        b.iter(|| {
            let new_length = file_length + data.len();
            file.write_all(&data).unwrap();
            if new_length > allocated_length {
                allocated_length += PREALLOC_SIZE;
                file.set_len(allocated_length as u64).unwrap();
            }
            file.sync_data().unwrap();
            file_length = new_length;
        });
    });
    group.bench_function("syncrange", |b| {
        let mut file = NamedTempFile::new_in(".").unwrap();
        let file = file.as_file_mut();
        let mut file_length = 0;
        let mut allocated_length = 0;
        b.iter(|| {
            let new_length = file_length + data.len();
            file.write_all(&data).unwrap();
            match new_length.checked_sub(allocated_length) {
                Some(zeroes_needed) if zeroes_needed > 0 => {
                    allocated_length += PREALLOC_SIZE;
                    file.write_all(&zeroes).unwrap();
                    file.sync_data().unwrap();
                    file.seek(SeekFrom::Start(new_length as u64)).unwrap();
                }
                _ => {
                    let result = unsafe {
                        libc::sync_file_range(
                            file.as_raw_fd(),
                            file_length as i64,
                            data.len() as i64,
                            libc::SYNC_FILE_RANGE_WAIT_BEFORE
                                | libc::SYNC_FILE_RANGE_WRITE
                                | libc::SYNC_FILE_RANGE_WAIT_AFTER,
                        )
                    };
                    assert_eq!(result, 0);
                }
            }
            file_length = new_length;
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
