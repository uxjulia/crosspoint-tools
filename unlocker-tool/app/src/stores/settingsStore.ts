import { create } from "zustand";

const STORAGE_KEY = "xteink-unlocker-settings";

type PersistedSettings = {
  showCustomFirmwareOption: boolean;
  showPrereleaseFirmware: boolean;
};

type SettingsState = PersistedSettings & {
  setShowCustomFirmwareOption: (value: boolean) => void;
  setShowPrereleaseFirmware: (value: boolean) => void;
};

function loadSettings(): PersistedSettings {
  const defaults: PersistedSettings = {
    showCustomFirmwareOption: false,
    showPrereleaseFirmware: false,
  };

  if (typeof window === "undefined") return defaults;

  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) return defaults;
    const parsed = JSON.parse(raw) as Partial<PersistedSettings>;
    return {
      showCustomFirmwareOption: parsed.showCustomFirmwareOption === true,
      showPrereleaseFirmware: parsed.showPrereleaseFirmware === true,
    };
  } catch {
    return defaults;
  }
}

function saveSettings(settings: PersistedSettings) {
  window.localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
  ...loadSettings(),
  setShowCustomFirmwareOption: (showCustomFirmwareOption) => {
    set({ showCustomFirmwareOption });
    const { showPrereleaseFirmware } = get();
    saveSettings({ showCustomFirmwareOption, showPrereleaseFirmware });
  },
  setShowPrereleaseFirmware: (showPrereleaseFirmware) => {
    set({ showPrereleaseFirmware });
    const { showCustomFirmwareOption } = get();
    saveSettings({ showCustomFirmwareOption, showPrereleaseFirmware });
  },
}));
