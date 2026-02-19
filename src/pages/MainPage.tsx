import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";

interface HotkeyConfig {
  region: string;
  fullscreen: string;
  window: string;
}

function MainPage() {
  const [token, setToken] = useState("");
  const [savedToken, setSavedToken] = useState<string | null>(null);
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
    invoke<string | null>("get_access_token")
      .then((t) => {
        if (t) {
          setSavedToken(t);
          setToken(t);
        }
      })
      .catch(console.error);

    // Load hotkeys
    invoke<HotkeyConfig>("get_hotkeys")
      .then(setHotkeys)
      .catch(console.error);
  }, []);

  const handleSaveToken = async () => {
    try {
      await invoke("set_access_token", { token });
      setSavedToken(token);
      setStatus("Токен сохранён!");
      setTimeout(() => setStatus(""), 3000);
    } catch (err) {
      setStatus(`Ошибка: ${err}`);
    }
  };

  const handleClearToken = async () => {
    try {
      await invoke("set_access_token", { token: "" });
      setSavedToken(null);
      setToken("");
      setStatus("Токен удалён");
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
        <h2>Настройки download.ru</h2>
        <p className="hint">
          Для загрузки скриншотов нужен OAuth токен.
          <br />
          Получите его в настройках профиля на download.ru
        </p>

        <div className="token-form">
          <input
            type="password"
            value={token}
            onChange={(e) => setToken(e.target.value)}
            placeholder="Вставьте access token..."
            className="token-input"
          />
          <div className="token-buttons">
            <button onClick={handleSaveToken} disabled={!token}>
              Сохранить
            </button>
            {savedToken && (
              <button onClick={handleClearToken} className="danger">
                Удалить
              </button>
            )}
          </div>
        </div>

        {savedToken && (
          <p className="token-status success">Токен настроен</p>
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
