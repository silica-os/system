kind: autotools
description: dotconf, configuration file parser library (LGPL-2.1).

build-depends:
- freedesktop-sdk.bst:public-stacks/buildsystem-autotools.bst

depends:
- freedesktop-sdk.bst:public-stacks/runtime-minimal.bst

variables:
  autogen: autoreconf -fi

sources:
- kind: tar
  url: github:williamh/dotconf/archive/refs/tags/v1.4.1.tar.gz
  ref: 5922c46cacf99b2ecc4853d28a2bda4a489292e73276e604bd9cba29dfca892d
