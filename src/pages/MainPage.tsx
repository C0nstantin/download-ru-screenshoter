import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useTranslation } from "../i18n/useTranslation";
import { useI18nStore, Locale } from "../i18n/i18nStore";

interface HotkeyConfig {
  region: string;
  fullscreen: string;
  window: string;
}

function MainPage() {
  const [isLoggedIn, setIsLoggedIn] = useState(false);
  const [isWaiting, setIsWaiting] = useState(false);
  const [status, setStatus] = useState("");
  const [hotkeys, setHotkeys] = useState<HotkeyConfig>({
    region: "Ctrl+Shift+4",
    fullscreen: "Ctrl+Shift+3",
    window: "Ctrl+Shift+Alt+3",
  });
  const [editingHotkey, setEditingHotkey] = useState<"region" | "fullscreen" | "window" | null>(null);
  const hotkeyInputRef = useRef<HTMLInputElement>(null);
  const [devMode, setDevMode] = useState(false);
  const [isBeta, setIsBeta] = useState(false);
  const { t } = useTranslation();
  const { locale, setLocale } = useI18nStore();

  useEffect(() => {
    invoke<boolean>("load_saved_token")
      .then((loaded) => setIsLoggedIn(loaded))
      .catch(console.error);

    invoke<HotkeyConfig>("get_hotkeys")
      .then(setHotkeys)
      .catch(console.error);

    invoke<boolean>("is_dev_mode").then((dm) => {
      setDevMode(dm);
      if (dm) {
        invoke<string>("get_api_url").then((url) => setIsBeta(url.includes("beta.")));
      }
    });

    // Auto-capture when OAuth completes in webview
    const unlisten = listen<boolean>("oauth-complete", (event) => {
      setIsWaiting(false);
      if (event.payload) {
        setIsLoggedIn(true);
        setStatus(t("main.authSuccess"));
        setTimeout(() => setStatus(""), 3000);
      } else {
        setStatus(t("main.authError"));
      }
    });

    return () => { unlisten.then((f) => f()); };
  }, []);

  // Focus the hotkey input after React commits the re-render.
  // setTimeout(0) was unreliable: on first click the macrotask could fire
  // before the input was mounted, leaving the field visible but not focused,
  // so the first keystroke was dropped and a second click was needed.
  useEffect(() => {
    if (editingHotkey && hotkeyInputRef.current) {
      hotkeyInputRef.current.focus();
    }
  }, [editingHotkey]);

  const handleLogin = async () => {
    try {
      setIsWaiting(true);
      setStatus("");
      await invoke("open_oauth_browser");
    } catch (err) {
      setIsWaiting(false);
      setStatus(t("main.error", { msg: String(err) }));
    }
  };

  const handleLogout = async () => {
    try {
      await invoke("logout");
      setIsLoggedIn(false);
      setStatus(t("main.logoutDone"));
      setTimeout(() => setStatus(""), 3000);
    } catch (err) {
      setStatus(t("main.error", { msg: String(err) }));
    }
  };

  const handleHotkeyKeyDown = (e: React.KeyboardEvent, type: "region" | "fullscreen" | "window") => {
    e.preventDefault();
    e.stopPropagation();

    const parts: string[] = [];
    if (e.ctrlKey) parts.push("Ctrl");
    if (e.metaKey) parts.push("Cmd");
    if (e.altKey) parts.push("Alt");
    if (e.shiftKey) parts.push("Shift");

    const key = e.key;
    if (key === "Control" || key === "Meta" || key === "Alt" || key === "Shift") return;

    // Use e.code for physical key to avoid layout-specific chars (e.g. Shift+2 → "@")
    let normalized: string;
    if (e.code.startsWith("Digit")) {
      normalized = e.code.replace("Digit", "");
    } else if (e.code.startsWith("Key")) {
      normalized = e.code.replace("Key", "");
    } else if (key.length === 1) {
      normalized = key.toUpperCase();
    } else {
      normalized = key;
    }
    parts.push(normalized);

    if (parts.length < 2) {
      setStatus(t("main.needModifier"));
      setTimeout(() => setStatus(""), 3000);
      return;
    }

    const shortcut = parts.join("+");
    const newHotkeys = { ...hotkeys, [type]: shortcut };
    setHotkeys(newHotkeys);
    setEditingHotkey(null);

    invoke("set_hotkeys", { config: newHotkeys })
      .then(() => { setStatus(t("main.hotkeysSaved")); setTimeout(() => setStatus(""), 3000); })
      .catch((err) => setStatus(t("main.error", { msg: String(err) })));
  };

  const startEditingHotkey = async (type: "region" | "fullscreen" | "window") => {
    await invoke("unregister_hotkeys");
    setEditingHotkey(type);
    // Focus happens in the useEffect above — guaranteed to run after commit.
  };

  const cancelEditingHotkey = () => {
    setEditingHotkey(null);
    // Re-register current hotkeys
    invoke("set_hotkeys", { config: hotkeys }).catch(() => {});
  };

  const hotkeyLabels: Record<string, string> = {
    region: t("main.screenshotRegion"),
    fullscreen: t("main.screenshotFullscreen"),
    window: t("main.screenshotWindow"),
  };

  return (
    <main className="main-page">
      <h1>{t("main.title")}</h1>

      <section className="section">
        <h2>{t("main.hotkeys")}</h2>
        <div className="shortcuts">
          {(["region", "fullscreen", "window"] as const).map((type) => (
            <div className="shortcut" key={type}>
              {editingHotkey === type ? (
                <input
                  ref={hotkeyInputRef}
                  type="text"
                  className="hotkey-input"
                  placeholder={t("main.hotkeyPlaceholder")}
                  onKeyDown={(e) => handleHotkeyKeyDown(e, type)}
                  onBlur={() => cancelEditingHotkey()}
                  readOnly
                />
              ) : (
                <button className="hotkey-btn" onClick={() => startEditingHotkey(type)}>
                  {hotkeys[type].split("+").map((k, i) => <kbd key={i}>{k}</kbd>)}
                </button>
              )}
              <span>{hotkeyLabels[type]}</span>
            </div>
          ))}
        </div>
        <p className="hint" style={{ marginTop: "10px" }}>{t("main.hotkeyHint")}</p>
      </section>

      <section className="section">
        <h2>{t("main.account")}</h2>
        {isLoggedIn ? (
          <div className="auth-status">
            <p className="token-status success">{t("main.authorized")}</p>
            <button onClick={handleLogout} className="danger">{t("main.logout")}</button>
          </div>
        ) : (
          <div className="auth-form">
            <p className="hint">{t("main.loginHint")}</p>
            <button onClick={handleLogin} className="primary" disabled={isWaiting}>
              {isWaiting ? t("main.waitingAuth") : t("main.loginButton")}
            </button>
          </div>
        )}
        {status && <p className="status">{status}</p>}
      </section>

      <section className="section">
        <h2>{t("main.howToUse")}</h2>
        <ol>
          <li>{t("main.howToStep1", { hotkey: hotkeys.region })}</li>
          <li>{t("main.howToStep2")}</li>
          <li>{t("main.howToStep3")}</li>
        </ol>
      </section>

      <section className="section">
        <h2>{t("main.language")}</h2>
        <div style={{ display: "flex", gap: 8 }}>
          {(["ru", "en"] as const).map((l) => (
            <button
              key={l}
              className={locale === l ? "primary" : "secondary"}
              onClick={() => setLocale(l as Locale)}
            >
              {l === "ru" ? "Русский" : "English"}
            </button>
          ))}
        </div>
      </section>

      {devMode && (
        <section className="section" style={{ borderTop: "1px dashed #ccc", marginTop: 12, paddingTop: 12 }}>
          <label style={{ display: "flex", alignItems: "center", gap: 8, cursor: "pointer" }}>
            <input
              type="checkbox"
              checked={isBeta}
              onChange={async (e) => {
                const useBeta = e.target.checked;
                setIsBeta(useBeta);
                try {
                  const newUrl = await invoke<string>("toggle_beta_server", { useBeta });
                  setStatus(`API: ${newUrl}`);
                  setTimeout(() => setStatus(""), 3000);
                } catch (err) {
                  setStatus(t("main.error", { msg: String(err) }));
                }
              }}
            />
            Beta сервер (beta.download.ru)
          </label>
        </section>
      )}

      <section className="section" style={{ borderTop: "1px solid #eee", marginTop: 12, paddingTop: 12 }}>
        <button
          className="secondary"
          style={{ fontSize: 12, opacity: 0.7 }}
          onClick={async () => {
            try {
              const report = await invoke<Record<string, unknown>>("run_diagnostics");
              const text = JSON.stringify(report, null, 2);
              await navigator.clipboard.writeText(text);
              setStatus("Diagnostic copied to clipboard");
              setTimeout(() => setStatus(""), 3000);
            } catch (err) {
              setStatus(String(err));
            }
          }}
        >
          📋 Copy Diagnostics
        </button>
      </section>
    </main>
  );
}

export default MainPage;
