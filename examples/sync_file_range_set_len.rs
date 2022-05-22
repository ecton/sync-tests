use std::{
    env::args,
    fs::{File, OpenOptions},
    io::{ErrorKind, Write},
    os::unix::io::AsRawFd,
    path::{Path, PathBuf},
};

use rand::{thread_rng, Rng};

fn main() {
    let path = PathBuf::from(args().nth(1).unwrap());
    let verification_path = Path::new("verification");
    verify_previous_operation(verification_path, &path);

    if !path.exists() || !verification_path.exists() {
        // Create both files, and set their lengths to 4096 bytes. This will
        // fill them with 0s once they're fully synchronized.
        let file = File::create(&path).unwrap();
        file.set_len(4096).unwrap();
        file.sync_all().unwrap();
        drop(file);
        let file = File::create(verification_path).unwrap();
        file.set_len(4096).unwrap();
        file.sync_all().unwrap();
        drop(file);

        // Because at least one of these files didn't exist, we need to also
        // sync the directory's metadata to ensure the file still exists after
        // the poweroff this script will cause.
        let file = File::open(".").unwrap();
        file.sync_all().unwrap();
        let file = File::open(path.parent().unwrap()).unwrap();
        file.sync_all().unwrap();
    }

    // Generate a random page of data
    let mut rng = thread_rng();
    let mut data = vec![0; 4096];
    for byte in &mut data {
        *byte = rng.gen();
    }

    // Save the verification file.
    let mut file = OpenOptions::new()
        .write(true)
        .open(verification_path)
        .unwrap();
    file.write_all(&data).unwrap();
    // Use sync_all to ensure the file is fully synced. `sync_data` should be
    // safe, but since that is also being tested in this suite, we want to be as
    // safe as we can with this file's synchronization. In theory, they should
    // be the same due to this being a full, single-page modification.
    file.sync_all().unwrap();
    drop(file);

    // Pre-open /proc/sysrq-trigger
    let mut sysrq_trigger = OpenOptions::new()
        .write(true)
        .open("/proc/sysrq-trigger")
        .unwrap();

    // Write the data to the file.
    let mut file = OpenOptions::new().write(true).open(path).unwrap();
    file.write_all(&data).unwrap();

    // Use `sync_file_range` with the parameters that the man page says: "This
    // is a write-for-data-integrity operation that will ensure that all pages
    // in the specified range which were dirty when sync_file_range() was called
    // are committed to disk."
    let result = unsafe {
        libc::sync_file_range(
            file.as_raw_fd(),
            0,
            4096,
            libc::SYNC_FILE_RANGE_WAIT_BEFORE
                | libc::SYNC_FILE_RANGE_WRITE
                | libc::SYNC_FILE_RANGE_WAIT_AFTER,
        )
    };
    // Now that `sync_file_range` has returned, the data should be flushed on
    // disk if the filesystem supports the operation.
    assert_eq!(result, 0);

    // Tell the kernel to power off immediately.
    sysrq_trigger.write_all(b"o").unwrap();
    sysrq_trigger.flush().unwrap();
    // Despite telling it to do it immediately, the program is still running. To
    // simulate a true power failure, we will use std::process::exit to
    // immediately end the process with no cleanup.
    std::process::exit(0);
}

fn verify_previous_operation(verification_path: &Path, path: &Path) {
    let expected_contents = match std::fs::read(verification_path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == ErrorKind::NotFound => {
            println!("No previous operation found to verify.");
            return;
        }
        Err(other) => panic!("Error reading verification file: {other:?}"),
    };

    let data = std::fs::read(path).unwrap();
    if data != expected_contents {
        println!(
            "Operation verification failed. Got {:?}, expected {:?}",
            data, expected_contents
        );
    } else {
        println!("Previous operation verified. {}", data.len());
    }
}
