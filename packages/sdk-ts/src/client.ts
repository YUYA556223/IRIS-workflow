// Thin REST client for IRIS host-backend.

import type {
  Device,
  ExecutionResult,
  Notification,
  SduiSpec,
  Widget,
  Workflow,
} from "./types";

export interface HealthResponse {
  status: string;
  service: string;
  version: string;
}

export interface ClientOptions {
  baseUrl: string;
  fetchImpl?: typeof fetch;
}

export class IrisClient {
  readonly baseUrl: string;
  private readonly fetchImpl: typeof fetch;

  constructor(opts: ClientOptions) {
    this.baseUrl = opts.baseUrl.replace(/\/+$/, "");
    this.fetchImpl = opts.fetchImpl ?? fetch;
  }

  async request<T>(
    method: string,
    path: string,
    body?: unknown,
    signal?: AbortSignal,
  ): Promise<T> {
    const res = await this.fetchImpl(`${this.baseUrl}${path}`, {
      method,
      headers: { "Content-Type": "application/json" },
      body: body !== undefined ? JSON.stringify(body) : undefined,
      signal,
    });
    if (!res.ok) {
      let detail = "";
      try {
        detail = await res.text();
      } catch {
        /* noop */
      }
      throw new IrisHttpError(res.status, `${method} ${path}: ${detail}`);
    }
    if (res.status === 204) return undefined as T;
    return (await res.json()) as T;
  }

  // ----- Health -----
  health(signal?: AbortSignal): Promise<HealthResponse> {
    return this.request("GET", "/health", undefined, signal);
  }

  // ----- Devices -----
  listDevices(signal?: AbortSignal): Promise<Device[]> {
    return this.request("GET", "/devices", undefined, signal);
  }
  registerDevice(input: Omit<Device, "id" | "registered_at">): Promise<Device> {
    return this.request("POST", "/devices", input);
  }
  deleteDevice(id: string): Promise<void> {
    return this.request("DELETE", `/devices/${id}`);
  }

  // ----- Widgets -----
  listWidgets(signal?: AbortSignal): Promise<Widget[]> {
    return this.request("GET", "/widgets", undefined, signal);
  }
  getWidget(id: string, signal?: AbortSignal): Promise<Widget> {
    return this.request("GET", `/widgets/${id}`, undefined, signal);
  }
  patchWidgetBindings(
    id: string,
    bindings: Record<string, unknown>,
  ): Promise<Widget> {
    return this.request("PATCH", `/widgets/${id}/bindings`, { bindings });
  }
  deleteWidget(id: string): Promise<void> {
    return this.request("DELETE", `/widgets/${id}`);
  }

  // ----- SDUI -----
  listSduiSpecs(signal?: AbortSignal): Promise<SduiSpec[]> {
    return this.request("GET", "/sdui-specs", undefined, signal);
  }
  upsertSduiSpec(spec: SduiSpec): Promise<SduiSpec> {
    return this.request("POST", "/sdui-specs", spec);
  }
  deleteSduiSpec(id: string): Promise<void> {
    return this.request("DELETE", `/sdui-specs/${id}`);
  }

  // ----- Workflows -----
  listWorkflows(signal?: AbortSignal): Promise<Workflow[]> {
    return this.request("GET", "/workflows", undefined, signal);
  }
  getWorkflow(id: string, signal?: AbortSignal): Promise<Workflow> {
    return this.request("GET", `/workflows/${id}`, undefined, signal);
  }
  upsertWorkflow(wf: Workflow): Promise<Workflow> {
    return this.request("POST", "/workflows", wf);
  }
  deleteWorkflow(id: string): Promise<void> {
    return this.request("DELETE", `/workflows/${id}`);
  }
  runWorkflow(id: string, triggerData?: unknown): Promise<ExecutionResult> {
    return this.request("POST", `/workflows/${id}/run`, triggerData ?? {});
  }

  // ----- Executions -----
  listExecutions(opts: {
    workflowId?: string;
    limit?: number;
    signal?: AbortSignal;
  } = {}): Promise<ExecutionResult[]> {
    const params = new URLSearchParams();
    if (opts.workflowId) params.set("workflow_id", opts.workflowId);
    if (opts.limit) params.set("limit", String(opts.limit));
    const qs = params.toString();
    return this.request(
      "GET",
      `/executions${qs ? `?${qs}` : ""}`,
      undefined,
      opts.signal,
    );
  }
  getExecution(id: string, signal?: AbortSignal): Promise<ExecutionResult> {
    return this.request("GET", `/executions/${id}`, undefined, signal);
  }
  listWorkflowExecutions(
    workflowId: string,
    limit = 100,
    signal?: AbortSignal,
  ): Promise<ExecutionResult[]> {
    return this.request(
      "GET",
      `/workflows/${workflowId}/executions?limit=${limit}`,
      undefined,
      signal,
    );
  }

  // ----- Notifications -----
  dispatchNotification(
    input: Omit<Notification, "id" | "created_at"> & {
      data?: Record<string, unknown> | null;
    },
  ): Promise<{ notification: Notification; receivers: number }> {
    return this.request("POST", "/notifications", input);
  }
}

export class IrisHttpError extends Error {
  constructor(public readonly status: number, msg: string) {
    super(msg);
    this.name = "IrisHttpError";
  }
}

// WebSocket URL from a base HTTP URL.
export function wsUrl(baseUrl: string, path = "/ws"): string {
  const u = new URL(baseUrl);
  u.protocol = u.protocol === "https:" ? "wss:" : "ws:";
  u.pathname = path;
  return u.toString();
}
