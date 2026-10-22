kind: cmake
description: |
  silica-qt-test: a Qt Widgets window, for the real-app checks of
  tools/test-vm --desktop. Test only.

build-depends:
- freedesktop-sdk.bst:public-stacks/buildsystem-cmake.bst
- freedesktop-sdk.bst:components/mesa-headers.bst

depends:
- silica/test-apps/qtbase.bst

sources:
- kind: local
  path: files/test-apps/qt-test
