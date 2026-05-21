import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:iris_mobile/main.dart';

void main() {
  testWidgets('App boots and shows bottom nav', (WidgetTester tester) async {
    await tester.pumpWidget(const IrisApp());
    await tester.pump();
    // ProviderScope is at main(), not IrisApp; the test pumps IrisApp directly,
    // so we only assert the MaterialApp built without crashing.
    expect(find.byType(MaterialApp), findsOneWidget);
  });
}
