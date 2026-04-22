#include "CrossPointSettings.h"

#include
#include
#include
#include

#include
#include

#include "fontIds.h"

// Initialize the static instance
CrossPointSettings CrossPointSettings::instance;

void readAndValidate(FsFile& file, uint8\_t& member, const uint8\_t maxValue) {
 uint8\_t tempValue;
 serialization::readPod(file, tempValue);
 if (tempValue < maxValue) {
 member = tempValue;
 }
}

namespace {
constexpr uint8\_t SETTINGS\_FILE\_VERSION = 1;
constexpr char SETTINGS\_FILE\_BIN\[\] = "/.crosspoint/settings.bin";
constexpr char SETTINGS\_FILE\_JSON\[\] = "/.crosspoint/settings.json";
constexpr char SETTINGS\_FILE\_BAK\[\] = "/.crosspoint/settings.bin.bak";

// Convert legacy front button layout into explicit logical->hardware mapping.
void applyLegacyFrontButtonLayout(CrossPointSettings& settings) {
 switch (static\_cast(settings.frontButtonLayout)) {
 case CrossPointSettings::LEFT\_RIGHT\_BACK\_CONFIRM:
 settings.frontButtonBack = CrossPointSettings::FRONT\_HW\_LEFT;
 settings.frontButtonConfirm = CrossPointSettings::FRONT\_HW\_RIGHT;
 settings.frontButtonLeft = CrossPointSettings::FRONT\_HW\_BACK;
 settings.frontButtonRight = CrossPointSettings::FRONT\_HW\_CONFIRM;
 break;
 case CrossPointSettings::LEFT\_BACK\_CONFIRM\_RIGHT:
 settings.frontButtonBack = CrossPointSettings::FRONT\_HW\_CONFIRM;
 settings.frontButtonConfirm = CrossPointSettings::FRONT\_HW\_LEFT;
 settings.frontButtonLeft = CrossPointSettings::FRONT\_HW\_BACK;
 settings.frontButtonRight = CrossPointSettings::FRONT\_HW\_RIGHT;
 break;
 case CrossPointSettings::BACK\_CONFIRM\_RIGHT\_LEFT:
 settings.frontButtonBack = CrossPointSettings::FRONT\_HW\_BACK;
 settings.frontButtonConfirm = CrossPointSettings::FRONT\_HW\_CONFIRM;
 settings.frontButtonLeft = CrossPointSettings::FRONT\_HW\_RIGHT;
 settings.frontButtonRight = CrossPointSettings::FRONT\_HW\_LEFT;
 break;
 case CrossPointSettings::BACK\_CONFIRM\_LEFT\_RIGHT:
 default:
 settings.frontButtonBack = CrossPointSettings::FRONT\_HW\_BACK;
 settings.frontButtonConfirm = CrossPointSettings::FRONT\_HW\_CONFIRM;
 settings.frontButtonLeft = CrossPointSettings::FRONT\_HW\_LEFT;
 settings.frontButtonRight = CrossPointSettings::FRONT\_HW\_RIGHT;
 break;
 }
}

} // namespace

void CrossPointSettings::validateFrontButtonMapping(CrossPointSettings& settings) {
 const uint8\_t mapping\[\] = {settings.frontButtonBack, settings.frontButtonConfirm, settings.frontButtonLeft,
 settings.frontButtonRight};
 for (size\_t i = 0; i < 4; i++) {
 for (size\_t j = i + 1; j < 4; j++) {
 if (mapping\[i\] == mapping\[j\]) {
 settings.frontButtonBack = FRONT\_HW\_BACK;
 settings.frontButtonConfirm = FRONT\_HW\_CONFIRM;
 settings.frontButtonLeft = FRONT\_HW\_LEFT;
 settings.frontButtonRight = FRONT\_HW\_RIGHT;
 return;
 }
 }
 }
}

bool CrossPointSettings::saveToFile() const {
 Storage.mkdir("/.crosspoint");
 return JsonSettingsIO::saveSettings(\*this, SETTINGS\_FILE\_JSON);
}

bool CrossPointSettings::loadFromFile() {
 // Try JSON first
 if (Storage.exists(SETTINGS\_FILE\_JSON)) {
 String json = Storage.readFile(SETTINGS\_FILE\_JSON);
 if (!json.isEmpty()) {
 bool resave = false;
 bool result = JsonSettingsIO::loadSettings(\*this, json.c\_str(), &resave);
 if (result && resave) {
 if (saveToFile()) {
 LOG\_DBG("CPS", "Resaved settings to update format");
 } else {
 LOG\_ERR("CPS", "Failed to resave settings after format update");
 }
 }
 return result;
 }
 }

 // Fall back to binary migration
 if (Storage.exists(SETTINGS\_FILE\_BIN)) {
 if (loadFromBinaryFile()) {
 if (saveToFile()) {
 Storage.rename(SETTINGS\_FILE\_BIN, SETTINGS\_FILE\_BAK);
 LOG\_DBG("CPS", "Migrated settings.bin to settings.json");
 return true;
 } else {
 LOG\_ERR("CPS", "Failed to save migrated settings to JSON");
 return false;
 }
 }
 }

 return false;
}

