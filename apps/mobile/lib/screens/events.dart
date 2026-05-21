import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:intl/intl.dart';

import '../state/providers.dart';

class EventsScreen extends ConsumerWidget {
  const EventsScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final events = ref.watch(eventLogProvider);
    final notifier = ref.read(eventLogProvider.notifier);
    final deviceId = ref.watch(deviceIdProvider);

    return Scaffold(
      appBar: AppBar(
        title: const Text('Live events'),
        actions: [
          IconButton(
            onPressed: notifier.clear,
            icon: const Icon(Icons.delete_sweep_outlined),
            tooltip: 'Clear',
          ),
        ],
      ),
      body: Column(
        children: [
          Container(
            width: double.infinity,
            color: Colors.black12,
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 6),
            child: Text(
              deviceId == null
                  ? 'Device not registered yet'
                  : 'device_id: ${deviceId.substring(0, 12)}…  •  ${events.length} events',
              style: const TextStyle(fontSize: 11, fontFamily: 'monospace'),
            ),
          ),
          Expanded(
            child: events.isEmpty
                ? const Center(
                    child: Text(
                      'No events yet.\nRun a workflow or wait for cron to fire.',
                      textAlign: TextAlign.center,
                      style: TextStyle(color: Colors.grey),
                    ),
                  )
                : ListView.separated(
                    separatorBuilder: (_, __) => const Divider(height: 1),
                    itemCount: events.length,
                    itemBuilder: (context, i) {
                      final ev = events[i];
                      final title = ev.event['title'] as String? ??
                          ev.event['tool_name'] as String? ??
                          '';
                      final body = ev.event['body'] as String? ?? '';
                      return ExpansionTile(
                        leading: _typeIcon(ev.type),
                        title: Text(ev.type,
                            style: const TextStyle(
                                fontWeight: FontWeight.bold, fontSize: 14)),
                        subtitle: Text(
                          [
                            DateFormat.Hms().format(ev.receivedAt),
                            if (title.isNotEmpty) title,
                            if (body.isNotEmpty) body,
                          ].where((s) => s.isNotEmpty).join(' • '),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: const TextStyle(fontSize: 12),
                        ),
                        children: [
                          Container(
                            width: double.infinity,
                            color: Colors.black12,
                            padding: const EdgeInsets.all(12),
                            child: Text(
                              ev.event.toString(),
                              style: const TextStyle(
                                  fontFamily: 'monospace', fontSize: 11),
                            ),
                          ),
                        ],
                      );
                    },
                  ),
          ),
        ],
      ),
    );
  }

  Widget _typeIcon(String type) {
    IconData icon;
    switch (type) {
      case 'notification-delivered':
        icon = Icons.notifications_active_outlined;
        break;
      case 'widget-created':
      case 'widget-updated':
      case 'widget-deleted':
        icon = Icons.widgets_outlined;
        break;
      case 'sdui-updated':
        icon = Icons.layers_outlined;
        break;
      case 'permission-requested':
        icon = Icons.shield_outlined;
        break;
      case 'host-ping':
        icon = Icons.favorite_outline;
        break;
      default:
        icon = Icons.bolt;
    }
    return Icon(icon, size: 20);
  }
}
