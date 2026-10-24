kind: script
description: |
  Development extension: the Dock over the one in the system image (or in an
  image without it), for tools/dev-sysext, which builds it from a workspace of
  the dock checkout.

(@): elements/silica/extensions/sysext.yml

build-depends:
  (>):
  - filename: silica/desktop/dock.bst
    config:
      location: /payload

variables:
  sysext-name: silica-dock-dev
  sysext-paths: usr/bin/silica-dock
