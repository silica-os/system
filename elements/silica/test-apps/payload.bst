kind: manual
description: |
  The test apps (GTK 3 and GTK 4 demos, silica-qt-test) and the libraries they
  need that the system image lacks: everything they pull in, minus what the
  image already has. Payload of the compositor-apps extension.

build-depends:
- freedesktop-sdk.bst:public-stacks/buildsystem-make.bst
- freedesktop-sdk.bst:components/glib.bst
- filename: silica/system/filesystem.bst
  config:
    location: /base
- filename: freedesktop-sdk.bst:components/gtk3.bst
  config:
    location: /apps
- filename: silica/test-apps/gtk4.bst
  config:
    location: /apps
# gdk-pixbuf loads images through glycin's loader processes, sandboxed with bwrap.
- filename: freedesktop-sdk.bst:components/glycin.bst
  config:
    location: /apps
- filename: freedesktop-sdk.bst:components/bubblewrap.bst
  config:
    location: /apps
- filename: silica/test-apps/qt-test.bst
  config:
    location: /apps

config:
  install-commands:
  - |
    set -eu
    cd /apps
    find usr ! -type d | while read -r f; do
      case "$f" in
        usr/include/*|*.a|*.la|*/pkgconfig/*|usr/%{lib}/cmake/*|usr/%{lib}/qt6/mkspecs/*) continue ;;
        usr/lib/debug/*|usr/share/man/*|usr/share/doc/*|usr/share/info/*|usr/share/aclocal/*) continue ;;
        usr/share/gtk-doc/*|usr/share/gir-1.0/*|usr/share/vala/*|usr/share/locale/*) continue ;;
      esac
      if [ -e "/base/$f" ] || [ -L "/base/$f" ]; then continue; fi
      mkdir -p "%{install-root}/${f%/*}"
      cp -a "$f" "%{install-root}/$f"
    done
  - |
    set -eu
    # The extension's schema cache shadows the image's, so it holds both.
    mkdir -p %{build-root}/schemas '%{install-root}%{datadir}/glib-2.0/schemas'
    cp /base%{datadir}/glib-2.0/schemas/*.xml %{build-root}/schemas/ 2>/dev/null || true
    cp /apps%{datadir}/glib-2.0/schemas/*.xml %{build-root}/schemas/
    glib-compile-schemas --targetdir='%{install-root}%{datadir}/glib-2.0/schemas' %{build-root}/schemas
  - |
    set -eu
    for app in gtk3-demo gtk4-demo; do test -x "%{install-root}%{bindir}/$app"; done
    test -x '%{install-root}%{prefix}/libexec/silica/tests/silica-qt-test'
