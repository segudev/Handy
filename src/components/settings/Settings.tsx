import React from "react";
// import { AIConfig } from "./AIConfig";
import { KeyboardShortcuts } from "./KeyboardShortcuts";

export const Settings: React.FC = () => {
  return (
    <div className="min-h-screen bg-gray-100 py-8 px-4">
      <div className="max-w-3xl mx-auto space-y-8">
        <h1 className="text-3xl font-bold text-gray-900">Settings</h1>

        {/* AI Configuration Section */}
        <div className="bg-white shadow rounded-lg p-6">
          <h2 className="text-xl font-semibold text-gray-900 mb-4">
            AI Configuration
          </h2>
          {/* <AIConfig /> */}
        </div>

        {/* Keyboard Shortcuts Section */}
        <div className="bg-white shadow rounded-lg p-6">
          <h2 className="text-xl font-semibold text-gray-900 mb-4">
            Keyboard Shortcuts
          </h2>
          <KeyboardShortcuts />
        </div>
      </div>
    </div>
  );
};
