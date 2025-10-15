# WASM Memory Estimator - Raspberry Pi 5 Deployment

## Overview
This project analyzes WebAssembly (WASM) memory usage and can be cross-compiled for Raspberry Pi 5.

## Cross-Compilation for Raspberry Pi 5

### Prerequisites

#### On macOS:
```bash
# Install Homebrew if not already installed
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install cross-compilation tools
brew install aarch64-unknown-linux-gnu
```

#### On Linux (Ubuntu/Debian):
```bash
sudo apt-get update
sudo apt-get install -y gcc-aarch64-linux-gnu
```

### Setup and Build

1. **Run the setup script:**
   ```bash
   ./cross-compile.sh
   ```

2. **Build for Raspberry Pi 5:**
   ```bash
   cargo build --release --target aarch64-unknown-linux-gnu
   ```

3. **Copy to Raspberry Pi 5:**
   ```bash
   scp target/aarch64-unknown-linux-gnu/release/memory-estimator pi@<rpi-ip>:/home/pi/
   ```

## Memory Measurement Notes

### macOS Memory Measurement Issues
The memory measurement on macOS may be inaccurate due to:
- Different memory reporting mechanisms
- System-level memory management
- Process isolation differences

### Raspberry Pi 5 Advantages
- More accurate memory measurement using Linux `/proc/self/statm`
- Better process isolation
- Native ARM64 performance

## Usage on Raspberry Pi 5

1. **SSH into your Raspberry Pi 5:**
   ```bash
   ssh pi@<rpi-ip>
   ```

2. **Make the binary executable:**
   ```bash
   chmod +x memory-estimator
   ```

3. **Run the memory estimator:**
   ```bash
   ./memory-estimator
   ```

## Dependencies on Raspberry Pi 5

The following libraries may need to be installed on the Raspberry Pi 5:

```bash
sudo apt-get update
sudo apt-get install -y libc6-dev libssl-dev
```

## Troubleshooting

### Common Issues:

1. **Missing system libraries:**
   - Install required system libraries as shown above

2. **Permission denied:**
   - Ensure the binary is executable: `chmod +x memory-estimator`

3. **WASM files not found:**
   - Copy the `wasm-modules/` directory to the Raspberry Pi 5
   - Ensure the binary is run from the correct directory

## Performance Notes

- Raspberry Pi 5 provides better memory measurement accuracy
- ARM64 architecture offers native performance
- Linux provides more detailed process memory information
