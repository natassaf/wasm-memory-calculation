How to run on raspberry pi 5

cd /tmp
wget https://github.com/microsoft/onnxruntime/releases/download/v1.22.0/onnxruntime-linux-aarch64-1.22.0.tgz
tar -xzf onnxruntime-linux-aarch64-1.22.0.tgz
sudo cp -r onnxruntime-linux-aarch64-1.22.0/lib/* /usr/local/lib/
sudo ldconfig
ldconfig -p | grep onnxruntime

echo 'export ORT_LIB_LOCATION=/tmp/onnxruntime-linux-aarch64-1.22.0/lib' >> ~/.bashrc
echo 'export ORT_STRATEGY=system' >> ~/.bashrc
echo 'export LD_LIBRARY_PATH=/tmp/onnxruntime-linux-aarch64-1.22.0/lib:$LD_LIBRARY_PATH' >> ~/.bashrc



# Update system
sudo apt update && sudo apt upgrade -y

# Install basic development tools
sudo apt install -y build-essential pkg-config libssl-dev git curl
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source ~/.cargo/env

# Install ONNX Runtime development package
sudo apt install libonnxruntime-dev

# Verify it's installed
dpkg -L libonnxruntime-dev | grep "\.so"
# Should show: /usr/lib/aarch64-linux-gnu/libonnxruntime.so

# Set the environment variable to point to the correct directory
export ORT_LIB_LOCATION=/usr/lib/aarch64-linux-gnu
export ORT_STRATEGY=system

# Also add it to the library path
export LD_LIBRARY_PATH=/usr/lib/aarch64-linux-gnu:$LD_LIBRARY_PATH

# Navigate to your project
cd /path/to/memory-estimator

# Build
cargo build --release

# Run
./target/release/memory-estimator