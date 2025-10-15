#!/bin/bash

# Deploy script for Raspberry Pi 5
# This copies the source code to Raspberry Pi 5 and builds it there

echo "=== Raspberry Pi 5 Deployment Script ==="
echo ""

# Check if RPI_IP environment variable is set
if [ -z "$RPI_IP" ]; then
    echo "Please set RPI_IP environment variable:"
    echo "export RPI_IP=192.168.1.100  # Replace with your Pi's IP"
    echo ""
    echo "Then run: ./deploy-to-rpi5.sh"
    exit 1
fi

echo "Deploying to Raspberry Pi 5 at: $RPI_IP"
echo ""

# Create a temporary directory for the source
TEMP_DIR="/tmp/memory-estimator-$(date +%s)"
mkdir -p "$TEMP_DIR"

echo "📦 Copying source code to temporary directory..."
cp -r . "$TEMP_DIR/"

# Remove unnecessary files to reduce transfer size
cd "$TEMP_DIR"
rm -rf target/
rm -rf .git/
rm -f *.log

echo "🚀 Copying to Raspberry Pi 5..."
scp -r . pi@$RPI_IP:/home/pi/memory-estimator/

echo "🔧 Building on Raspberry Pi 5..."
ssh pi@$RPI_IP << 'EOF'
cd /home/pi/memory-estimator
echo "Installing Rust (if not already installed)..."
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source ~/.cargo/env

echo "Building memory-estimator..."
cargo build --release

echo "Build complete! Binary location:"
ls -lh target/release/memory-estimator

echo "Testing the binary..."
./target/release/memory-estimator
EOF

echo ""
echo "✅ Deployment complete!"
echo "Binary is now available at: pi@$RPI_IP:/home/pi/memory-estimator/target/release/memory-estimator"

# Cleanup
rm -rf "$TEMP_DIR"

