import { useEffect, useState } from "react";
import MainPage from "./pages/MainPage";
import OverlayPage from "./pages/OverlayPage";
import EditorPage from "./pages/EditorPage";
import "./App.css";

function App() {
  const [route, setRoute] = useState<string>("/");

  useEffect(() => {
    // Simple hash-based routing for multi-window support
    const hash = window.location.hash.replace("#", "") || "/";
    setRoute(hash);

    const handleHashChange = () => {
      const newHash = window.location.hash.replace("#", "") || "/";
      setRoute(newHash);
    };

    window.addEventListener("hashchange", handleHashChange);
    return () => window.removeEventListener("hashchange", handleHashChange);
  }, []);

  switch (route) {
    case "/overlay":
      return <OverlayPage />;
    case "/editor":
      return <EditorPage />;
    default:
      return <MainPage />;
  }
}

export default App;
