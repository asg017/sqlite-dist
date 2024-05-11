#!/bin/bash

platforms=(
  "macos-x86_64   x86_64-macos    dylib"
  "macos-aarch64  aarch64-macos   dylib"
  "linux-x86_64   x86_64-linux    so"
  "linux-aarch64  aarch64-linux   so"
  "windows-x86_64 x86_64-windows  dll"
)

for platform in "${platforms[@]}"; do
  read -r DIRECTORY_NAME ZIG_TARGET LOADABLE_SUFFIX <<< "$platform"

  mkdir -p dist/$DIRECTORY_NAME

  make \
    TARGET=$ZIG_TARGET \
    TARGET_LOADABLE=dist/$DIRECTORY_NAME/sample0.$LOADABLE_SUFFIX \
    TARGET_STATIC=dist/$DIRECTORY_NAME/libsqlite_sample0.a \
    TARGET_H=dist/$DIRECTORY_NAME/sqlite_sample.h \
    loadable static h
done

exit


mkdir -p dist/macos-x86_64
mkdir -p dist/macos-aarch64
mkdir -p dist/linux-x86_64
mkdir -p dist/linux-aarch64
mkdir -p dist/windows-x86_64
mkdir -p dist/wasm32-emscripten

make TARGET=x86_64-macos   TARGET_LOADABLE=dist/macos-x86_64/sample0.dylib  TARGET_STATIC=dist/macos-x86_64/libsqlite_sample0.a   loadable static
make TARGET=aarch64-macos  TARGET_LOADABLE=dist/macos-aarch64/sample0.dylib TARGET_STATIC=dist/macos-aarch64/libsqlite_sample0.a  loadable static
make TARGET=x86_64-linux   TARGET_LOADABLE=dist/linux-x86_64/sample0.so     TARGET_STATIC=dist/linux-x86_64/libsqlite_sample0.a   loadable static
make TARGET=aarch64-linux  TARGET_LOADABLE=dist/linux-aarch64/sample0.so    TARGET_STATIC=dist/linux-aarch64/libsqlite_sample0.a  loadable static
make TARGET=x86_64-windows TARGET_LOADABLE=dist/windows-x86_64/sample0.dll  TARGET_STATIC=dist/windows-x86_64/libsqlite_sample0.a loadable static
touch dist/wasm32-emscripten/sqlite3.mjs dist/wasm32-emscripten/sqlite3.wasm
