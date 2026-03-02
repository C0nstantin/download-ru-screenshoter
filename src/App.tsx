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
    const allowedRoutes = ["/", "/overlay", "/overlay-video", "/editor", "/recording", "/video-result"];
    const sanitize = (raw: string) => {
      // Strip query params from hash (e.g. "#/editor?t=123" → "/editor")
      const h = raw.replace("#", "").split("?")[0] || "/";
      return allowedRoutes.includes(h) ? h : "/";
    };

    setRoute(sanitize(window.location.hash));

    const handleHashChange = () => {
      setRoute(sanitize(window.location.hash));
    };

    window.addEventListener("hashchange", handleHashChange);
    return () => window.removeEventListener("hashchange", handleHashChange);
  }, []);

  switch (route) {
    case "/overlay":
      return <OverlayPage mode="screenshot" />;
    case "/overlay-video":
      return <OverlayPage mode="video" />;
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
