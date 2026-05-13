"use client";

import { useState } from "react";
import { ExcelPreviewer } from "@hewliyang/xlsx-preview/react";

export default function Page() {
  const [file, setFile] = useState<File | null>(null);

  return (
    <div style={{ padding: 16 }}>
      <h1 style={{ margin: 0 }}>xlsx-preview · Next.js</h1>
      <p>
        <input
          type="file"
          accept=".xlsx"
          onChange={(e) => setFile(e.target.files?.[0] ?? null)}
        />
      </p>
      <div
        style={{
          height: "80vh",
          border: "1px solid #ddd",
          borderRadius: 8,
          overflow: "hidden",
        }}
      >
        <ExcelPreviewer file={file} style={{ width: "100%", height: "100%" }} />
      </div>
    </div>
  );
}
