import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../api/client.dart';
import '../api/types.dart';
import '../api/ws.dart';

/// Saved host base URL. Defaults to localhost for the simulator.
final hostUrlProvider = StateNotifierProvider<HostUrlNotifier, String>((ref) {
  return HostUrlNotifier();
});

class HostUrlNotifier extends StateNotifier<String> {
  HostUrlNotifier() : super('http://127.0.0.1:8787') {
    _load();
  }

  static const _key = 'iris_host_url';

  Future<void> _load() async {
    final prefs = await SharedPreferences.getInstance();
    final saved = prefs.getString(_key);
    if (saved != null && saved.isNotEmpty) state = saved;
  }

  Future<void> set(String url) async {
    state = url;
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString(_key, url);
  }
}

/// API client bound to current host URL.
final irisClientProvider = Provider<IrisClient>((ref) {
  final url = ref.watch(hostUrlProvider);
  return IrisClient(url);
});

/// Locally remembered device id (created on first launch).
final deviceIdProvider =
    StateNotifierProvider<DeviceIdNotifier, String?>((ref) {
  return DeviceIdNotifier(ref);
});

class DeviceIdNotifier extends StateNotifier<String?> {
  DeviceIdNotifier(this.ref) : super(null) {
    _init();
  }

  final Ref ref;
  static const _key = 'iris_device_id';

  Future<void> _init() async {
    final prefs = await SharedPreferences.getInstance();
    final saved = prefs.getString(_key);
    if (saved != null) {
      state = saved;
    } else {
      await reregister();
    }
  }

  Future<void> reregister() async {
    final client = ref.read(irisClientProvider);
    try {
      final dev = await client.registerDevice(
        kind: 'ios',
        name: 'iris-mobile',
        capabilities: ['notification', 'widget', 'sdui'],
      );
      state = dev.id;
      final prefs = await SharedPreferences.getInstance();
      await prefs.setString(_key, dev.id);
    } catch (_) {
      // host unreachable — leave null so UI can show error
    }
  }
}

// Lists
final workflowsProvider = FutureProvider<List<Workflow>>((ref) async {
  final client = ref.watch(irisClientProvider);
  return client.listWorkflows();
});

final executionsProvider = FutureProvider<List<ExecutionResult>>((ref) async {
  final client = ref.watch(irisClientProvider);
  return client.listExecutions(limit: 100);
});

final devicesProvider = FutureProvider<List<Device>>((ref) async {
  final client = ref.watch(irisClientProvider);
  return client.listDevices();
});

/// Live WS event feed. Connects when deviceId becomes available.
final wsConnectionProvider = Provider<IrisWsConnection?>((ref) {
  final deviceId = ref.watch(deviceIdProvider);
  final base = ref.watch(hostUrlProvider);
  if (deviceId == null) return null;
  final conn = IrisWsConnection(baseUrl: base, deviceId: deviceId);
  conn.connect();
  ref.onDispose(() {
    conn.close();
  });
  return conn;
});

final eventLogProvider =
    StateNotifierProvider<EventLogNotifier, List<DeliveryEventLog>>((ref) {
  return EventLogNotifier(ref);
});

class EventLogNotifier extends StateNotifier<List<DeliveryEventLog>> {
  EventLogNotifier(this.ref) : super(const []) {
    _bind();
  }

  final Ref ref;
  StreamSubscription? _sub;

  void _bind() {
    final conn = ref.watch(wsConnectionProvider);
    _sub?.cancel();
    _sub = conn?.events.listen((ev) {
      state = [ev, ...state].take(200).toList();
    });
    ref.onDispose(() => _sub?.cancel());
  }

  void clear() => state = const [];
}
