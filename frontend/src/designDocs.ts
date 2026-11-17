export interface DesignDoc {
  id: string;
  file: string;
  title: string;
  group: string;
  answers: string;
}

export const DESIGN_DOCS: DesignDoc[] = [
  { id: 'overview', file: '/design/01-overview.md', title: 'How this app works', group: 'Start here', answers: 'The big picture, end to end' },
  { id: 'architecture', file: '/design/02-architecture.md', title: 'System architecture', group: 'Architecture', answers: 'Components, layers, deployment' },
  { id: 'uml', file: '/design/03-uml.md', title: 'UML — components & classes', group: 'Architecture', answers: 'Static structure: classes, enums, interfaces' },
  { id: 'data-model', file: '/design/05-data-model.md', title: 'Data model & graph', group: 'Architecture', answers: 'Entities, relationships, persistence' },
  { id: 'activity', file: '/design/04-activity.md', title: 'Activity & control flow', group: 'Dynamics', answers: 'Ingestion, search, graph rebuild, states' },
  { id: 'runtime', file: '/design/07-runtime-delivery.md', title: 'Runtime & delivery', group: 'Dynamics', answers: 'Build, routing, Tailscale, API' },
  { id: 'ai', file: '/design/08-ai-enhancement.md', title: 'AI enhancement layer', group: 'Dynamics', answers: 'Ollama via fox-services, routing, JSON contract' },
  { id: 'mermaid', file: '/design/06-mermaid-guide.md', title: 'Diagram & Mermaid guide', group: 'Diagrams', answers: 'Diagram types, syntax, zoom controls' },
];
