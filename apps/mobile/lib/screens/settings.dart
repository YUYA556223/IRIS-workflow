import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../state/providers.dart';

class SettingsScreen extends ConsumerStatefulWidget {
  const SettingsScreen({super.key});

  @override
  ConsumerState<SettingsScreen> createState() => _SettingsScreenState();
}

class _SettingsScreenState extends ConsumerState<SettingsScreen> {
  late final TextEditingController _ctrl;

  @override
  void initState() {
    super.initState();
    _ctrl = TextEditingController(text: ref.read(hostUrlProvider));
  }

  @override
  void dispose() {
    _ctrl.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final url = ref.watch(hostUrlProvider);
    final deviceId = ref.watch(deviceIdProvider);
    return Scaffold(
      appBar: AppBar(title: const Text('Settings')),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          const Text(
            'Host URL',
            style: TextStyle(fontSize: 12, color: Colors.grey),
          ),
          const SizedBox(height: 4),
          Row(
            children: [
              Expanded(
                child: TextField(
                  controller: _ctrl,
                  decoration: const InputDecoration(
                    border: OutlineInputBorder(),
                    hintText: 'http://127.0.0.1:8787',
                  ),
                  keyboardType: TextInputType.url,
                ),
              ),
              const SizedBox(width: 8),
              FilledButton(
                onPressed: () async {
                  await ref.read(hostUrlProvider.notifier).set(_ctrl.text);
                  await ref.read(deviceIdProvider.notifier).reregister();
                  if (!mounted) return;
                  ref.invalidate(workflowsProvider);
                  ref.invalidate(executionsProvider);
                  ref.invalidate(devicesProvider);
                  ScaffoldMessenger.of(context).showSnackBar(
                    const SnackBar(content: Text('Saved & reconnected')),
                  );
                },
                child: const Text('Save'),
              ),
            ],
          ),
          const SizedBox(height: 6),
          Text('Current: $url',
              style: const TextStyle(fontSize: 11, fontFamily: 'monospace')),
          const Divider(height: 32),
          const Text(
            'Device',
            style: TextStyle(fontSize: 12, color: Colors.grey),
          ),
          const SizedBox(height: 4),
          ListTile(
            contentPadding: EdgeInsets.zero,
            title: Text(
              deviceId ?? '(not registered)',
              style: const TextStyle(fontFamily: 'monospace', fontSize: 12),
            ),
            subtitle: const Text('Auto-registered on first launch.'),
            trailing: IconButton(
              onPressed: () async {
                await ref.read(deviceIdProvider.notifier).reregister();
                if (!mounted) return;
                ScaffoldMessenger.of(context).showSnackBar(
                  const SnackBar(content: Text('Re-registered')),
                );
              },
              icon: const Icon(Icons.refresh),
            ),
          ),
          const Divider(height: 32),
          const Text(
            'About',
            style: TextStyle(fontSize: 12, color: Colors.grey),
          ),
          const SizedBox(height: 4),
          const Text('IRIS-workflow Mobile · v0.1.0'),
          const SizedBox(height: 4),
          const Text(
              'Personal automation client. ホストPCの host-backend に Tailscale 経由で接続して下さい。',
              style: TextStyle(fontSize: 12, color: Colors.grey)),
        ],
      ),
    );
  }
}
