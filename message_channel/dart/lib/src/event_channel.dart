import 'dart:async';

import 'package:flutter/foundation.dart';

import 'message_channel.dart';
import 'method_channel.dart';

class NativeEventChannel {
  const NativeEventChannel(
    this.name, {
    required this.context,
  });

  Stream<dynamic> receiveBroadcastStream([dynamic arguments]) {
    final messageChannel = MessageChannel(name, context: context);
    final methodChannel =
        NativeMethodChannel.withMessageChannel(messageChannel);
    late StreamController<dynamic> controller;
    controller = StreamController.broadcast(onListen: () async {
      messageChannel.setHandler((message) {
        controller.add(message);
      });
      try {
        await methodChannel.invokeMethod<void>('listen', arguments);
      } catch (exception, stack) {
        FlutterError.reportError(FlutterErrorDetails(
          exception: exception,
          stack: stack,
          library: 'irondash core library',
          context: ErrorDescription(
              'while activating platform stream on channel $name'),
        ));
      }
    }, onCancel: () async {
      messageChannel.setHandler(null);
      controller.close();
      try {
        await methodChannel.invokeMethod<void>('cancel');
      } catch (exception, stack) {
        FlutterError.reportError(FlutterErrorDetails(
          exception: exception,
          stack: stack,
          library: 'native shell library',
          context: ErrorDescription(
              'while de-activating platform stream on channel $name'),
        ));
      }
    });
    return controller.stream;
  }

  final String name;
  final MessageChannelContext context;
}
