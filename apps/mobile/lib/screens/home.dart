import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'workflows.dart';
import 'executions.dart';
import 'events.dart';
import 'settings.dart';
import '../state/providers.dart';

class HomeScreen extends ConsumerStatefulWidget {
  const HomeScreen({super.key});

  @override
  ConsumerState<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends ConsumerState<HomeScreen> {
  int _index = 0;

  static const _tabs = [
    _Tab(icon: Icons.play_circle_outline, label: 'Workflows'),
    _Tab(icon: Icons.history, label: 'Executions'),
    _Tab(icon: Icons.bolt_outlined, label: 'Live'),
    _Tab(icon: Icons.settings_outlined, label: 'Settings'),
  ];

  static const _screens = [
    WorkflowsScreen(),
    ExecutionsScreen(),
    EventsScreen(),
    SettingsScreen(),
  ];

  @override
  Widget build(BuildContext context) {
    // Eagerly initialize WS connection by reading the provider.
    ref.watch(wsConnectionProvider);
    ref.watch(eventLogProvider);

    return Scaffold(
      body: SafeArea(child: _screens[_index]),
      bottomNavigationBar: NavigationBar(
        selectedIndex: _index,
        onDestinationSelected: (i) => setState(() => _index = i),
        destinations: [
          for (final t in _tabs)
            NavigationDestination(icon: Icon(t.icon), label: t.label),
        ],
      ),
    );
  }
}

class _Tab {
  const _Tab({required this.icon, required this.label});
  final IconData icon;
  final String label;
}
