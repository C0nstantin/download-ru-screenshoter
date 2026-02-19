import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";

interface HotkeyConfig {
  region: string;
  fullscreen: string;
  window: string;
}

function MainPage() {
  const [isLoggedIn, setIsLoggedIn] = useState(false);
  const [oauthCode, setOauthCode] = useState("");
  const [showCodeInput, setShowCodeInput] = useState(false);
  const [status, setStatus] = useState("");
  const [hotkeys, setHotkeys] = useState<HotkeyConfig>({
    region: "Ctrl+Shift+4",
    fullscreen: "Ctrl+Shift+3",
    window: "Ctrl+Shift+Alt+3",
  });
  const [editingHotkey, setEditingHotkey] = useState<"region" | "fullscreen" | "window" | null>(null);
  const hotkeyInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    // Load saved token on mount
    invoke<boolean>("load_saved_token")
      .then((loaded) => setIsLoggedIn(loaded))
      .catch(console.error);

    // Load hotkeys
    invoke<HotkeyConfig>("get_hotkeys")
      .then(setHotkeys)
      .catch(console.error);
  }, []);

  const handleLogin = async () => {
    try {
      await invoke("open_oauth_browser");
      setShowCodeInput(true);
      setStatus("Войдите в браузере и вставьте полученный код");
    } catch (err) {
      setStatus(`Ошибка: ${err}`);
    }
  };

  const handleExchangeCode = async () => {
    if (!oauthCode.trim()) return;
    try {
      setStatus("Получаем токен...");
      await invoke("exchange_oauth_code", { code: oauthCode.trim() });
      setIsLoggedIn(true);
      setShowCodeInput(false);
      setOauthCode("");
      setStatus("Успешно авторизованы!");
      setTimeout(() => setStatus(""), 3000);
    } catch (err) {
      setStatus(`Ошибка авторизации: ${err}`);
    }
  };

  const handleLogout = async () => {
    try {
      await invoke("logout");
      setIsLoggedIn(false);
      setShowCodeInput(false);
      setOauthCode("");
      setStatus("Выход выполнен");
      setTimeout(() => setStatus(""), 3000);
    } catch (err) {
      setStatus(`Ошибка: ${err}`);
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

    // Get the key
    let key = e.key;
    if (key === "Control" || key === "Meta" || key === "Alt" || key === "Shift") {
      return; // Don't save modifier-only
    }

    // Normalize key names
    if (key.length === 1) {
      key = key.toUpperCase();
    } else if (key.startsWith("Digit")) {
      key = key.replace("Digit", "");
    } else if (key.startsWith("Key")) {
      key = key.replace("Key", "");
    }

    parts.push(key);

    if (parts.length < 2) {
      setStatus("Нужно использовать модификатор (Ctrl, Cmd, Alt, Shift)");
      setTimeout(() => setStatus(""), 3000);
      return;
    }

    const shortcut = parts.join("+");
    const newHotkeys = { ...hotkeys, [type]: shortcut };
    setHotkeys(newHotkeys);
    setEditingHotkey(null);

    // Save to backend
    invoke("set_hotkeys", { config: newHotkeys })
      .then(() => {
        setStatus("Горячие клавиши сохранены!");
        setTimeout(() => setStatus(""), 3000);
      })
      .catch((err) => {
        setStatus(`Ошибка: ${err}`);
      });
  };

  const startEditingHotkey = (type: "region" | "fullscreen" | "window") => {
    setEditingHotkey(type);
    setTimeout(() => hotkeyInputRef.current?.focus(), 0);
  };

  return (
    <main className="main-page">
      <h1>Download Screenshoter</h1>

      <section className="section">
        <h2>Горячие клавиши</h2>
        <div className="shortcuts">
          <div className="shortcut">
            {editingHotkey === "region" ? (
              <input
                ref={hotkeyInputRef}
                type="text"
                className="hotkey-input"
                placeholder="Нажмите комбинацию..."
                onKeyDown={(e) => handleHotkeyKeyDown(e, "region")}
                onBlur={() => setEditingHotkey(null)}
                readOnly
              />
            ) : (
              <button className="hotkey-btn" onClick={() => startEditingHotkey("region")}>
                {hotkeys.region.split("+").map((k, i) => (
                  <kbd key={i}>{k}</kbd>
                ))}
              </button>
            )}
            <span>Скриншот области</span>
          </div>
          <div className="shortcut">
            {editingHotkey === "fullscreen" ? (
              <input
                ref={hotkeyInputRef}
                type="text"
                className="hotkey-input"
                placeholder="Нажмите комбинацию..."
                onKeyDown={(e) => handleHotkeyKeyDown(e, "fullscreen")}
                onBlur={() => setEditingHotkey(null)}
                readOnly
              />
            ) : (
              <button className="hotkey-btn" onClick={() => startEditingHotkey("fullscreen")}>
                {hotkeys.fullscreen.split("+").map((k, i) => (
                  <kbd key={i}>{k}</kbd>
                ))}
              </button>
            )}
            <span>Скриншот экрана</span>
          </div>
          <div className="shortcut">
            {editingHotkey === "window" ? (
              <input
                ref={hotkeyInputRef}
                type="text"
                className="hotkey-input"
                placeholder="Нажмите комбинацию..."
                onKeyDown={(e) => handleHotkeyKeyDown(e, "window")}
                onBlur={() => setEditingHotkey(null)}
                readOnly
              />
            ) : (
              <button className="hotkey-btn" onClick={() => startEditingHotkey("window")}>
                {hotkeys.window.split("+").map((k, i) => (
                  <kbd key={i}>{k}</kbd>
                ))}
              </button>
            )}
            <span>Скриншот окна</span>
          </div>
        </div>
        <p className="hint" style={{ marginTop: "10px" }}>
          Кликните на комбинацию для изменения
        </p>
      </section>

      <section className="section">
        <h2>Аккаунт download.ru</h2>

        {isLoggedIn ? (
          <div className="auth-status">
            <p className="token-status success">Вы авторизованы</p>
            <button onClick={handleLogout} className="danger">
              Выйти
            </button>
          </div>
        ) : (
          <div className="auth-form">
            {!showCodeInput ? (
              <>
                <p className="hint">
                  Войдите в аккаунт download.ru для загрузки скриншотов
                </p>
                <button onClick={handleLogin} className="primary">
                  Войти через download.ru
                </button>
              </>
            ) : (
              <>
                <p className="hint">
                  Браузер открыт. После входа скопируйте код и вставьте сюда:
                </p>
                <div className="token-form">
                  <input
                    type="text"
                    value={oauthCode}
                    onChange={(e) => setOauthCode(e.target.value)}
                    onKeyDown={(e) => e.key === "Enter" && handleExchangeCode()}
                    placeholder="Вставьте код авторизации..."
                    className="token-input"
                    autoFocus
                  />
                  <div className="token-buttons">
                    <button onClick={handleExchangeCode} disabled={!oauthCode.trim()}>
                      Подтвердить
                    </button>
                    <button onClick={() => { setShowCodeInput(false); setOauthCode(""); }}>
                      Отмена
                    </button>
                  </div>
                </div>
              </>
            )}
          </div>
        )}

        {status && <p className="status">{status}</p>}
      </section>

      <section className="section">
        <h2>Как использовать</h2>
        <ol>
          <li>Нажмите <kbd>Ctrl+Shift+4</kbd> для выделения области</li>
          <li>Отредактируйте скриншот (стрелки, рамки, текст)</li>
          <li>Нажмите "Загрузить" - ссылка скопируется в буфер</li>
        </ol>
      </section>
    </main>
  );
}

export default MainPage;
