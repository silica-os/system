kind: cmake
description: |
  Qt 6 base (Core, Gui, Widgets and the Wayland platform), for the real-app
  checks of tools/test-vm --desktop. Test only, never part of the system image.

build-depends:
- freedesktop-sdk.bst:public-stacks/buildsystem-cmake.bst
- freedesktop-sdk.bst:components/perl.bst
- freedesktop-sdk.bst:components/python3.bst
- freedesktop-sdk.bst:components/mesa-headers.bst
- freedesktop-sdk.bst:components/wayland-protocols.bst

depends:
- freedesktop-sdk.bst:public-stacks/runtime-minimal.bst
- freedesktop-sdk.bst:components/zlib.bst
- freedesktop-sdk.bst:components/zstd.bst
- freedesktop-sdk.bst:components/glib.bst
- freedesktop-sdk.bst:components/icu.bst
- freedesktop-sdk.bst:components/libdbus.bst
- freedesktop-sdk.bst:components/openssl.bst
- freedesktop-sdk.bst:components/freetype.bst
- freedesktop-sdk.bst:components/fontconfig.bst
- freedesktop-sdk.bst:components/harfbuzz.bst
- freedesktop-sdk.bst:components/libpng.bst
- freedesktop-sdk.bst:components/jpeg.bst
- freedesktop-sdk.bst:components/libdrm.bst
- freedesktop-sdk.bst:components/libglvnd.bst
- freedesktop-sdk.bst:components/libxkbcommon.bst
- freedesktop-sdk.bst:components/wayland.bst

variables:
  # Test only: no debug split (dwz trips over Qt's hard-linked tools).
  strip-binaries: ''
  qt-libdir: '%{lib}/qt6'
  cmake-local: >-
    -DQT_BUILD_EXAMPLES=OFF
    -DQT_BUILD_TESTS=OFF
    -DQT_BUILD_BENCHMARKS=OFF
    -DFEATURE_xcb=OFF
    -DFEATURE_xlib=OFF
    -DFEATURE_sql=OFF
    -DFEATURE_printsupport=OFF
    -DINSTALL_LIBDIR=%{lib}
    -DINSTALL_ARCHDATADIR=%{qt-libdir}
    -DINSTALL_PLUGINSDIR=%{qt-libdir}/plugins
    -DINSTALL_LIBEXECDIR=%{qt-libdir}/libexec
    -DINSTALL_MKSPECSDIR=%{qt-libdir}/mkspecs
    -DINSTALL_QMLDIR=%{qt-libdir}/qml
    -DINSTALL_INCLUDEDIR=include/qt6
    -DINSTALL_DATADIR=share/qt6

config:
  install-commands:
    (>):
    - test -e '%{install-root}%{prefix}/%{qt-libdir}/plugins/platforms/libqwayland.so'

sources:
- kind: tar
  url: qt:6.11/6.11.2/submodules/qtbase-everywhere-src-6.11.2.tar.xz
  ref: 5b2e00eccaf5a4d8c14134ffa0ea8dfd0a35ae1ffc7f8d87fa4305a1ed23cf22
