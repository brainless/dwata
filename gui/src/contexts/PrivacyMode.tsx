import type { JSX } from "solid-js";
import { createContext, createEffect, createSignal, useContext } from "solid-js";

type PrivacyModeContextValue = {
  isEnabled: () => boolean;
  setEnabled: (enabled: boolean) => void;
};

const PrivacyModeContext = createContext<PrivacyModeContextValue>();
const PRIVACY_MODE_STORAGE_KEY = "dwata:privacy-blur";

export function PrivacyModeProvider(props: { children: JSX.Element }) {
  const getInitialValue = (): boolean => {
    if (typeof window === "undefined") return false;
    const stored = window.localStorage.getItem(PRIVACY_MODE_STORAGE_KEY);
    if (stored === null) return false;
    return stored === "true";
  };

  const [isEnabled, setEnabled] = createSignal(getInitialValue());

  createEffect(() => {
    if (typeof window === "undefined") return;
    window.localStorage.setItem(
      PRIVACY_MODE_STORAGE_KEY,
      String(isEnabled()),
    );
  });

  return (
    <PrivacyModeContext.Provider value={{ isEnabled, setEnabled }}>
      {props.children}
    </PrivacyModeContext.Provider>
  );
}

export function usePrivacyMode(): PrivacyModeContextValue {
  const ctx = useContext(PrivacyModeContext);
  if (!ctx) {
    throw new Error("usePrivacyMode must be used within PrivacyModeProvider");
  }
  return ctx;
}
