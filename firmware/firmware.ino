#include <Bounce2.h>
#include <EEPROM.h>

#include "Keyboard.h"

const char VERSION[] = "1.0.0";

/* Change USB VID & PID:

    To change the USB Vendor ID (VID) and Product ID (PID),
    modify the following lines in the "boards.txt" file
    located in the Arduino hardware folder for your board.

    For example, for the Arduino Pro Micro, the path might be:
    <Arduino IDE installation folder>/hardware/arduino/avr/boards.txt

    Find the line starting with "micro.build.vid=" and change
    the VID to 0xF144 (OpenFire+1). Change the PID
    in the line starting with "micro.build.pid=" to 0x1001 or 0x1002
    depending on which pedal you are configuring.
    Change the product string in "micro.usb.usb_product=" to
    "PAAP_P1" or "PAAP_P2" accordingly.

    Note: Restart IDE after making changes to boards.txt.
*/

// #define DEBUG.

// EEPROM address for storing keycode.
const int EEPROM_KEYCODE_ADDR = 0;
// 'l' key code.
const int DEFAULT_KEY_CODE = 108;

int KEY_CODE = DEFAULT_KEY_CODE;

const int PIN_PEDAL = 7;
Bounce2::Button pedal = Bounce2::Button();

String serialBuffer = "";

/**
 * Checks if a keycode is valid.
 *
 * Validates that the keycode falls within the printable ASCII range (32-126).
 *
 * @param keycode The keycode to validate.
 * @return True if the keycode is valid, false otherwise.
 */
bool isValidKeyCode(int keycode) {
    return (keycode >= 32 && keycode <= 126);
}

/**
 * Loads the keycode from EEPROM.
 *
 * Reads the stored keycode from EEPROM and validates it. If the stored value
 * is invalid, sets KEY_CODE to DEFAULT_KEY_CODE and saves it to EEPROM.
 */
void loadKeyCode() {
    int storedKey = EEPROM.read(EEPROM_KEYCODE_ADDR);
    if (isValidKeyCode(storedKey)) {
        KEY_CODE = storedKey;
    } else {
        KEY_CODE = DEFAULT_KEY_CODE;
        saveKeyCode();
    }
}

/**
 * Saves the current keycode to EEPROM.
 *
 * Validates KEY_CODE before writing it to EEPROM. Only saves if the keycode
 * is within the valid printable ASCII range.
 */
void saveKeyCode() {
    if (isValidKeyCode(KEY_CODE)) {
        EEPROM.write(EEPROM_KEYCODE_ADDR, KEY_CODE);
    }
}

/**
 * Processes incoming serial commands.
 *
 * Handles the following commands:
 * - HEL: Responds with "LO\n"
 * - KEY: Responds with "KEY<keycode>\n"
 * - KEY<key>: Sets new keycode and responds with "ACK\n"
 * - VER: Responds with "VER<version>\n"
 *
 * @param cmd The command string to process (without newline).
 */
void processSerialCommand(String cmd) {
    cmd.trim();

    if (cmd == "HEL") {
        Serial.println("LO");
    } else if (cmd == "KEY") {
        Serial.print("KEY");
        Serial.println((char)KEY_CODE);
    } else if (cmd.startsWith("KEY") && cmd.length() == 4) {
        // Extract the key character (position 3).
        char newKey = cmd.charAt(3);
        KEY_CODE = (int)newKey;
        saveKeyCode();
        Serial.println("ACK");
    } else if (cmd == "VER") {
        Serial.print("VER");
        Serial.println(VERSION);
    }
}

/**
 * Handles incoming serial data.
 *
 * Reads characters from the serial port and buffers them until a newline
 * character is received. When a complete command is received, it processes
 * the command and sends "OK\n" response.
 */
void handleSerial() {
    while (Serial.available() > 0) {
        char c = Serial.read();
        if (c == '\n') {
            processSerialCommand(serialBuffer);
            serialBuffer = "";
        } else {
            serialBuffer += c;
        }
    }
}

/**
 * Arduino setup function.
 *
 * Initializes serial communication at 115200 baud, loads the keycode from
 * EEPROM, configures the pedal button with debouncing and internal pullup,
 * and initializes the USB Keyboard interface.
 */
void setup() {
    // Initialize serial communication.
    Serial.begin(115200);

    // Load keycode from EEPROM.
    loadKeyCode();

    // Setup the Bounce2 button with internal pullup.
    pedal.attach(PIN_PEDAL, INPUT_PULLUP);
    // Set debounce interval to 5ms.
    pedal.interval(5);
    // Because of INPUT_PULLUP, pressed state is LOW.
    pedal.setPressedState(LOW);

    Keyboard.begin();

#ifdef DEBUG
    Serial.println("PAAP started.");
    Serial.print("Using key code: ");
    Serial.print(KEY_CODE);
    Serial.print(" on pin ");
    Serial.println(PIN_PEDAL);
#endif
}

/**
 * Arduino main loop function.
 *
 * Continuously handles serial commands, updates the pedal button state,
 * prints debug information every second, and sends keyboard press/release
 * events when the pedal state changes.
 */
void loop() {
    // Handle serial commands.
    handleSerial();

    // Update the Bounce2 button state.
    pedal.update();

#ifdef DEBUG
    static uint32_t lastRun = 0L;
    if (lastRun + 1000L < millis()) {
        lastRun = millis();
        Serial.print("DBG: Keycode=");
        Serial.print(KEY_CODE);
        Serial.print(" PedalState=");
        Serial.println(pedal.read() == HIGH ? "RELEASED" : "PRESSED");
    }
#endif

    // Check if button was pressed.
    if (pedal.pressed()) {
#ifdef DEBUG
        Serial.println("Pedal pressed.");
#endif
        Keyboard.press(KEY_CODE);
    }

    // Check if button was released.
    if (pedal.released()) {
#ifdef DEBUG
        Serial.println("Pedal released.");
#endif
        Keyboard.release(KEY_CODE);
    }
}