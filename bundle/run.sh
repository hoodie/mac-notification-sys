#!/bin/bash

cargo build --example simple
rm -rf simple.app
mkdir -p simple.app/Contents/MacOS
mkdir -p simple.app/Contents/Resources
cp target/debug/examples/simple simple.app/Contents/MacOS/simple-bin
cp bundle/Info.plist simple.app/Contents/
cp bundle/rust-logo.icns simple.app/Contents/Resources
codesign --force --sign "$CERTIFICATE" -o runtime --entitlements ./bundle/simple.app.xcent --timestamp=none --generate-entitlement-der ./simple.app
open simple.app
