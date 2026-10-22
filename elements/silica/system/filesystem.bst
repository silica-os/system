kind: compose
description: The Silica base system, as files.

build-depends:
- silica/system/deps.bst

config:
  exclude:
  - debug
  - devel
  - doc
  - tests
  include-orphans: True
