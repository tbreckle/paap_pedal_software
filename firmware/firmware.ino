#include <Bounce2.h>
#include <EEPROM.h>
#include <Keyboard.h>
#include <PluggableUSB.h>

#include "version.h"

// #define DEBUG.

// EEPROM address for storing keycode.
const int EEPROM_KEYCODE_ADDR = 0;
// EEPROM address for storing the pedal number.
const int EEPROM_PEDAL_ADDR = 1;
// 'l' key code.
const int DEFAULT_KEY_CODE = 108;
// Pedal numbers range from 1 to 16.
const uint8_t MIN_PEDAL_NUMBER = 1;
const uint8_t MAX_PEDAL_NUMBER = 16;
const uint8_t DEFAULT_PEDAL_NUMBER = 1;

int KEY_CODE = DEFAULT_KEY_CODE;

const int PIN_PEDAL = 7;
Bounce2::Button pedal = Bounce2::Button();

String serialBuffer = "";

/**
 * Reads the pedal number from EEPROM.
 *
 * @return The stored pedal number, or DEFAULT_PEDAL_NUMBER if none is stored (erased EEPROM
 *         reads 0xFF).
 */
uint8_t readPedalNumber() {
    uint8_t stored = EEPROM.read(EEPROM_PEDAL_ADDR);
    if (stored >= MIN_PEDAL_NUMBER && stored <= MAX_PEDAL_NUMBER) {
        return stored;
    }
    return DEFAULT_PEDAL_NUMBER;
}

/**
 * Adds the pedal number to the USB serial number.
 *
 * The AVR core builds the USB serial number from the short names of all PluggableUSB
 * modules (the Keyboard's HID module contributes "HIDxx"). This module has no interfaces
 * or endpoints and only appends "PAAPnn", so the configurator app, /dev/serial/by-id and
 * other tools can tell pedals apart without opening their serial ports. The name is built
 * when the host requests it, so it always shows the pedal number stored in EEPROM.
 */
class PedalId : public PluggableUSBModule {
    public:
        PedalId() : PluggableUSBModule(0, 0, NULL) {
            PluggableUSB().plug(this);
        }

    protected:
        bool setup(USBSetup&) override {
            return false;
        }

        int getInterface(uint8_t*) override {
            return 0;
        }

        int getDescriptor(USBSetup&) override {
            return 0;
        }

        uint8_t getShortName(char* name) override {
            uint8_t number = readPedalNumber();
            name[0] = 'P';
            name[1] = 'A';
            name[2] = 'A';
            name[3] = 'P';
            name[4] = '0' + number / 10;
            name[5] = '0' + number % 10;
            return 6;
        }
};

PedalId pedalId;

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
 * Makes the host enumerate the device again.
 *
 * Detaches from the USB bus for a moment, so the host reads the USB serial number with the
 * new pedal number. The serial port disappears and comes back, possibly under a new name.
 */
void reconnectUsb() {
    // Give the host time to read the pending response.
    Serial.flush();
    delay(100);
    Keyboard.releaseAll();

    UDCON |= (1 << DETACH);
    delay(500);
    UDCON &= ~(1 << DETACH);
}

/**
 * Processes incoming serial commands.
 *
 * Handles the following commands:
 * - HEL: Responds with "LO\n"
 * - KEY: Responds with "KEY<keycode>\n"
 * - KEY<key>: Sets new keycode and responds with "ACK\n"
 * - VER: Responds with "VER<version>\n"
 * - PED: Responds with "PED<number>\n"
 * - PED<number>: Sets the pedal number (1-16), responds with "ACK\n" and reconnects USB,
 *   or responds with "ERR\n" if the number is invalid
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
        Serial.println(FIRMWARE_VERSION_STRING);
    } else if (cmd == "PED") {
        Serial.print("PED");
        Serial.println(readPedalNumber());
    } else if (cmd.startsWith("PED")) {
        String digits = cmd.substring(3);
        long number = digits.toInt();
        bool isNumber = digits.length() <= 2 && String(number) == digits;
        if (isNumber && number >= MIN_PEDAL_NUMBER && number <= MAX_PEDAL_NUMBER) {
            if (number != readPedalNumber()) {
                EEPROM.write(EEPROM_PEDAL_ADDR, (uint8_t)number);
                Serial.println("ACK");
                reconnectUsb();
            } else {
                Serial.println("ACK");
            }
        } else {
            Serial.println("ERR");
        }
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