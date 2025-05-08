import React, { useEffect } from "react";
import { load } from "@tauri-apps/plugin-store";
import {
  SettingsSchema,
  ShortcutBinding,
  ShortcutBindingSchema,
  ShortcutBindingsMap,
} from "../../lib/types";
import { invoke } from "@tauri-apps/api/core";

export const KeyboardShortcuts: React.FC = () => {
  const [bindings, setBindings] = React.useState<ShortcutBindingsMap>({});

  useEffect(() => {
    load("settings_store.json", { autoSave: false }).then((r) => {
      console.log("loaded store", r);

      r.get("settings").then((s) => {
        const settings = SettingsSchema.parse(s);
        setBindings(settings.bindings);
      });
    });

    // setTimeout(() => {
    //   console.log("invoked set binding");
    //   invoke("change_binding", { id: "test", binding: "alt+d" }).then((b) => {
    //     const newBinding = ShortcutBindingSchema.parse(b);
    //     console.log(bindings);
    //     setBindings((prev) => ({ ...prev, [newBinding.id]: newBinding }));
    //   });
    // }, 1000);
  }, []);

  return (
    <div className="space-y-4">
      {Object.entries(bindings).map(([id, binding]) => (
        <div
          key={id}
          className="flex items-center justify-between p-4 rounded-lg border border-gray-200 hover:bg-gray-50"
        >
          <div>
            <h3 className="text-sm font-medium text-gray-900">
              {binding.name}
            </h3>
            <p className="text-sm text-gray-500">{binding.description}</p>
          </div>
          <div className="flex items-center space-x-1">
            <div className="px-2 py-1 text-sm font-semibold text-gray-800 bg-gray-100 border border-gray-200 rounded">
              {binding.current_binding}
            </div>
            <button
              className="px-2 py-1 text-sm font-semibold text-gray-800 bg-gray-100 border border-gray-200 rounded hover:bg-gray-50"
              onClick={() => {
                invoke("reset_binding", { id }).then((b) => {
                  const newBinding = ShortcutBindingSchema.parse(b);
                  setBindings({ ...bindings, [newBinding.id]: newBinding });
                });
              }}
            >
              reset
            </button>
          </div>
        </div>
      ))}
    </div>
  );
};
