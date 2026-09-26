#pragma once

// Firmware version, reported to the app in the response to the VER command.
// CI overwrites this file with the version from scripts/version.sh. The committed 0.0.0
// marks local builds as unofficial.
#define FIRMWARE_VERSION_MAJOR 0
#define FIRMWARE_VERSION_MINOR 0
#define FIRMWARE_VERSION_PATCH 0
#define FIRMWARE_VERSION_STRING "0.0.0"
