import { useEffect, useState } from "react";
import MainPage from "./pages/MainPage";
import OverlayPage from "./pages/OverlayPage";
import EditorPage from "./pages/EditorPage";
import RecordingPage from "./pages/RecordingPage";
import VideoResultPage from "./pages/VideoResultPage";
import "./App.css";

function App() {
  const [route, setRoute] = useState<string>("/");

  useEffect(() => {
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
      return <OverlayPage mode="screenshot" />;
    case "/editor":
      return <EditorPage />;
    case "/recording":
      return <RecordingPage />;
    case "/video-result":
      return <VideoResultPage />;
    default:
      return <MainPage />;
  }
}

export default App;
