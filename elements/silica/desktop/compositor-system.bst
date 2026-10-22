kind: filter
description: |
  The compositor as the system image carries it: without the "tests"
  domain (silica-test-client), which only the test extension ships.

build-depends:
- silica/desktop/compositor.bst

runtime-depends:
- freedesktop-sdk.bst:components/libinput.bst
- freedesktop-sdk.bst:components/libseat.bst
- freedesktop-sdk.bst:components/libxkbcommon.bst
- freedesktop-sdk.bst:components/pixman.bst
- freedesktop-sdk.bst:components/libglvnd.bst
- freedesktop-sdk.bst:vm/mesa-default.bst

config:
  exclude:
  - tests
  include-orphans: true
