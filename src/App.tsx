import React from "react";
import { BrowserRouter, Routes, Route } from "react-router-dom";
import { Home } from "./pages/Home";
import { Meeting } from "./pages/Meeting";
import { Settings } from "./pages/Settings";

export default function App() {
  return (
    <BrowserRouter>
      <div style={{ minHeight: "100vh", background: "#f9fafb", fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif" }}>
        <Routes>
          <Route path="/" element={<Home />} />
          <Route path="/meeting/:id" element={<Meeting />} />
          <Route path="/settings" element={<Settings />} />
        </Routes>
      </div>
    </BrowserRouter>
  );
}