bool CrossPointSettings::loadFromBinaryFile() {
 FsFile inputFile;
 if (!Storage.openFileForRead("CPS", SETTINGS\_FILE\_BIN, inputFile)) {
 return false;
 }

 uint8\_t version;
 serialization::readPod(inputFile, version);
 if (version != SETTINGS\_FILE\_VERSION) {
 LOG\_ERR("CPS", "Deserialization failed: Unknown version %u", version);
 return false;
 }

 uint8\_t fileSettingsCount = 0;
 serialization::readPod(inputFile, fileSettingsCount);

 uint8\_t settingsRead = 0;
 bool frontButtonMappingRead = false;
 do {
 readAndValidate(inputFile, sleepScreen, SLEEP\_SCREEN\_MODE\_COUNT);
 if (++settingsRead >= fileSettingsCount) break;
 serialization::readPod(inputFile, extraParagraphSpacing);
 if (++settingsRead >= fileSettingsCount) break;
 readAndValidate(inputFile, shortPwrBtn, SHORT\_PWRBTN\_COUNT);
 if (++settingsRead >= fileSettingsCount) break;
 readAndValidate(inputFile, statusBar, STATUS\_BAR\_MODE\_COUNT); // legacy
 if (++settingsRead >= fileSettingsCount) break;
 readAndValidate(inputFile, orientation, ORIENTATION\_COUNT);
 if (++settingsRead >= fileSettingsCount) break;
 readAndValidate(inputFile, frontButtonLayout, FRONT\_BUTTON\_LAYOUT\_COUNT);
 if (++settingsRead >= fileSettingsCount) break;
 readAndValidate(inputFile, sideButtonLayout, SIDE\_BUTTON\_LAYOUT\_COUNT);
 if (++settingsRead >= fileSettingsCount) break;
 readAndValidate(inputFile, fontFamily, FONT\_FAMILY\_COUNT);
 if (++settingsRead >= fileSettingsCount) break;
 readAndValidate(inputFile, fontSize, FONT\_SIZE\_COUNT);
 if (++settingsRead >= fileSettingsCount) break;
 readAndValidate(inputFile, lineSpacing, LINE\_COMPRESSION\_COUNT);
 if (++settingsRead >= fileSettingsCount) break;
 readAndValidate(inputFile, paragraphAlignment, PARAGRAPH\_ALIGNMENT\_COUNT);
 if (++settingsRead >= fileSettingsCount) break;
 readAndValidate(inputFile, sleepTimeout, SLEEP\_TIMEOUT\_COUNT);
 if (++settingsRead >= fileSettingsCount) break;
 readAndValidate(inputFile, refreshFrequency, REFRESH\_FREQUENCY\_COUNT);
 if (++settingsRead >= fileSettingsCount) break;
 serialization::readPod(inputFile, screenMargin);
 if (++settingsRead >= fileSettingsCount) break;
 readAndValidate(inputFile, sleepScreenCoverMode, SLEEP\_SCREEN\_COVER\_MODE\_COUNT);
 if (++settingsRead >= fileSettingsCount) break;
 {
 std::string urlStr;
 serialization::readString(inputFile, urlStr);
 strncpy(opdsServerUrl, urlStr.c\_str(), sizeof(opdsServerUrl) - 1);
 opdsServerUrl\[sizeof(opdsServerUrl) - 1\] = '\\0';
 }
 if (++settingsRead >= fileSettingsCount) break;
 serialization::readPod(inputFile, textAntiAliasing);
 if (++settingsRead >= fileSettingsCount) break;
 readAndValidate(inputFile, hideBatteryPercentage, HIDE\_BATTERY\_PERCENTAGE\_COUNT);
 if (++settingsRead >= fileSettingsCount) break;
 serialization::readPod(inputFile, longPressChapterSkip);
 if (++settingsRead >= fileSettingsCount) break;
 serialization::readPod(inputFile, hyphenationEnabled);
 if (++settingsRead >= fileSettingsCount) break;
 {
 std::string usernameStr;
 serialization::readString(inputFile, usernameStr);
 strncpy(opdsUsername, usernameStr.c\_str(), sizeof(opdsUsername) - 1);
 opdsUsername\[sizeof(opdsUsername) - 1\] = '\\0';
 }
 if (++settingsRead >= fileSettingsCount) break;
 {
 std::string passwordStr;
 serialization::readString(inputFile, passwordStr);
 strncpy(opdsPassword, passwordStr.c\_str(), sizeof(opdsPassword) - 1);
 opdsPassword\[sizeof(opdsPassword) - 1\] = '\\0';
 }
 if (++settingsRead >= fileSettingsCount) break;
 readAndValidate(inputFile, sleepScreenCoverFilter, SLEEP\_SCREEN\_COVER\_FILTER\_COUNT);
 if (++settingsRead >= fileSettingsCount) break;
 serialization::readPod(inputFile, uiTheme);
 if (++settingsRead >= fileSettingsCount) break;
 readAndValidate(inputFile, frontButtonBack, FRONT\_BUTTON\_HARDWARE\_COUNT);
 if (++settingsRead >= fileSettingsCount) break;
 readAndValidate(inputFile, frontButtonConfirm, FRONT\_BUTTON\_HARDWARE\_COUNT);
 if (++settingsRead >= fileSettingsCount) break;
 readAndValidate(inputFile, frontButtonLeft, FRONT\_BUTTON\_HARDWARE\_COUNT);
 if (++settingsRead >= fileSettingsCount) break;
 readAndValidate(inputFile, frontButtonRight, FRONT\_BUTTON\_HARDWARE\_COUNT);
 frontButtonMappingRead = true;
 if (++settingsRead >= fileSettingsCount) break;
 serialization::readPod(inputFile, fadingFix);
 if (++settingsRead >= fileSettingsCount) break;
 serialization::readPod(inputFile, embeddedStyle);
 if (++settingsRead >= fileSettingsCount) break;
 } while (false);

 if (frontButtonMappingRead) {
 CrossPointSettings::validateFrontButtonMapping(\*this);
 } else {
 applyLegacyFrontButtonLayout(\*this);
 }

 LOG\_DBG("CPS", "Settings loaded from binary file");
 return true;
}

