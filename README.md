# sync-tests

## Benchmark

This project began as an investigation into the relative speed of:

- `append`: appending with each write and calling `sync_data()`
- `preallocate`: when more space is needed for a write, grow the file by a
  larger amount than needed, then call `sync_data()` to flush the new file
  contents to disk.
- `syncrange`: The same strategy as `preallocate`, but instead of calling
  `sync_data()`, call `sync_file_range()`.

### @ecton's personal results

```sh
$ cargo bench
writes/append           time:   [2.6331 ms 2.6682 ms 2.7124 ms]
writes/preallocate      time:   [1.2481 ms 1.2599 ms 1.2749 ms]
writes/syncrange        time:   [191.45 us 193.74 us 196.35 us]
```

## Testing Data Loss Examples

Build the examples:

- `cargo build --examples -r`

Ensure the build artifacts are synchronized.

- `sync .`
- `sync target`
- `sync target/release`
- `sync target/release/examples`
- `sync target/release/examples/*`

Run each example multiple times inside of a virtual machine. Each example
requires permissions to write to `/proc/sysrq-trigger`, as they write `o` to
trigger an immediate power off in the kernel.

## @ecton's personal results

- OS: Ubuntu 20.04.4 LTS
- Kernel: 5.4.0-110-generic
- Architecture: x86_64
- Host: QEMU

### sync_data

This example tests using `File::sync_data()`. Under the hood, this translates to
an `fdatasync()`. Due to how this is documented on Linux, I expected no failures
and experienced none on any of the filesystems tested.

### sync_file_range

This example tests using `sync_file_range` on a pre-allocated file. When
initializing the file, the file is manually filled with zeroes rather than
relying on `File::set_len` to do it for us.

Based on what I had read before starting this test, I was hoping to prove that
the implementation on `ext4` was able to provide durable writes. My results:

| Filesystem | Write Persisted |
|------------|-----------------|
| btrfs      | No              |
| ext4       | Yes             |
| xfs        | Yes             |
| zfs        | No              |

### sync_file_range_set_len

This example tests using `sync_file_range` on a pre-allocated file. When
initializing the file, `File::set_len` is called which uses `ftruncate` under
the hood. `ftruncate`'s documentation does not claim to fill the new space with
zeroes, but rather, "If the file size is increased, the extended area shall
appear as if it were zero-filled."

This test's only interesting output compared to `sync_file_range` is the initial
verification, as all subsequent verifications would match the results of the
`sync_file_range` example.

My findings are that this example fails to verify the first operation on all
filesystems. Thus, to safely extend the file, the file must be manually filled
for the new data to persist.
