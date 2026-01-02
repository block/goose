declare module '3d-force-graph' {
  import { Scene, Camera, WebGLRenderer, Object3D } from 'three';

  export interface GraphNode {
    id: string;
    [key: string]: unknown;
  }

  export interface GraphLink {
    source: string | GraphNode;
    target: string | GraphNode;
    value?: number;
    linkType?: string;
    [key: string]: unknown;
  }

  export interface GraphData {
    nodes: GraphNode[];
    links: GraphLink[];
  }

  export interface ForceGraph3DInstance {
    (element: HTMLElement): ForceGraph3DInstance;
    graphData(data?: GraphData): ForceGraph3DInstance;
    nodeId(accessor: string | ((node: GraphNode) => string)): ForceGraph3DInstance;
    nodeVal(accessor: string | ((node: GraphNode) => number)): ForceGraph3DInstance;
    nodeLabel(accessor: string | ((node: GraphNode) => string)): ForceGraph3DInstance;
    nodeColor(accessor: string | ((node: GraphNode) => string)): ForceGraph3DInstance;
    nodeOpacity(opacity: number): ForceGraph3DInstance;
    nodeThreeObject(accessor: (node: GraphNode) => Object3D): ForceGraph3DInstance;
    linkWidth(accessor: number | ((link: GraphLink) => number)): ForceGraph3DInstance;
    linkColor(accessor: string | ((link: GraphLink) => string)): ForceGraph3DInstance;
    linkOpacity(opacity: number): ForceGraph3DInstance;
    backgroundColor(color: string): ForceGraph3DInstance;
    width(width: number): ForceGraph3DInstance;
    height(height: number): ForceGraph3DInstance;
    onNodeHover(
      callback: (node: GraphNode | null, prevNode: GraphNode | null) => void
    ): ForceGraph3DInstance;
    onNodeClick(callback: (node: GraphNode, event: MouseEvent) => void): ForceGraph3DInstance;
    cameraPosition(
      position?: { x: number; y: number; z: number },
      lookAt?: GraphNode,
      transitionMs?: number
    ): ForceGraph3DInstance;
    scene(): Scene;
    camera(): Camera;
    renderer(): WebGLRenderer;
    _destructor?: () => void;
  }

  export default function ForceGraph3D(): (element: HTMLElement) => ForceGraph3DInstance;
}