float CrossPointSettings::getReaderLineCompression() const {
 switch (fontFamily) {
 case NOTOSERIF:
 default:
 switch (lineSpacing) {
 case TIGHT:
 return 0.95f;
 case NORMAL:
 default:
 return 1.0f;
 case WIDE:
 return 1.1f;
 }
 case NOTOSANS:
 switch (lineSpacing) {
 case TIGHT:
 return 0.90f;
 case NORMAL:
 default:
 return 0.95f;
 case WIDE:
 return 1.0f;
 }
 case OPENDYSLEXIC:
 switch (lineSpacing) {
 case TIGHT:
 return 0.90f;
 case NORMAL:
 default:
 return 0.95f;
 case WIDE:
 return 1.0f;
 }
 }
}

unsigned long CrossPointSettings::getSleepTimeoutMs() const {
 switch (sleepTimeout) {
 case SLEEP\_1\_MIN:
 return 1UL \* 60 \* 1000;
 case SLEEP\_5\_MIN:
 return 5UL \* 60 \* 1000;
 case SLEEP\_10\_MIN:
 default:
 return 10UL \* 60 \* 1000;
 case SLEEP\_15\_MIN:
 return 15UL \* 60 \* 1000;
 case SLEEP\_30\_MIN:
 return 30UL \* 60 \* 1000;
 }
}

int CrossPointSettings::getRefreshFrequency() const {
 switch (refreshFrequency) {
 case REFRESH\_1:
 return 1;
 case REFRESH\_5:
 return 5;
 case REFRESH\_10:
 return 10;
 case REFRESH\_15:
 default:
 return 15;
 case REFRESH\_30:
 return 30;
 }
}

int CrossPointSettings::getReaderFontId() const {
 switch (fontFamily) {
 case NOTOSERIF:
 default:
 switch (fontSize) {
 case SMALL:
 return NOTOSERIF\_12\_FONT\_ID;
 case MEDIUM:
 default:
 return NOTOSERIF\_14\_FONT\_ID;
 case LARGE:
 return NOTOSERIF\_16\_FONT\_ID;
 case EXTRA\_LARGE:
 return NOTOSERIF\_18\_FONT\_ID;
 }
 case NOTOSANS:
 switch (fontSize) {
 case SMALL:
 return NOTOSANS\_12\_FONT\_ID;
 case MEDIUM:
 default:
 return NOTOSANS\_14\_FONT\_ID;
 case LARGE:
 return NOTOSANS\_16\_FONT\_ID;
 case EXTRA\_LARGE:
 return NOTOSANS\_18\_FONT\_ID;
 }
 case OPENDYSLEXIC:
 switch (fontSize) {
 case SMALL:
 return OPENDYSLEXIC\_8\_FONT\_ID;
 case MEDIUM:
 default:
 return OPENDYSLEXIC\_10\_FONT\_ID;
 case LARGE:
 return OPENDYSLEXIC\_12\_FONT\_ID;
 case EXTRA\_LARGE:
 return OPENDYSLEXIC\_14\_FONT\_ID;
 }
 }
}