import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:intl/intl.dart';

import '../state/providers.dart';

class ExecutionsScreen extends ConsumerWidget {
  const ExecutionsScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final execs = ref.watch(executionsProvider);
    return Scaffold(
      appBar: AppBar(
        title: const Text('Executions'),
        actions: [
          IconButton(
            onPressed: () => ref.invalidate(executionsProvider),
            icon: const Icon(Icons.refresh),
          ),
        ],
      ),
      body: execs.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (e, _) => Center(
          child: Padding(
            padding: const EdgeInsets.all(24),
            child: Text('Failed to load: $e', textAlign: TextAlign.center),
          ),
        ),
        data: (list) {
          if (list.isEmpty) {
            return const Center(child: Text('No executions yet.'));
          }
          return ListView.separated(
            separatorBuilder: (_, __) => const Divider(height: 1),
            itemCount: list.length,
            itemBuilder: (context, i) {
              final e = list[i];
              final dur = e.finishedAt.difference(e.startedAt).inMilliseconds;
              return ListTile(
                leading: _statusDot(e.status),
                title: Text(e.workflowId,
                    style: const TextStyle(fontFamily: 'monospace')),
                subtitle: Text(
                  '${DateFormat.yMd().add_Hms().format(e.startedAt.toLocal())} • '
                  '$dur ms • ${e.nodes.length} nodes',
                  style: const TextStyle(fontSize: 12),
                ),
                trailing: Text(
                  e.executionId.substring(0, 8),
                  style: const TextStyle(fontSize: 10, fontFamily: 'monospace'),
                ),
                onTap: () {
                  showModalBottomSheet(
                    context: context,
                    isScrollControlled: true,
                    builder: (ctx) => _ExecutionDetailSheet(execution: e),
                  );
                },
              );
            },
          );
        },
      ),
    );
  }

  Widget _statusDot(String status) {
    final color = status == 'success'
        ? Colors.green
        : status == 'failed'
            ? Colors.red
            : Colors.grey;
    return Container(
      width: 12,
      height: 12,
      decoration: BoxDecoration(color: color, shape: BoxShape.circle),
    );
  }
}

class _ExecutionDetailSheet extends StatelessWidget {
  const _ExecutionDetailSheet({required this.execution});
  final dynamic execution;

  @override
  Widget build(BuildContext context) {
    final nodes = (execution.nodes as List).cast<dynamic>();
    return DraggableScrollableSheet(
      expand: false,
      initialChildSize: 0.7,
      maxChildSize: 0.95,
      builder: (_, scrollCtrl) {
        return Padding(
          padding: const EdgeInsets.all(16),
          child: ListView(
            controller: scrollCtrl,
            children: [
              Row(
                children: [
                  Text(execution.workflowId as String,
                      style: const TextStyle(
                          fontFamily: 'monospace', fontWeight: FontWeight.bold)),
                  const Spacer(),
                  Chip(
                    label: Text(execution.status as String),
                    backgroundColor: (execution.status as String) == 'success'
                        ? Colors.green.withValues(alpha: 0.15)
                        : Colors.red.withValues(alpha: 0.15),
                  ),
                ],
              ),
              const SizedBox(height: 8),
              Text(
                'Exec ID: ${execution.executionId}',
                style: const TextStyle(fontSize: 11),
              ),
              const Divider(height: 24),
              const Text('Nodes',
                  style: TextStyle(fontWeight: FontWeight.bold)),
              const SizedBox(height: 8),
              ...nodes.map((n) => Card(
                    child: Padding(
                      padding: const EdgeInsets.all(12),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Row(
                            children: [
                              Text(n.nodeId as String,
                                  style: const TextStyle(
                                      fontWeight: FontWeight.bold)),
                              const SizedBox(width: 8),
                              Text('[${n.kind}]',
                                  style: const TextStyle(
                                      fontSize: 11, color: Colors.grey)),
                              const Spacer(),
                              Chip(
                                label: Text(n.status as String),
                                padding: EdgeInsets.zero,
                              ),
                            ],
                          ),
                          const SizedBox(height: 8),
                          if (n.output != null)
                            Text(n.output.toString(),
                                style: const TextStyle(
                                    fontSize: 11, fontFamily: 'monospace')),
                          if (n.error != null)
                            Text(n.error as String,
                                style: const TextStyle(
                                    fontSize: 11, color: Colors.red)),
                        ],
                      ),
                    ),
                  )),
            ],
          ),
        );
      },
    );
  }
}
