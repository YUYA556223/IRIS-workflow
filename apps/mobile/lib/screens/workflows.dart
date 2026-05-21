import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../state/providers.dart';

class WorkflowsScreen extends ConsumerWidget {
  const WorkflowsScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final workflows = ref.watch(workflowsProvider);
    return Scaffold(
      appBar: AppBar(
        title: const Text('Workflows'),
        actions: [
          IconButton(
            onPressed: () => ref.invalidate(workflowsProvider),
            icon: const Icon(Icons.refresh),
          ),
        ],
      ),
      body: workflows.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (e, _) => Center(
          child: Padding(
            padding: const EdgeInsets.all(24),
            child: Text('Failed to load workflows:\n$e',
                textAlign: TextAlign.center),
          ),
        ),
        data: (list) => list.isEmpty
            ? const Center(child: Text('No workflows loaded.'))
            : ListView.separated(
                separatorBuilder: (_, __) => const Divider(height: 1),
                itemCount: list.length,
                itemBuilder: (context, i) {
                  final wf = list[i];
                  return ListTile(
                    title: Text(wf.name),
                    subtitle: Text(
                      '${wf.id}  •  ${wf.triggerLabel}  •  ${wf.nodes.length} nodes',
                      style: const TextStyle(fontSize: 12),
                    ),
                    trailing: FilledButton.tonalIcon(
                      icon: const Icon(Icons.play_arrow, size: 18),
                      label: const Text('Run'),
                      onPressed: () async {
                        final messenger = ScaffoldMessenger.of(context);
                        try {
                          final client = ref.read(irisClientProvider);
                          final res = await client.runWorkflow(wf.id);
                          messenger.showSnackBar(SnackBar(
                            content: Text(
                              'Run ${res.status} • ${res.nodes.length} nodes',
                            ),
                          ));
                          ref.invalidate(executionsProvider);
                        } catch (e) {
                          messenger.showSnackBar(
                            SnackBar(content: Text('Failed: $e')),
                          );
                        }
                      },
                    ),
                  );
                },
              ),
      ),
    );
  }
}
