kind: script
description: |
  Test extension: GTK 3, GTK 4 and Qt 6 apps, with the libraries the image
  lacks, for the real-app checks of tools/test-vm --desktop. Never part of the
  system image.

(@): elements/silica/extensions/sysext.yml

build-depends:
  (>):
  - filename: silica/test-apps/payload.bst
    config:
      location: /payload

variables:
  sysext-name: silica-compositor-apps
  sysext-paths: usr
