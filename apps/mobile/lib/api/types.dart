// Domain type mirrors of host-backend. Keep in sync with packages/sdk-ts/src/types.ts.

class Device {
  Device({
    required this.id,
    required this.kind,
    required this.name,
    required this.capabilities,
    required this.registeredAt,
  });

  factory Device.fromJson(Map<String, dynamic> json) => Device(
        id: json['id'] as String,
        kind: json['kind'] as String,
        name: json['name'] as String,
        capabilities:
            (json['capabilities'] as List<dynamic>).cast<String>(),
        registeredAt: DateTime.parse(json['registered_at'] as String),
      );

  final String id;
  final String kind;
  final String name;
  final List<String> capabilities;
  final DateTime registeredAt;
}

class Workflow {
  Workflow({
    required this.id,
    required this.name,
    required this.description,
    required this.trigger,
    required this.nodes,
    required this.edges,
  });

  factory Workflow.fromJson(Map<String, dynamic> json) => Workflow(
        id: json['id'] as String,
        name: json['name'] as String,
        description: json['description'] as String?,
        trigger: json['trigger'] as Map<String, dynamic>,
        nodes: (json['nodes'] as List<dynamic>)
            .cast<Map<String, dynamic>>(),
        edges: (json['edges'] as List<dynamic>? ?? [])
            .cast<Map<String, dynamic>>(),
      );

  final String id;
  final String name;
  final String? description;
  final Map<String, dynamic> trigger;
  final List<Map<String, dynamic>> nodes;
  final List<Map<String, dynamic>> edges;

  String get triggerLabel {
    final t = trigger['type'] as String? ?? '?';
    switch (t) {
      case 'manual':
        return 'manual';
      case 'cron':
        return 'cron(${trigger['schedule']})';
      case 'webhook':
        return 'webhook(/hooks/${trigger['path']})';
      case 'fs-watch':
        return 'fs(${trigger['path']})';
      case 'mqtt':
        return 'mqtt(${trigger['topic']})';
      default:
        return t;
    }
  }
}

class NodeExecution {
  NodeExecution({
    required this.nodeId,
    required this.kind,
    required this.status,
    required this.startedAt,
    required this.finishedAt,
    required this.output,
    required this.error,
  });

  factory NodeExecution.fromJson(Map<String, dynamic> json) => NodeExecution(
        nodeId: json['node_id'] as String,
        kind: json['kind'] as String,
        status: json['status'] as String,
        startedAt: DateTime.parse(json['started_at'] as String),
        finishedAt: DateTime.parse(json['finished_at'] as String),
        output: json['output'],
        error: json['error'] as String?,
      );

  final String nodeId;
  final String kind;
  final String status;
  final DateTime startedAt;
  final DateTime finishedAt;
  final dynamic output;
  final String? error;
}

class ExecutionResult {
  ExecutionResult({
    required this.executionId,
    required this.workflowId,
    required this.status,
    required this.startedAt,
    required this.finishedAt,
    required this.nodes,
    required this.triggerData,
    required this.error,
  });

  factory ExecutionResult.fromJson(Map<String, dynamic> json) =>
      ExecutionResult(
        executionId: json['execution_id'] as String,
        workflowId: json['workflow_id'] as String,
        status: json['status'] as String,
        startedAt: DateTime.parse(json['started_at'] as String),
        finishedAt: DateTime.parse(json['finished_at'] as String),
        nodes: (json['nodes'] as List<dynamic>)
            .map((n) => NodeExecution.fromJson(n as Map<String, dynamic>))
            .toList(),
        triggerData: json['trigger_data'],
        error: json['error'] as String?,
      );

  final String executionId;
  final String workflowId;
  final String status;
  final DateTime startedAt;
  final DateTime finishedAt;
  final List<NodeExecution> nodes;
  final dynamic triggerData;
  final String? error;
}

class DeliveryEventLog {
  DeliveryEventLog({
    required this.receivedAt,
    required this.event,
  });

  final DateTime receivedAt;
  final Map<String, dynamic> event;

  String get type => (event['type'] as String?) ?? 'unknown';
}
