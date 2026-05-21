import 'dart:async';
import 'dart:convert';

import 'package:web_socket_channel/web_socket_channel.dart';

import 'types.dart';

/// Connect to the host backend's `/ws` and stream parsed delivery events.
class IrisWsConnection {
  IrisWsConnection({required this.baseUrl, required this.deviceId});

  final String baseUrl;
  final String deviceId;

  WebSocketChannel? _channel;
  StreamSubscription<dynamic>? _sub;
  final StreamController<DeliveryEventLog> _events =
      StreamController.broadcast();

  Stream<DeliveryEventLog> get events => _events.stream;

  Future<void> connect() async {
    final wsBase = baseUrl.replaceFirst(RegExp(r'^http'), 'ws');
    final url = Uri.parse('$wsBase/ws');
    _channel = WebSocketChannel.connect(url);
    _channel!.sink.add(jsonEncode({'type': 'hello', 'device_id': deviceId}));
    _sub = _channel!.stream.listen((data) {
      if (data is String) {
        try {
          final parsed = jsonDecode(data) as Map<String, dynamic>;
          _events.add(DeliveryEventLog(
            receivedAt: DateTime.now(),
            event: parsed,
          ));
        } catch (_) {
          // skip malformed
        }
      }
    });
  }

  Future<void> close() async {
    await _sub?.cancel();
    await _channel?.sink.close();
    await _events.close();
  }
}
