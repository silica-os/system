kind: autotools
description: Portable C audio library used by espeak-ng.

build-depends:
- freedesktop-sdk.bst:public-stacks/buildsystem-autotools.bst

depends:
- freedesktop-sdk.bst:public-stacks/runtime-minimal.bst
- freedesktop-sdk.bst:components/alsa-lib.bst
- freedesktop-sdk.bst:components/libpulse.bst

variables:
  autogen: NOCONFIGURE=1 ./autogen.sh

public:
  bst:
    split-rules:
      devel:
        (>):
        - '%{libdir}/libpcaudio.so'

sources:
- kind: tar
  url: github:espeak-ng/pcaudiolib/archive/refs/tags/1.3.tar.gz
  ref: 86edb75048eec7fcd8d74f9568f051d50b4781982215095b96ba9c2c9c7c159b
