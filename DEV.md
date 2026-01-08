# 🛠️ Developer Documentation

This document contains technical information for developers who want to build, modify, or contribute to the PAAP project (firmware and configurator).

---

## 📁 Project Structure

```
.
├── firmware/                          # Arduino firmware
│   └── firmware.ino                  # Main firmware code
├── app/                               # GUI configurator
│   ├── Cargo.toml                    # Rust project configuration
│   └── src/
│       ├── main.rs                   # Application entry point and main struct
│       ├── config.rs                 # Configuration and theme management
│       ├── connection.rs             # Connection state management
│       ├── serial.rs                 # Serial port communication logic
│       ├── ui.rs                     # UI components and rendering
│       └── icon.rs                   # Application icon generation
├── README.md                         # User-facing documentation
└── DEV.md                            # This file - developer documentation
```

---

# 🔌 Firmware Development

## Technical Specifications

- **Board**: Arduino Pro Micro (ATmega32U4) or any AVR board with native USB HID.
- **Debounce**: Bounce2 library at 5ms.
- **HID**: Arduino Keyboard library v1.0.6.
- **Pin**: D7 with INPUT_PULLUP (pressed = LOW).
- **Default key**: 'l' (keycode 108), configurable via serial protocol or GUI app.
- **Storage**: EEPROM for persistent keycode storage.

## Architecture Overview

The firmware uses `Bounce2::Button` to detect `pressed()`/`released()` transitions, calling `Keyboard.press()` and `Keyboard.release()` with the configured key code.

