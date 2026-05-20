import { useState } from "react";
import { api } from "../api";
import {
  Callout,
  Card,
  Eyebrow,
  Heading,
  PrimaryButton,
} from "../components/ui";
import { isWindows, isMac, isLinux } from "../platform";

export function Consent() {
  const [general, setGeneral] = useState(false);
  const [recovery, setRecovery] = useState(false);

  const ready = general && recovery;

  return (
    <div className="space-y-6">
      <div className="flex items-end justify-between gap-4">
        <div>
          <Eyebrow>Step 1 · Welcome</Eyebrow>
          <Heading>Install CrossPoint on your Xteink</Heading>
        </div>
        <PrimaryButton
          disabled={!ready}
          onClick={() => api.acceptConsent(general, recovery)}
        >
          Continue
        </PrimaryButton>
      </div>

      <Card>
        <h2 className="font-serif text-lg font-medium text-stone-900">
          What Unlocker will do
        </h2>
        <ul className="mt-3 space-y-2 text-sm/6 text-stone-600">
          <li>
            {isWindows()
              ? "– Briefly turn on Windows Mobile Hotspot to create a private network for your device. Your Wi-Fi adapter will be in use during the install."
              : isMac()
              ? "– Briefly disconnect your Mac's Wi-Fi to create a private hotspot for your device. Wired Ethernet is unaffected."
              : isLinux()
              ? "– Briefly turn on Linux Wi-Fi Hotspot to create a private network for your device. Your Wi-Fi adapter will be in use during the install."
              : "Don't know what your system is, please report this..."
            }
          </li>
          <li>
            – Replace the firmware on the Xteink with CrossPoint, using the
            device's own native update mechanism.
          </li>
        </ul>

        <div className="mt-6 space-y-3">
          <label className="flex items-start gap-3 text-sm/6 text-stone-700">
            <input
              type="checkbox"
              checked={general}
              onChange={(e) => setGeneral(e.target.checked)}
              className="mt-1 size-4 rounded border-stone-300 text-brand-500 focus:ring-brand-500"
            />
            <span>
              I understand Unlocker will modify network settings on this
              computer and install firmware on my device.
            </span>
          </label>
          <label className="flex items-start gap-3 text-sm/6 text-stone-700">
            <input
              type="checkbox"
              checked={recovery}
              onChange={(e) => setRecovery(e.target.checked)}
              className="mt-1 size-4 rounded border-stone-300 text-brand-500 focus:ring-brand-500"
            />
            <span>
              I understand that if my device has the USB lockdown,{" "}
              <strong>recovery from a failed install is significantly harder</strong>
              . In the worst case, a failed install can leave the device
              non-functional.
            </span>
          </label>
        </div>
      </Card>

      <Callout variant="warn" title="Already tried the WebSerial flasher?">
        If your Xteink wouldn't enter download mode and the flasher couldn't
        connect, that's the lockdown Unlocker is designed for. You're in the
        right place.{" "}
        <a
          href="https://crosspointreader.com#flash-tools"
          target="_blank"
          rel="noreferrer"
          className="font-medium underline underline-offset-2"
        >
          If USB still works for you
        </a>
        , the WebSerial flasher is a safer first try.
      </Callout>

      <p className="text-xs text-stone-400">
        Need to repair network settings or stop the helper? Use the gear icon
        in the top-right.
      </p>
    </div>
  );
}
