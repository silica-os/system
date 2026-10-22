kind: autotools
description: eSpeak NG text-to-speech synthesizer.

build-depends:
- freedesktop-sdk.bst:public-stacks/buildsystem-autotools.bst

depends:
- silica/components/pcaudiolib.bst
- freedesktop-sdk.bst:public-stacks/runtime-minimal.bst

variables:
  autogen: NOCONFIGURE=1 ./autogen.sh
  conf-local: >-
    --with-pcaudiolib
    --without-sonic
    --disable-rpath

environment:
  # The data files are compiled by the freshly built, not yet installed binary;
  # libtool's wrapper breaks when it changes directory.
  LD_LIBRARY_PATH: '%{build-root}/src/.libs'

public:
  bst:
    split-rules:
      devel:
        (>):
        - '%{libdir}/libespeak-ng.so'

sources:
- kind: tar
  url: github:espeak-ng/espeak-ng/archive/refs/tags/1.52.0.tar.gz
  ref: bb4338102ff3b49a81423da8a1a158b420124b055b60fa76cfb4b18677130a23
