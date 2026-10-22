# Silica system tools

Rust workspace, standard library only (no crates.io dependencies: the image builds offline).

| Crate | Binary | What |
| --- | --- | --- |
| `efi` | (library) | UEFI variables, EFI_LOAD_OPTION, GPT, ESP discovery, boot entry management |
| `boot` | `silica-boot` | `status`, `add`, `next`, `bless`, `sync` for the UEFI boot entries |
| `install` | `silica-install` | text-mode installer: `systemd-repart` plus boot entries |
| `recovery` | `silica-recovery` | numbered recovery menu on the console |

The entry layout and the update flow are described in [`docs/boot.md`](../docs/boot.md).

## Build and test

```
cd src
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
cargo build --release --locked
```

Tests never touch the host: they use fake directories for efivars, sysfs, `/dev`, `/proc` and the
ESP (builders in `efi/src/fixture.rs`). To run a tool against a fake tree by hand, set
`SILICA_ROOT=DIR` (everything is looked up below `DIR`) or pass `--efivars DIR` and `--sysfs DIR`.

## Conventions

- No `unwrap`/`expect`/`panic` outside tests; errors are messages and a non-zero exit code
  (1 for failures, 2 for usage errors).
- The only `unsafe` is the `FS_IOC_SETFLAGS` ioctl that clears the immutable flag efivarfs puts on
  variable files (`efi/src/efivars.rs`).

Run the gate with the target directory outside the tree (the image build
copies `src/` as is):
`CARGO_TARGET_DIR=/dev/shm/silica-target cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`
