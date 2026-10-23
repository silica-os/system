kind: script
description: |
  Development extension: AppKit's library and the Kit Gallery, for
  tools/dev-sysext, which builds them from a workspace of the appkit checkout.

(@): elements/silica/extensions/sysext.yml

build-depends:
  (>):
  - filename: silica/desktop/appkit.bst
    config:
      location: /payload

variables:
  sysext-name: silica-appkit-dev
  sysext-paths: >-
    usr/bin/appkit-gallery
    usr/bin/appkit-extras
    usr/bin/appkit-touchpad
    usr/bin/appkit-materials
    usr/lib/x86_64-linux-gnu/libappkit.so
