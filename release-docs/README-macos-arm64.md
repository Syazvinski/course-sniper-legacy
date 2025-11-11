# macOS (Apple Silicon) Installation Guide

## Verify Checksum
```bash
shasum -a 256 -c checksum.sha256
```

## Make Executable and Run
1. Open Terminal in the directory containing the files
2. Make the file executable:
   ```bash
   chmod +x BINARY_NAME
   ```
3. Run the program:
   ```bash
   ./BINARY_NAME
   ```

Note: If you see a security warning, go to System Preferences > Security & Privacy and click "Open Anyway".

## Optional: Move to System Path
```bash
sudo mv BINARY_NAME /usr/local/bin/
```