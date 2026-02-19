import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

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

  useEffect(() => {
    invoke<boolean>("load_saved_token")
      .then((loaded) => setIsLoggedIn(loaded))
      .catch(console.error);

    invoke<HotkeyConfig>("get_hotkeys")
      .then(setHotkeys)
      .catch(console.error);

    // Auto-capture when OAuth completes in webview
    const unlisten = listen<boolean>("oauth-complete", (event) => {
      setIsWaiting(false);
      if (event.payload) {
        setIsLoggedIn(true);
        setStatus("Успешно авторизованы!");
        setTimeout(() => setStatus(""), 3000);
      } else {
        setStatus("Ошибка авторизации. Попробуйте ещё раз.");
      }
    });

    return () => { unlisten.then((f) => f()); };
  }, []);

  const handleLogin = async () => {
    try {
      setIsWaiting(true);
      setStatus("");
      await invoke("open_oauth_browser");
    } catch (err) {
      setIsWaiting(false);
      setStatus(`Ошибка: ${err}`);
    }
  };

  const handleLogout = async () => {
    try {
      await invoke("logout");
      setIsLoggedIn(false);
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

    let key = e.key;
    if (key === "Control" || key === "Meta" || key === "Alt" || key === "Shift") return;
    if (key.length === 1) key = key.toUpperCase();
    else if (key.startsWith("Digit")) key = key.replace("Digit", "");
    else if (key.startsWith("Key")) key = key.replace("Key", "");
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

    invoke("set_hotkeys", { config: newHotkeys })
      .then(() => { setStatus("Горячие клавиши сохранены!"); setTimeout(() => setStatus(""), 3000); })
      .catch((err) => setStatus(`Ошибка: ${err}`));
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
          {(["region", "fullscreen", "window"] as const).map((type) => (
            <div className="shortcut" key={type}>
              {editingHotkey === type ? (
                <input
                  ref={hotkeyInputRef}
                  type="text"
                  className="hotkey-input"
                  placeholder="Нажмите комбинацию..."
                  onKeyDown={(e) => handleHotkeyKeyDown(e, type)}
                  onBlur={() => setEditingHotkey(null)}
                  readOnly
                />
              ) : (
                <button className="hotkey-btn" onClick={() => startEditingHotkey(type)}>
                  {hotkeys[type].split("+").map((k, i) => <kbd key={i}>{k}</kbd>)}
                </button>
              )}
              <span>
                {type === "region" ? "Скриншот области" : type === "fullscreen" ? "Скриншот экрана" : "Скриншот окна"}
              </span>
            </div>
          ))}
        </div>
        <p className="hint" style={{ marginTop: "10px" }}>Кликните на комбинацию для изменения</p>
      </section>

      <section className="section">
        <h2>Аккаунт download.ru</h2>
        {isLoggedIn ? (
          <div className="auth-status">
            <p className="token-status success">Вы авторизованы</p>
            <button onClick={handleLogout} className="danger">Выйти</button>
          </div>
        ) : (
          <div className="auth-form">
            <p className="hint">Войдите в аккаунт download.ru для загрузки скриншотов</p>
            <button onClick={handleLogin} className="primary" disabled={isWaiting}>
              {isWaiting ? "Ожидаем авторизацию..." : "Войти через download.ru"}
            </button>
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
