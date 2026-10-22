kind: autotools
description: Speech Dispatcher, common speech synthesis interface (GPL-2.0+).

build-depends:
- freedesktop-sdk.bst:public-stacks/buildsystem-autotools.bst

depends:
- freedesktop-sdk.bst:public-stacks/runtime-minimal.bst
- freedesktop-sdk.bst:components/glib.bst
- freedesktop-sdk.bst:components/sndfile.bst
- freedesktop-sdk.bst:components/libpulse.bst
- freedesktop-sdk.bst:components/systemd.bst
- silica/components/dotconf.bst
- silica/components/espeak-ng.bst

variables:
  # Only the espeak-ng module is built; the other engines are not in the base
  # system. Audio goes through the PipeWire PulseAudio client.
  conf-local: >-
    --with-espeak-ng
    --without-espeak
    --without-flite
    --without-ibmtts
    --without-voxin
    --without-ivona
    --without-pico
    --without-baratinoo
    --without-kali
    --without-libao
    --without-alsa
    --without-oss
    --without-nas
    --with-pulse
    --disable-python
    --disable-doc
    --with-systemdsystemunitdir=%{indep-libdir}/systemd/system
    --with-systemduserunitdir=%{indep-libdir}/systemd/user

sources:
- kind: tar
  url: github:brailcom/speechd/releases/download/0.12.1/speech-dispatcher-0.12.1.tar.gz
  ref: b14a5238d287d2dcce4dd42bbd66ca65fa228e7e683708267f7b34036f7ba4b4
