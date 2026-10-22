kind: meson
description: |
  GTK 4 with its demos, for the real-app checks of tools/test-vm --desktop.
  Test only, never part of the system image; freedesktop-sdk ships GTK 3 alone.

build-depends:
- freedesktop-sdk.bst:public-stacks/buildsystem-meson.bst
- freedesktop-sdk.bst:components/wayland-protocols.bst
- freedesktop-sdk.bst:components/mesa-headers.bst

depends:
- freedesktop-sdk.bst:public-stacks/runtime-minimal.bst
- freedesktop-sdk.bst:components/glib.bst
- freedesktop-sdk.bst:components/cairo.bst
- freedesktop-sdk.bst:components/pango.bst
- freedesktop-sdk.bst:components/harfbuzz.bst
- freedesktop-sdk.bst:components/fribidi.bst
- freedesktop-sdk.bst:components/fontconfig.bst
- freedesktop-sdk.bst:components/gdk-pixbuf.bst
- freedesktop-sdk.bst:components/graphene.bst
- freedesktop-sdk.bst:components/libepoxy.bst
- freedesktop-sdk.bst:components/libpng.bst
- freedesktop-sdk.bst:components/libtiff.bst
- freedesktop-sdk.bst:components/jpeg.bst
- freedesktop-sdk.bst:components/libdrm.bst
- freedesktop-sdk.bst:components/libxkbcommon.bst
- freedesktop-sdk.bst:components/wayland.bst
- freedesktop-sdk.bst:components/iso-codes.bst
- freedesktop-sdk.bst:components/hicolor-icon-theme.bst
- freedesktop-sdk.bst:components/libglvnd.bst

variables:
  meson-local: >-
    -Dx11-backend=false
    -Dwayland-backend=true
    -Dbroadway-backend=false
    -Dmedia-gstreamer=disabled
    -Dprint-cpdb=disabled
    -Dprint-cups=disabled
    -Dvulkan=disabled
    -Dcloudproviders=disabled
    -Dsysprof=disabled
    -Dtracker=disabled
    -Dcolord=disabled
    -Daccesskit=disabled
    -Dintrospection=disabled
    -Ddocumentation=false
    -Dscreenshots=false
    -Dman-pages=false
    -Dbuild-demos=true
    -Dbuild-testsuite=false
    -Dbuild-examples=false
    -Dbuild-tests=false

sources:
- kind: tar
  url: gnome_downloads:gtk/4.22/gtk-4.22.5.tar.xz
  ref: 7fd725deb2cb3f8dc218ad862c5056ff8548f49d3b0e4081796e444c22d19686
