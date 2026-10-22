kind: junction

sources:
- kind: git
  url: gitlab:freedesktop-sdk/freedesktop-sdk.git
  track: release/26.08
  ref: d0c0b597d58f8cc14ee26c427aec1be13c6a313e

config:
  options:
    target_arch: '%{target_arch}'
    minimal_vm:
    - firmware
    - locale
