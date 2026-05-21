import 'package:dio/dio.dart';

import 'types.dart';

class IrisClient {
  IrisClient(this.baseUrl)
      : _dio = Dio(
          BaseOptions(
            baseUrl: baseUrl,
            connectTimeout: const Duration(seconds: 10),
            receiveTimeout: const Duration(seconds: 60),
            contentType: 'application/json',
          ),
        );

  final String baseUrl;
  final Dio _dio;

  Future<Map<String, dynamic>> health() async {
    final resp = await _dio.get<Map<String, dynamic>>('/health');
    return resp.data!;
  }

  // ----- Workflows -----
  Future<List<Workflow>> listWorkflows() async {
    final resp = await _dio.get<List<dynamic>>('/workflows');
    return (resp.data ?? [])
        .map((j) => Workflow.fromJson(j as Map<String, dynamic>))
        .toList();
  }

  Future<ExecutionResult> runWorkflow(String id,
      {Map<String, dynamic>? triggerData}) async {
    final resp = await _dio.post<Map<String, dynamic>>(
      '/workflows/$id/run',
      data: triggerData ?? {},
    );
    return ExecutionResult.fromJson(resp.data!);
  }

  // ----- Executions -----
  Future<List<ExecutionResult>> listExecutions({int limit = 100}) async {
    final resp = await _dio.get<List<dynamic>>(
      '/executions',
      queryParameters: {'limit': limit},
    );
    return (resp.data ?? [])
        .map((j) => ExecutionResult.fromJson(j as Map<String, dynamic>))
        .toList();
  }

  // ----- Devices -----
  Future<List<Device>> listDevices() async {
    final resp = await _dio.get<List<dynamic>>('/devices');
    return (resp.data ?? [])
        .map((j) => Device.fromJson(j as Map<String, dynamic>))
        .toList();
  }

  Future<Device> registerDevice({
    required String kind,
    required String name,
    required List<String> capabilities,
  }) async {
    final resp = await _dio.post<Map<String, dynamic>>('/devices', data: {
      'kind': kind,
      'name': name,
      'capabilities': capabilities,
    });
    return Device.fromJson(resp.data!);
  }
}
