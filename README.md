# PAAP — Arcade Pedal to Keyboard

[![Build Firmware](https://github.com/tbreckle/paap_pedal_software/actions/workflows/build-firmware.yaml/badge.svg)](https://github.com/tbreckle/paap_pedal_software/actions/workflows/build-firmware.yaml) [![Build App](https://github.com/tbreckle/paap_pedal_software/actions/workflows/build.yaml/badge.svg)](https://github.com/tbreckle/paap_pedal_software/actions/workflows/build.yaml)

**PAAP** turns your arcade pedal into a keyboard key! Simply plug it in, press the pedal, and it types a key on your computer — perfect for arcade games, simulators, or any application where you want hands-free keyboard control.

## ✨ What You Get

- 🎮 **Plug & Play** - Works like a USB keyboard, no drivers needed
- ⌨️ **Customizable Key** - Choose any keyboard key you want
- 🖥️ **Easy Setup** - User-friendly app to configure your pedal
- 💾 **Remembers Settings** - Your key choice is saved even after unplugging
- 🪟 **Works Everywhere** - Compatible with Windows, Linux, and macOS

## 🎯 What You Need

- PAAP arcade pedal hardware ([available here](https://ko-fi.com/s/10c3e43f22) - created by JayBomb999)
- A computer with a USB port
- The PAAP Configurator app (included in this package)

## 🎮 Use Cases

PAAP is perfect for:
- Light gun arcade games (reload pedal)
- Racing simulators (clutch, brake, gear shift)
- Music applications (sustain pedal, drum trigger)
- Accessibility (hands-free keyboard input)
- Streaming (push-to-talk, scene switching)
- Any application where you want foot-controlled keyboard input

## 📝 Default Settings

Out of the box, your PAAP is configured with:
- **Default key:** 'l' (lowercase L)
- Your custom settings are saved and persist even after unplugging

## 🚀 Quick Start

### 1. Connect Your Pedal

1. Plug the PAAP device into a USB port on your computer
2. Your computer will recognize it as a keyboard (no installation needed!)
3. By default, pressing the pedal types the letter 'l'

### 2. Configure Your Key (Optional)

If you want to use a different key:

1. Download and run the **PAAP Configurator** app (see below)
2. Select your device from the dropdown menu
3. Press the key you want on your keyboard
4. Click "Save to device"
5. Done! Your pedal now uses your chosen key

## 🔧 PAAP Configurator App

The configurator app makes it easy to change which key your pedal presses.

### Installation

**Windows:**
1. Download `paap-configurator.exe` from the releases
2. Double-click to run (no installation required)

**Linux:**
1. Download `paap-configurator` from the releases
2. Make it executable: `chmod +x paap-configurator`
3. Run it: `./paap-configurator`

### How to Use

1. **Connect** - Plug in your PAAP pedal
2. **Open the app** - Launch PAAP Configurator
3. **Select device** - Choose your pedal from the dropdown in the top-right corner
4. **Pick your key** - Click in the "Key to emulate" field and press your desired key
5. **Save** - Click "Save to device"
6. **Test** - Open any text editor and press your pedal!

### App Tips

- 💡 The status bar shows if you're connected
- 🎨 Switch between Dark and Light themes in the bottom-right
- 🔄 Click "Refresh Ports" if your device doesn't appear
- 📱 Current key is displayed in the status bar when connected

## ❓ Troubleshooting

### My pedal isn't working

**Nothing happens when I press the pedal:**
- Check that the USB cable is fully plugged in
- Try a different USB port
- Make sure your pedal hardware is properly assembled

**Computer doesn't recognize the device:**
- Try unplugging and plugging it back in
- Use a different USB cable
- Restart your computer

### Configurator Issues

**My device doesn't appear in the list:**
- Make sure it's plugged in
- Click "Refresh Ports" button
- Try a different USB port
- **Linux only:** You might need to run this command once:
  ```bash
  sudo usermod -a -G dialout $USER
  ```
  Then log out and log back in

**Can't connect to device:**
- Close any other programs that might be using the device
- Unplug and replug the pedal
- Restart the configurator app

**Save fails:**
- Check that you're still connected (look at the status bar)
- Try disconnecting and reconnecting

### Still Having Problems?

- Make sure you're using the latest firmware and configurator
- Check the [DEV.md](DEV.md) file for more technical information
- Report issues on the project page

## 🛠️ For Developers & Advanced Users

Want to modify the firmware, build from source, or understand the technical details?

Check out [DEV.md](DEV.md) for:
- Firmware building and upload instructions
- Serial protocol documentation
- Source code information
- Contribution guidelines

## 📄 Credits & License

- Hardware design: JayBomb999 - [Available on Ko-fi](https://ko-fi.com/s/10c3e43f22)
- Firmware & software: This project

This project is provided as-is for configuring PAAP devices.

---

## ⚖️ Legal Disclaimer

**USE AT YOUR OWN RISK**

This software and firmware are provided "AS IS" without warranty of any kind, express or implied, including but not limited to the warranties of merchantability, fitness for a particular purpose, and non-infringement.

**Important Notes:**

- The developers and contributors of this project assume **no liability** for any damages, losses, or issues arising from the use, misuse, or inability to use this software or hardware
- This includes, but is not limited to: hardware damage, data loss, system malfunction, or any other direct or indirect consequences
- By using this software, you acknowledge that you do so at your own risk
- Users are responsible for ensuring compatibility with their systems and compliance with local regulations
- This project is an independent effort and is not affiliated with or endorsed by any hardware manufacturer unless explicitly stated

**For Hobbyist and Educational Use:**

This project is intended for personal, educational, and hobbyist use. Users modifying firmware or hardware configurations should have appropriate technical knowledge and accept full responsibility for their modifications.

If you're unsure about any aspect of installation or use, please consult the documentation or seek assistance from knowledgeable individuals before proceeding.
