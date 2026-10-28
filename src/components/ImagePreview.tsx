import { useEffect, useRef, useState } from "react";
import { Download, Image as ImageIcon, AlertCircle } from "lucide-react";

interface ImagePreviewProps {
  imageData: string; // "rgba:W:H:base64data"
  width?: number;
}

export function ImagePreview({ imageData, width: propWidth }: ImagePreviewProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [error, setError] = useState(false);
  const [loaded, setLoaded] = useState(false);

  const parsed = parseImageData(imageData);

  useEffect(() => {
    if (!parsed || !canvasRef.current) return;
    const { w, h, data } = parsed;
    const canvas = canvasRef.current;

    try {
      // Decode base64 → Uint8Array
      const binaryStr = atob(data);
      const bytes = new Uint8Array(binaryStr.length);
      for (let i = 0; i < binaryStr.length; i++) {
        bytes[i] = binaryStr.charCodeAt(i);
      }

      // Scale canvas to fit preview area
      const maxW = 480;
      const maxH = 160;
      const scale = Math.min(maxW / w, maxH / h, 1);
      canvas.width = Math.floor(w * scale);
      canvas.height = Math.floor(h * scale);

      const ctx = canvas.getContext("2d");
      if (!ctx) return;

      // Create ImageData from RGBA bytes
      const imgData = new ImageData(new Uint8ClampedArray(bytes), w, h);

      // Draw at original size then downscale
      const offscreen = new OffscreenCanvas(w, h);
      const offCtx = offscreen.getContext("2d");
      if (!offCtx) return;
      offCtx.putImageData(imgData, 0, 0);
      ctx.drawImage(offscreen, 0, 0, canvas.width, canvas.height);
      setLoaded(true);
    } catch (e) {
      console.error("Image decode error:", e);
      setError(true);
    }
  }, [imageData]);

  const handleExport = () => {
    if (!canvasRef.current || !parsed) return;
    const link = document.createElement("a");
    link.download = `cymos-image-${Date.now()}.png`;
    link.href = canvasRef.current.toDataURL("image/png");
    link.click();
  };

  if (!parsed) {
    return (
      <div className="image-preview" style={{ padding: 16, color: "var(--text-muted)", fontSize: 12 }}>
        <AlertCircle size={16} />
        <span style={{ marginLeft: 6 }}>Unable to decode image</span>
      </div>
    );
  }

  const { w, h } = parsed;
  const fileSize = propWidth !== undefined ? undefined : Math.round(imageData.length * 0.75 / 1024);

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
      <div className="image-preview" style={{ position: "relative" }}>
        {!loaded && !error && (
          <div style={{ padding: 16, color: "var(--text-muted)", display: "flex", alignItems: "center", gap: 6 }}>
            <ImageIcon size={14} />
            <span style={{ fontSize: 11 }}>Rendering image...</span>
          </div>
        )}
        {error && (
          <div style={{ padding: 16, color: "var(--text-muted)", display: "flex", alignItems: "center", gap: 6 }}>
            <AlertCircle size={14} />
            <span style={{ fontSize: 11 }}>Image preview failed</span>
          </div>
        )}
        <canvas ref={canvasRef} style={{ display: loaded ? "block" : "none", maxWidth: "100%" }} />
      </div>
      <div className="meta-strip">
        <span className="meta-item">
          <ImageIcon size={10} />
          {w}×{h}px
        </span>
        {fileSize !== undefined && (
          <span className="meta-item">{fileSize}KB</span>
        )}
        <button
          className="icon-btn"
          onClick={handleExport}
          style={{ marginLeft: "auto", fontSize: 11, padding: "2px 8px", display: "flex", gap: 3 }}
          title="Export as PNG"
        >
          <Download size={11} /> Export
        </button>
      </div>
    </div>
  );
}

function parseImageData(data: string): { w: number; h: number; data: string } | null {
  if (!data.startsWith("rgba:")) return null;
  const parts = data.split(":");
  if (parts.length < 4) return null;
  const [, wStr, hStr, ...rest] = parts;
  const w = parseInt(wStr, 10);
  const h = parseInt(hStr, 10);
  if (isNaN(w) || isNaN(h)) return null;
  return { w, h, data: rest.join(":") };
}
