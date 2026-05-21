// IRIS-workflow Server-driven UI renderer for Flutter
//
// Placeholder. P7 (SDUI) で実装。サーバから JSON スキーマを受け取り Flutter widget ツリーを生成する。

library iris_sdui_renderer;

import 'package:flutter/widgets.dart';

class SduiRenderer extends StatelessWidget {
  final Map<String, dynamic> spec;

  const SduiRenderer({super.key, required this.spec});

  @override
  Widget build(BuildContext context) {
    // TODO(P7): スキーマを再帰的に解釈して widget ツリーに展開する
    return Text('IRIS SDUI renderer placeholder (spec id: ${spec['id'] ?? 'unknown'})');
  }
}
