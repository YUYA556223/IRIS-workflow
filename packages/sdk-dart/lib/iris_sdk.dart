// IRIS-workflow Dart SDK
//
// Placeholder. P5 (Flutter Mobile MVP) で host-backend REST/WS を呼ぶ実装を追加する。

library iris_sdk;

const sdkVersion = '0.0.1';

class HealthResponse {
  final String status;
  final String service;
  final String version;

  HealthResponse({required this.status, required this.service, required this.version});

  factory HealthResponse.fromJson(Map<String, dynamic> json) => HealthResponse(
        status: json['status'] as String,
        service: json['service'] as String,
        version: json['version'] as String,
      );
}
