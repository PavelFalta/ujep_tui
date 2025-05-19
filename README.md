# UJEP Timetable Client - Lightning Fast Access & Filter

![image](https://github.com/user-attachments/assets/c46b014d-7074-4d22-b937-5a8036267a7c)

## Overview 
- The tui is designed to fetch and display your timetable data from UJEP's API.
- Login using your stag creds and watch it provide a rice-friendly interface to browse/filter classes.
- Written in Rust.

You have two options to install the client:

### Option 1: Use Released Binaries
- **Version:** v1.2
- **Installation Guide:** Detailed instructions are available in the release description on my [GitHub Releases page](https://github.com/PavelFalta/ujep_tui/releases).

Simply download the pre-built binary for your operating system and follow the guide provided in the release description.

### Option 2: Build from Source 🔧
If you prefer to build the project yourself, follow these steps:

1. **Clone the Repository:**
   ```bash
   git clone https://github.com/PavelFalta/ujep_tui.git
   cd ujep_tui
   ```

2. **Install Rust:**
   Ensure you have the Rust toolchain installed. You can get it from [rustup.rs](https://rustup.rs/).

3. **Build the Project:**
   ```bash
   cargo build --release
   ```
   The compiled binary will be located in the `target/release/` directory.

4. **Run the Application:**
   ```bash
   ./target/release/timetable_visualizer
   ```

## Usage 🚀

Once installed, launch the application from your terminal. The interface allows you to:
- Log in using your UJEP credentials.
- Fetch and display your timetable data.
- Navigate through upcoming classes.
- View class details.
- Ignore classes you can't be arsed to attend.
- Filter/search classes.
- Toggle a very cool ASCII clock I implemented just because I can.

Also runs offline provided you already logged in at least once before.
