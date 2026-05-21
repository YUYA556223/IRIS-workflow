// Domain type definitions mirrored from apps/host-backend/src/domain.
// Keep in sync with packages/proto/*.schema.json.

export type DeviceKind = "ios" | "windows" | "iot-mqtt" | "browser";

export type Capability =
  | "widget"
  | "notification"
  | "voice-in"
  | "sdui"
  | "mqtt-pub"
  | "mqtt-sub";

export interface Device {
  id: string;
  kind: DeviceKind;
  name: string;
  capabilities: Capability[];
  registered_at: string;
}

export type DeliveryTarget =
  | { type: "all" }
  | { type: "device"; id: string }
  | { type: "kind"; kind: DeviceKind }
  | { type: "capability"; capability: Capability };

export interface SduiSpec {
  id: string;
  type: string; // "ComponentTree"
  root: Record<string, unknown>;
  bindings: Record<string, string>;
}

export interface Widget {
  id: string;
  name: string;
  sdui_spec_id: string;
  target: DeliveryTarget;
  bindings: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}

export type NotificationPriority = "low" | "normal" | "high" | "critical";

export interface Notification {
  id: string;
  target: DeliveryTarget;
  title: string;
  body: string;
  priority: NotificationPriority;
  data?: Record<string, unknown> | null;
  created_at: string;
}

// ----- Workflow ----

export type TriggerSpec =
  | { type: "manual" }
  | { type: "cron"; schedule: string }
  | { type: "webhook"; path: string }
  | { type: "fs-watch"; path: string }
  | { type: "mqtt"; topic: string };

export type NodeType = "ai" | "transform" | "action" | "branch" | "parallel";

export interface WorkflowNode {
  id: string;
  type: NodeType;
  using?: string;
  with: Record<string, unknown>;
}

export interface WorkflowEdge {
  from: string;
  to: string;
}

export interface Workflow {
  id: string;
  name: string;
  description?: string;
  trigger: TriggerSpec;
  nodes: WorkflowNode[];
  edges: WorkflowEdge[];
}

// ----- Execution ----

export type ExecutionStatus = "success" | "failed";
export type NodeStatus = "success" | "failed" | "skipped";

export interface NodeExecution {
  node_id: string;
  kind: NodeType;
  status: NodeStatus;
  started_at: string;
  finished_at: string;
  output: unknown;
  error?: string;
}

export interface ExecutionResult {
  execution_id: string;
  workflow_id: string;
  status: ExecutionStatus;
  trigger_data: unknown;
  started_at: string;
  finished_at: string;
  nodes: NodeExecution[];
  error?: string;
}

// ----- Delivery events (WebSocket payloads) ----

export type DeliveryEvent =
  | ({ type: "widget-created" } & Widget)
  | { type: "widget-updated"; widget_id: string; bindings: Record<string, unknown>; updated_at: string }
  | { type: "widget-deleted"; widget_id: string }
  | { type: "sdui-updated"; spec_id: string; spec: SduiSpec }
  | ({ type: "notification-delivered" } & Notification)
  | { type: "notification-cancelled"; notification_id: string }
  | { type: "permission-requested"; request_id: string; tool_name: string; tool_input: unknown; session_id?: string }
  | { type: "host-ping"; at: string };
