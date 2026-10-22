kind: script
description: |
  Test extension: silica-test-client, the Wayland client tools/test-vm maps
  on the compositor. Never part of the system image.

(@): elements/silica/extensions/sysext.yml

build-depends:
  (>):
  - filename: silica/desktop/compositor.bst
    config:
      location: /payload

variables:
  sysext-name: silica-compositor-tests
  sysext-paths: usr/libexec/silica/tests
