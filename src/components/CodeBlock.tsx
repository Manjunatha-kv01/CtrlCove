import { useEffect, useRef } from "react";
import hljs from "highlight.js/lib/core";
import javascript from "highlight.js/lib/languages/javascript";
import typescript from "highlight.js/lib/languages/typescript";
import python from "highlight.js/lib/languages/python";
import rust from "highlight.js/lib/languages/rust";
import java from "highlight.js/lib/languages/java";
import cpp from "highlight.js/lib/languages/cpp";
import css from "highlight.js/lib/languages/css";
import xml from "highlight.js/lib/languages/xml";
import sql from "highlight.js/lib/languages/sql";
import json from "highlight.js/lib/languages/json";
import bash from "highlight.js/lib/languages/bash";
import go from "highlight.js/lib/languages/go";
import "highlight.js/styles/github-dark-dimmed.css";
import { Copy, Check } from "lucide-react";
import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

// Register languages
hljs.registerLanguage("javascript", javascript);
hljs.registerLanguage("typescript", typescript);
hljs.registerLanguage("python", python);
hljs.registerLanguage("rust", rust);
hljs.registerLanguage("java", java);
hljs.registerLanguage("cpp", cpp);
hljs.registerLanguage("css", css);
hljs.registerLanguage("html", xml);
hljs.registerLanguage("sql", sql);
hljs.registerLanguage("json", json);
hljs.registerLanguage("bash", bash);
hljs.registerLanguage("go", go);

const LANG_MAP: Record<string, string> = {
  JavaScript: "javascript",
  TypeScript: "typescript",
  Python:     "python",
  Rust:       "rust",
  Java:       "java",
  "C++":      "cpp",
  C:          "cpp",
  HTML:       "html",
  CSS:        "css",
  SQL:        "sql",
  JSON:       "json",
  Go:         "go",
  Shell:      "bash",
  Markdown:   "plaintext",
};

interface CodeBlockProps {
  code: string;
  language?: string;
  maxHeight?: number;
}

export function CodeBlock({ code, language, maxHeight = 200 }: CodeBlockProps) {
  const codeRef = useRef<HTMLElement>(null);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    if (!codeRef.current) return;
    const hlLang = language ? LANG_MAP[language] : undefined;
    if (hlLang && hlLang !== "plaintext") {
      codeRef.current.innerHTML = hljs.highlight(code, { language: hlLang }).value;
    } else {
      codeRef.current.textContent = code;
      hljs.highlightElement(codeRef.current);
    }
  }, [code, language]);

  const handleCopy = async () => {
    try {
      await invoke("write_to_clipboard", { content: code });
      setCopied(true);
    } catch (err) {
      console.error("write_to_clipboard failed:", err);
      try {
        await navigator.clipboard.writeText(code);
        setCopied(true);
      } catch (nativeErr) {
        console.error("navigator.clipboard.writeText failed:", nativeErr);
        return;
      }
    }

    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="code-block">
      <div className="code-block-header">
        <span className="lang-badge">{language || "Code"}</span>
        <button className="icon-btn success" onClick={handleCopy} data-tooltip="Copy code">
          {copied ? <Check size={12} /> : <Copy size={12} />}
        </button>
      </div>
      <pre style={{ maxHeight, overflowY: "auto" }}>
        <code ref={codeRef} style={{ fontFamily: "'JetBrains Mono', monospace", fontSize: "12px" }}>
          {code}
        </code>
      </pre>
    </div>
  );
}
