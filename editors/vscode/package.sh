#!/bin/bash

# Install dependencies
echo "Installing dependencies..."
npm install

# Install vsce if not already installed
if ! command -v vsce &> /dev/null; then
    echo "Installing vsce..."
    npm install -g vsce
fi

# Compile TypeScript
echo "Compiling TypeScript..."
npm run compile

# Package the extension
echo "Packaging extension..."
vsce package

echo "Extension packaged successfully!"
echo "You can install the .vsix file with:"
echo "  code --install-extension lsp-bridge-0.1.0.vsix"