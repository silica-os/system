kind: script
description: |
  Development extension: the compositor's binaries and silica-test-client
  over the ones in the system image, for tools/dev-sysext, which builds it
  from a workspace of the compositor checkout.

(@): elements/silica/extensions/sysext.yml

build-depends:
  (>):
  - filename: silica/desktop/compositor.bst
    config:
      location: /payload

variables:
  sysext-name: silica-compositor-dev
  sysext-paths: >-
    usr/bin/silica-compositor
    usr/bin/silica-cli
    usr/bin/silica-action
    usr/libexec/silica/tests