The keycode is stored in EEPROM and can be configured via:
- Serial protocol commands (see [Serial Protocol](#serial-protocol) section).
- GUI configurator app.

### Hardware Wiring

- Connect a momentary foot switch between D7 and GND.
- Internal pull-up is enabled, so the circuit is: D7 — switch — GND.
- Pressing the switch pulls the pin LOW ("pressed").

### Runtime Behavior

- On press: logs (if DEBUG enabled) and calls `Keyboard.press(KEY_CODE)`.
- On release: logs (if DEBUG enabled) and calls `Keyboard.release(KEY_CODE)`.

## Serial Protocol

The firmware supports a simple ASCII-based serial protocol at 115200 baud for configuration.

### Protocol Commands

| Command | Description | Response | Example |
|---------|-------------|----------|---------|
| `HEL\n` | Initiate connection | `LO\n` | Connection handshake |
| `KEY\n` | Request current keycode | `KEY<key>\n` | `KEYl\n` |
| `KEY<key>\n` | Set new keycode | `ACK\n` | `KEYa\n` → `ACK\n` |
| `VER\n` | Request firmware version | `VER<version>\n` | `VER0.0.1\n` |

### Example Protocol Session

```
Host: HEL\n
Device: LO\n
Host: VER\n
Device: VER0.0.1\n
Host: KEY\n
Device: KEYl\n
Host: KEYk\n
Device: ACK\n
```

## Firmware Technology Stack

- **Platform**: Arduino (ATmega32U4).
- **Language**: C++ (Arduino framework).
- **Libraries**: 
  - Keyboard v1.0.6 (bundled with Arduino).
  - Bounce2 v2.71.0.
  - EEPROM (bundled with Arduino).

## Firmware Dependencies

- `Keyboard` v1.0.6 (bundled with Arduino for 32U4 boards).
- `Bounce2` v2.71.0 (install via Library Manager or with `arduino-cli lib install "Bounce2@2.71.0"`; upstream: https://github.com/thomasfredericks/Bounce2).

## Prerequisites

### Arduino IDE
Install from [https://www.arduino.cc/en/software](https://www.arduino.cc/en/software)

### Arduino CLI (Alternative)
```bash
# Install Arduino CLI
curl -fsSL https://raw.githubusercontent.com/arduino/arduino-cli/master/install.sh | sh

# Add to PATH or use absolute path
```

### Required Libraries
```bash
# Via Arduino CLI
arduino-cli lib install "Bounce2@2.71.0"

# Via Arduino IDE
# Library Manager → Search "Bounce2" → Install version 2.71.0
```

## Building Firmware

### Using Arduino IDE

1. Open `firmware/firmware.ino`
2. Select Board: **Tools → Board → Arduino AVR Boards → Arduino Leonardo** (or Arduino Micro)
3. Select Port: **Tools → Port → [Your Device Port]**
4. Verify/Compile: **Sketch → Verify/Compile** (Ctrl+R)
5. Upload: **Sketch → Upload** (Ctrl+U)

### Using Arduino CLI

```bash
# Update core index
arduino-cli core update-index

# Install Arduino AVR core
arduino-cli core install arduino:avr

# Update library index
arduino-cli lib update-index

# Install dependencies
arduino-cli lib install "Bounce2@2.71.0"

# Compile firmware
arduino-cli compile --fqbn arduino:avr:micro firmware/

# Upload (replace with your device port)
arduino-cli upload --fqbn arduino:avr:micro -p /dev/ttyACM0 firmware/
```

### Finding Your Device Port

**Linux:**
```bash
ls /dev/ttyACM* /dev/ttyUSB*
# or
arduino-cli board list
```

**Windows:**
```
Device Manager → Ports (COM & LPT)
# or use Arduino IDE's port selection
```

**macOS:**
```bash
ls /dev/cu.usbmodem*
```

## Firmware Configuration

### Edit Firmware Source

Edit [firmware/firmware.ino](firmware/firmware.ino) for customization:

- **Default keycode**: 'l' (108), stored in EEPROM, can be changed via serial protocol or configurator app.
- **Optional DEBUG mode**: Uncomment `#define DEBUG` to print events on Serial at 115200 baud.

### Debug Mode
Uncomment `#define DEBUG` in the firmware to enable serial debugging:
```cpp
#define DEBUG
```

This will print diagnostic messages to Serial Monitor at 115200 baud.

### Default Key Code
Change `DEFAULT_KEY_CODE` constant:
```cpp
const int DEFAULT_KEY_CODE = 108;  // 'l' key code
```

See [ASCII table](http://www.asciitable.com/) for key codes.

### Pin Configuration
Default pedal pin is D7. To change:
```cpp
const int PIN_PEDAL = 7;  // Change to desired pin
```

## Firmware Serial Protocol

The firmware implements an ASCII-based protocol at 115200 baud:

| Command | Description | Response |
|---------|-------------|----------|
| `HEL\n` | Connection handshake | `LO\n` |
| `KEY\n` | Get current keycode | `KEY<char>\n` |
| `KEY<char>\n` | Set keycode | `ACK\n` |
| `VER\n` | Get firmware version | `VER<version>\n` |

### Testing Serial Protocol

Using Arduino Serial Monitor or any serial terminal at 115200 baud:
```
> HEL
< LO
> VER
< VER0.0.1
> KEY
< KEYl
> KEYk
< ACK
```

## Troubleshooting Firmware

### Upload Issues
- **Port not found**: Check USB cable, try different USB port.
- **Permission denied** (Linux): Add user to dialout group:
  ```bash
  sudo usermod -a -G dialout $USER
  # Log out and back in
  ```
- **Device not recognized**: Press reset button twice quickly to enter bootloader mode.

## Advanced Firmware Options

### Customize USB VID/PID (Optional)

To set a custom USB Vendor ID and Product ID, edit your board's `boards.txt` file (located in Arduino install folder → `hardware/arduino/avr/boards.txt`).

For Arduino Pro Micro, update:
- `micro.build.vid=` to `0xF144` (OpenFire+1).
- `micro.build.pid=` to `0x1001` or `0x1002` (depending on pedal variant).
- `micro.usb.usb_product=` to `PAAP_P1` or `PAAP_P2`.

**Important**: Restart the Arduino IDE after making changes to boards.txt.

### Troubleshooting Firmware Issues

**Keyboard Not Working:**
- Verify board has native USB HID support (32U4, SAMD, RP2040).
- Check that Keyboard library is available for your board.
- Test with Serial Monitor first to verify basic functionality.

**Key Repeats Unexpectedly:**
- Increase debounce interval (default 5ms) in firmware:
  ```cpp
  pedal.interval(5);  // Change to higher value like 10 or 20
  ```

**Serial Communication Issues:**
- Ensure baud rate is 115200.
- Check line ending setting (should send `\n`).
- Verify no other program has the serial port open.

---

# 💻 Configurator Development

## Technology Stack

- **Language**: Rust (Edition 2021).
- **GUI Framework**: [egui](https://github.com/emilk/egui) via [eframe](https://docs.rs/eframe/).
- **Serial Communication**: [serialport-rs](https://gitlab.com/susurrus/serialport-rs).
- **Logging**: `log` + `env_logger`.

## Prerequisites

### Rust Toolchain

Install Rust from [https://rustup.rs/](https://rustup.rs/):
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### System Dependencies

#### Linux (Ubuntu/Debian)
```bash
sudo apt install libudev-dev
```

#### macOS
```bash
brew install libusb
```

#### Windows
No additional system dependencies required.

## Building Configurator

### Debug Build
For development with full debug symbols and no optimization:
```bash
cd app/
cargo build
```

Binary location: `app/target/debug/paap-configurator`

### Release Build
For optimized production binary:
```bash
cd app/
cargo build --release
```

Binary location: `app/target/release/paap-configurator`

The release profile is configured for minimal binary size:
- Size optimization (`opt-level = 'z'`).
- Link Time Optimization (LTO) enabled.
- Single codegen unit for better optimization.
- Stripped symbols.
- Abort on panic for smaller binary.

## Running Configurator

### Development Mode
```bash
cd app/
cargo run
```

### With Logging
Set the `RUST_LOG` environment variable:
```bash
cd app/
RUST_LOG=debug cargo run
RUST_LOG=info cargo run
```

### Running the Binary
```bash
./app/target/debug/paap-configurator      # Debug build
./app/target/release/paap-configurator    # Release build
```

## Configurator Development

### Code Architecture

The application is structured as a modular Rust application with clear separation of concerns:

#### **main.rs** - Application Entry Point
- **`SerialPortApp`**: Main application struct managing state and coordinating between modules.
  - `available_ports`: List of detected serial ports.
  - `connection_state`: Current connection status (from `connection` module).
  - `ui_state`: UI-specific state (from `ui` module).
  - `theme_mode`: Current theme selection.
- **`eframe::App` implementation**: Main update loop with action-based UI pattern.
- Application initialization and window configuration.

#### **config.rs** - Configuration Management
- **`ThemeMode` enum**: Dark, Light, System theme options.
- **`AppConfig` struct**: Persistent configuration storage.
- Configuration loading and saving using `confy` crate.

#### **connection.rs** - Connection State
- **`ConnectionState` enum**: Disconnected or Connected with device details.
  - Tracks: serial port handle, port name, current key, firmware version.
- Helper methods for querying connection status.

#### **serial.rs** - Serial Communication
- **`SerialManager`**: Handles all serial port operations.
  - Port discovery and display name formatting.
  - Device connection handshake protocol (HEL/LO).
  - Version retrieval (VER command).
  - Key reading and writing (KEY command).
  - Low-level serial response parsing and timeout handling.

#### **ui.rs** - User Interface
- **`UiState` struct**: UI-specific state (key input, error messages).
- **`UiAction` enum**: Event-driven UI actions (Connect, Disconnect, etc.).
- **Rendering functions**: Modular UI component rendering.
  - `render_top_bar`: Port selection and refresh controls.
  - `render_key_input`: Key configuration interface.
  - `render_bottom_panel`: Status display and theme selector.
  - `render_error_dialog`: Error modal window.
  - `apply_theme`: Theme application logic.

#### **icon.rs** - Application Icon
- Icon generation for window title bar.
- Creates a 32×32 keyboard icon programmatically.

### Communication Protocol

The `serial.rs` module implements the complete serial communication protocol:

#### Connection Handshake (in `SerialManager::connect`)
1. App sends: `HEL\n`.
2. Device responds: `LO\n`.
3. App sends: `VER\n`.
4. Device responds: `VER<VERSION>\n`.
5. App sends: `KEY\n`.
6. Device responds: `KEY<CHAR>\n` where `<CHAR>` is the current key.
7. Timeout: 2 seconds per command.

#### Key Configuration (in `SerialManager::save_key`)
1. App sends: `KEY<CHAR>\n` where `<CHAR>` is the new key.
2. Device responds: `ACK\n` on success.
3. Timeout: 2 seconds.

All protocol implementation is isolated in the `serial` module, with helper functions:
- `wait_for_simple_response`: Waits for exact string match (e.g., "LO", "ACK").
- `wait_for_prefixed_response`: Waits for response starting with prefix (e.g., "VER", "KEY").

### UI Layout

The UI is rendered by functions in the `ui` module:

- **Top-right** (`render_top_bar`): Port selection dropdown and refresh button.
- **Center** (`render_key_input`): Key configuration input and save button.
- **Bottom panel** (`render_bottom_panel`): Connection status, port count, and theme selector.
- **Modal** (`render_error_dialog`): Error messages displayed as centered dialogs.

### Action-Based UI Pattern

The application uses an action-based pattern to avoid borrow checker issues:
1. UI rendering functions return `Option<UiAction>` instead of taking callbacks.
2. Actions are collected during UI rendering.
3. After all UI is rendered, actions are processed in the main update loop.
4. This cleanly separates UI rendering (immutable borrows) from state mutation (mutable borrows).

## Dependencies

### Runtime Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `eframe` | 0.29 | GUI framework (includes egui) |
| `serialport` | 4.5 | Serial port communication |
| `log` | 0.4 | Logging facade |
| `env_logger` | 0.11 | Logger implementation |
| `confy` | 0.6 | Configuration file management |
| `serde` | 1.0 | Serialization/deserialization |

### Build Profiles

**Debug Profile**:
```toml
opt-level = 0
debug = true
strip = false
debug-assertions = true
overflow-checks = true
incremental = true
```

**Release Profile**:
```toml
opt-level = 'z'
lto = true
codegen-units = 1
strip = true
panic = 'abort'
```

## Debugging

### Enable Detailed Logging
```bash
cd app/
RUST_LOG=debug cargo run
```

### Check Serial Port Detection
The app logs all detected ports on startup with their details (VID, PID, manufacturer, product).

### Connection Issues
- Check device responds to `HEL\n` with `LO\n`.
- Verify timeout settings (currently 2 seconds).
- Ensure no other application has the port open.
- Test firmware with Serial Monitor first.

---

# 🚢 Release Process

## Firmware Release

1. Update `VERSION` constant in firmware:
   ```cpp
   const char VERSION[] = "0.0.2";
   ```
2. Test thoroughly on target hardware.
3. Compile and verify upload process.
4. Tag release in git.
5. Document any breaking changes.

## Configurator Release

1. Update version in `app/Cargo.toml`:
   ```toml
   version = "0.2.0"
   ```
2. Build release binary: `cd app/ && cargo build --release`.
3. Test the release binary thoroughly with actual hardware.
4. Strip binary (if not already done): `strip app/target/release/paap-configurator`.
5. Package for distribution.
6. Tag release in git.

### Binary Sizes (Typical)
- Debug: ~15-20 MB (with symbols).
- Release: ~3-5 MB (stripped, optimized).

---

# 🤝 Contributing

## Code Style

### Firmware (C++)
- Follow Arduino coding conventions.
- Use descriptive variable and function names.
- Add Google-style documentation comments for functions.
- Keep functions focused and modular.

### Configurator (Rust)
- Follow standard Rust formatting: `cargo fmt`.
- Check for common issues: `cargo clippy`.
- Ensure all warnings are addressed.
- Write idiomatic Rust code.
- Maintain module separation: keep UI, business logic, and serial communication separate.
- Use the action-based pattern for UI interactions to avoid borrow checker issues.
- Add comprehensive error handling with user-friendly error messages.

---

# 📚 References

## Firmware
- [Arduino Reference](https://www.arduino.cc/reference/en/)
- [Keyboard Library Documentation](https://www.arduino.cc/reference/en/language/functions/usb/keyboard/)
- [Bounce2 Documentation](https://github.com/thomasfredericks/Bounce2)
- [EEPROM Documentation](https://www.arduino.cc/en/Reference/EEPROM)

## Configurator
- [egui Documentation](https://docs.rs/egui/)
- [eframe Documentation](https://docs.rs/eframe/)
- [serialport-rs Documentation](https://docs.rs/serialport/)
- [Rust Book](https://doc.rust-lang.org/book/)

---

## 📄 License

See main README for license information.
