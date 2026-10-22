kind: manual
description: |
  Silica's system tools (src/): silica-boot (UEFI boot entries, A/B
  updates), silica-install and silica-recovery.

build-depends:
- freedesktop-sdk.bst:public-stacks/runtime-gnu.bst
- freedesktop-sdk.bst:components/rust.bst

depends:
- freedesktop-sdk.bst:components/systemd.bst

environment:
  CARGO_HOME: '%{build-root}/.cargo'

variables:
  strip-binaries: ''

config:
  build-commands:
  - cargo build --offline --locked --release

  install-commands:
  - |
    for tool in silica-boot silica-install silica-recovery; do
      install -Dm755 "target/release/${tool}" "%{install-root}%{libexecdir}/silica/${tool}"
      ln -s "../libexec/silica/${tool}" "%{install-root}%{bindir}/${tool}" 2>/dev/null || {
        mkdir -p "%{install-root}%{bindir}"
        ln -s "../libexec/silica/${tool}" "%{install-root}%{bindir}/${tool}"
      }
    done

sources:
- kind: local
  path: src
