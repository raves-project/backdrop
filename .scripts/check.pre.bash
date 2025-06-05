#!/bin/bash
set -e

# ensure `cargo` is present!
if ! command -v cargo >/dev/null 2>&1; then
    echo "Failed to find \`cargo\` installation!"
    exit 255
fi

# ...and that curl is around
if ! command -v curl >/dev/null 2>&1; then
    echo "Failed to find \`curl\`!"
    echo "Since you've installed \`pixi\`, you probably have it installed. So, please ensure it's in your path."
    exit 255
fi

# great..! now, grab `cargo-binstall` if it isn't installed already!
if ! command -v cargo-binstall >/dev/null 2>&1; then
    echo "Installing \`cargo-binstall\`..."
    curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
    echo "Completed \`cargo-binstall\` installation!"
fi

# if not present, install each tool...
#
# ...cargo-rdme
if ! command -v cargo-rdme >/dev/null 2>&1; then
    echo "Installing \`cargo-rdme\`..."
    cargo binstall cargo-rdme -y --quiet
    echo "Completed \`cargo-rdme\` installation!"
fi
#
# ...cargo-deny
if ! command -v cargo-deny >/dev/null 2>&1; then
    echo "Installing \`cargo-deny\`..."
    cargo binstall cargo-deny -y --quiet
    echo "Completed \`cargo-deny\` installation!"
fi
#
# ...cargo-machete
if ! command -v cargo-machete >/dev/null 2>&1; then
    echo "Installing \`cargo-machete\`..."
    cargo binstall cargo-machete -y --quiet
    echo "Completed \`cargo-machete\` installation!"
fi
#
# ...cargo-nextest
if ! command -v cargo-nextest >/dev/null 2>&1; then
    echo "Installing \`cargo-nextest\`..."
    cargo binstall cargo-nextest -y --quiet
    echo "Completed \`cargo-nextest\` installation!"
fi
