import { useEffect, useState, useMemo, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import ForceGraph2D from "react-force-graph-2d";
import { Network, Loader2 } from "lucide-react";
import type { GraphData, GraphNode, ClipboardItem } from "../types";

export function KnowledgeGraph() {
  const [data, setData] = useState<GraphData | null>(null);
  const [loading, setLoading] = useState(true);
  const [selectedNode, setSelectedNode] = useState<GraphNode | null>(null);
  const [relatedItems, setRelatedItems] = useState<ClipboardItem[]>([]);
  const graphRef = useRef<any>(null);
  
  const width = window.innerWidth - 220; // minus sidebar
  const height = window.innerHeight;

  useEffect(() => {
    loadGraph();
  }, []);

  const loadGraph = async () => {
    setLoading(true);
    try {
      const graphData = await invoke<GraphData>("get_graph_data");
      setData(graphData);
    } catch (err) {
      console.error("Failed to load graph", err);
    } finally {
      setLoading(false);
    }
  };

  const handleNodeClick = async (node: any) => {
    const typedNode = node as GraphNode;
    setSelectedNode(typedNode);
    // Center node
    if (graphRef.current) {
      graphRef.current.centerAt(node.x, node.y, 1000);
      graphRef.current.zoom(4, 1000);
    }

    try {
      // Find related items by doing a keyword/semantic search for the entity name
      // We can reuse semantic_search here since it does hybrid search
      const res = await invoke<any[]>("semantic_search", { query: typedNode.name, limit: 10 });
      setRelatedItems(res.map(r => r.item));
    } catch (e) {
      console.error(e);
    }
  };

  const gData = useMemo(() => {
    if (!data) return { nodes: [], links: [] };
    return {
      nodes: data.nodes.map(n => ({ ...n, val: 5 })),
      links: data.edges.map(e => ({
        source: e.source,
        target: e.target,
        name: e.relationship
      }))
    };
  }, [data]);

  return (
    <div style={{ display: "flex", width: "100%", height: "100%", overflow: "hidden", position: "relative" }}>
      {/* Canvas Area */}
      <div style={{ flex: 1, background: "var(--bg-base)" }}>
        {loading ? (
          <div style={{ display: "flex", alignItems: "center", justifyContent: "center", height: "100%", flexDirection: "column", gap: 12 }}>
             <Loader2 size={24} className="spin" color="#a78bfa" />
             <span style={{ fontSize: 13, color: "var(--text-muted)" }}>Initializing Knowledge Graph...</span>
          </div>
        ) : data && data.nodes.length === 0 ? (
          <div style={{ display: "flex", alignItems: "center", justifyContent: "center", height: "100%", flexDirection: "column", gap: 12 }}>
             <Network size={32} color="var(--border-strong)" />
             <span style={{ fontSize: 14, color: "var(--text-muted)" }}>Your graph is currently empty. Start copying items to build connections!</span>
          </div>
        ) : (
          <ForceGraph2D
            ref={graphRef}
            width={width}
            height={height}
            graphData={gData}
            nodeLabel="name"
            nodeColor={(n: any) => {
              if (selectedNode?.id === n.id) return "#a78bfa";
              switch (n.entity_type) {
                case "Technology": return "#38bdf8";
                case "Organization": return "#fbbf24";
                case "Person": return "#f472b6";
                case "Project": return "#34d399";
                default: return "#94a3b8";
              }
            }}
            nodeRelSize={4}
            linkColor={() => "rgba(167,139,250,0.3)"}
            linkDirectionalParticles={2}
            linkDirectionalParticleSpeed={0.01}
            onNodeClick={handleNodeClick}
            d3AlphaDecay={0.02}
            d3VelocityDecay={0.3}
          />
        )}
      </div>

      {/* Side Panel */}
      {selectedNode && (
        <div style={{
          width: 340,
          background: "var(--bg-panel)",
          borderLeft: "1px solid var(--border-base)",
          display: "flex",
          flexDirection: "column",
          boxShadow: "-4px 0 12px rgba(0,0,0,0.1)",
          zIndex: 10
        }}>
          <div style={{ padding: "20px 20px 15px", borderBottom: "1px solid var(--border-base)", position: "relative" }}>
            <button 
              onClick={() => setSelectedNode(null)}
              style={{ position: "absolute", top: 15, right: 15, background: "none", border: "none", color: "var(--text-muted)", cursor: "pointer" }}
            >
              ✕
            </button>
            <div style={{ fontSize: 11, fontWeight: 600, color: "#a78bfa", textTransform: "uppercase", letterSpacing: 1, marginBottom: 4 }}>
              {selectedNode.entity_type}
            </div>
            <div style={{ fontSize: 20, fontWeight: 700, color: "var(--text-main)" }}>
              {selectedNode.name}
            </div>
          </div>
          
          <div style={{ padding: "15px 20px", flex: 1, overflowY: "auto" }}>
            <h4 style={{ fontSize: 13, color: "var(--text-muted)", marginBottom: 12 }}>Connected Memories</h4>
            {relatedItems.length === 0 ? (
              <div style={{ fontSize: 13, color: "var(--text-muted)" }}>No items found.</div>
            ) : (
              <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
                {relatedItems.map(item => (
                  <div key={item.id} className="glass-card" style={{ padding: 12, borderRadius: 8, background: "rgba(255,255,255,0.03)" }}>
                    <div style={{ fontSize: 13, color: "var(--text-main)", overflow: "hidden", textOverflow: "ellipsis", display: "-webkit-box", WebkitLineClamp: 3, WebkitBoxOrient: "vertical", marginBottom: 8 }}>
                      {item.content}
                    </div>
                    {item.ai_category && (
                      <span style={{ fontSize: 10, background: "var(--bg-base)", padding: "2px 6px", borderRadius: 4, color: "var(--text-muted)" }}>
                        {item.ai_category}
                      </span>
                    )}
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
