#!/bin/bash

# Clone the repository
git clone https://github.com/hungify/lazyssh.git
cd lazyssh

# Build the project
cargo build --release

# Create the installation directory
INSTALL_DIR="/usr/local/bin"
sudo mkdir -p $INSTALL_DIR

# Copy the binary to the installation directory
sudo cp target/release/lazyssh $INSTALL_DIR

# Make the binary executable
sudo chmod +x $INSTALL_DIR/lazyssh

echo "lazyssh has been installed to $INSTALL_DIR"
