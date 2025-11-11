# Linux Installation Guide

## Verify Checksum
```bash
sha256sum -c checksum.sha256
```

## Make Executable and Run
1. Open terminal in the directory containing the files
2. Make the file executable:
   ```bash
   chmod +x BINARY_NAME
   ```
3. Run the program:
   ```bash
   ./BINARY_NAME
   ```

## Optional: Move to System Path
```bash
sudo mv BINARY_NAME /usr/local/bin/
```