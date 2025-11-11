# Windows Installation Guide

## Verify Checksum
1. Open Command Prompt as Administrator
2. Navigate to the directory containing the files
3. Run:
   ```
   certutil -hashfile BINARY_NAME.exe SHA256
   ```
4. Compare the output with the content of checksum.sha256

## Run the Program
1. Double click the .exe file
- Or from Command Prompt: `.\BINARY_NAME.exe`

Note: If Windows SmartScreen shows a warning, click "More Info" and then "Run Anyway".