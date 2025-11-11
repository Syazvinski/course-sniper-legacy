# course-sniper
A CLI tool for precision course registration.

## Usage
Requires chrome or chromium to be installed on your system.

Run with `./course-sniper [OPTIONS]` or you can add it to your PATH and run from anywhere `course-sniper [OPTIONS]`. Use `--help` to see all options.

Based on your operating system you may need to give it executable permissions `chmod +x course-sniper` or you can build from source.

Currently supported schools:
- Emory University

## Features
1. **Browser**: Operates a chromium instance through CDP, with the ability for headless execution as well.
2. **Login**: Used provided credentials to login and waits for DUO push if needed.
3. **Shopping Cart**: Automatically handles multiple shopping carts and gives full printout of classes in cart. Do not adjust your shopping cart from outside the tool while it is in use.
4. **Course Selection**: Can select any number of courses in the cart and then `course-sniper` will only target those specific courses.
5. **Actions**: For the selected courses can choose to validate or enroll.
    - Validate 
        - Immediately validates selected courses
        - Gives results
        - Exits
    - Enroll 
        - Prompts for an enrollment time
        - Waits for the enrollment time
        - Perfect reload
        - Registering for selected courses in a fraction of a second
        - Gives results
        - Exits
6. **Results**: Displays a full printout of validation/enrollment results.
7. **Coming Soon**:
    - Multiple concurrent snipers
    - Choice between multiple schools
    - Course fallbacks

## Installation
Download the latest release or build from source.

## Building From Source
Prerequisites:
- Rust toolchain which can be installed from [rustup.rs](https://rustup.rs)
- Cargo (comes with Rust)
- A linker (typically comes with a C compiler)

Build steps:
1. Clone the repo 
```bash
git clone https://github.com/ShaneBerhoff/course-sniper
cd course-sniper
```
2. Build for release `cargo build --release`
3. The compiled binary will be located at `./target/release/course-sniper`

## Contributing / Bugs
If you find a bug, report it in issues. If you want a feature, request it in issues. Feel free to patch the bug or add the feature yourself and submit a PR, if everything looks good I will merge it in.

## Disclaimer
This tool is intended for educational purposes. Be aware of your University's policies on automated systems for course registration. Use at your own risk.
