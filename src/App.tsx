import { useEffect, useState } from "react";

// import { invoke } from "@tauri-apps/api/core";
import "./App.css";

// import { isRegistered, register } from "@tauri-apps/plugin-global-shortcut";
import {
  checkAccessibilityPermissions,
  requestAccessibilityPermissions,
} from "tauri-plugin-macos-permissions-api";

function App() {
  // const [greetMsg, setGreetMsg] = useState("");
  // const [name, setName] = useState("");
  const [hasAccessibility, setHasAccessibility] = useState(false);

  // async function greet() {
  //   // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
  //   setGreetMsg(await invoke("greet", { name }));
  // }

  useEffect(() => {
    checkAccessibilityPermissions().then((hasPermissions) => {
      if (!hasPermissions) {
        requestAccessibilityPermissions().then((permissions) => {
          setHasAccessibility(permissions);
        });
        return;
      }
      setHasAccessibility(hasPermissions);
    });
  }, []);

  return (
    <main className="container">
      <h1 className="text-6xl text-stroke tracking-widest">handy</h1>

      <div className="grid grid-cols-2">
        accessibility perns: {hasAccessibility ? "true" : "false"}
      </div>
    </main>
  );
}

export default App;
