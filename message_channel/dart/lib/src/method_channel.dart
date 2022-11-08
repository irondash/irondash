import 'dart:async';

import 'package:flutter/services.dart';

import 'message_channel.dart';

typedef MethodCallHandler = FutureOr<dynamic> Function(MethodCall call);

class NativeMethodChannel {
  NativeMethodChannel(
    String name, {
    required MessageChannelContext context,
  }) : _messageChannel = MessageChannel(name, context: context);

  NativeMethodChannel.withMessageChannel(MessageChannel messageChannel)
      : _messageChannel = messageChannel;

  void setMethodCallHandler(MethodCallHandler? handler) {
    if (handler != null) {
      _messageChannel.setHandler((value) async {
        try {
          final res = await handler(MethodCall(value[0], value[1]));
          return ['ok', res];
        } on PlatformException catch (e) {
          return ['err', e.code, e.message, e.details];
        } catch (error) {
          return [
            'err',
            'unexpected-error',
            error.toString(),
            {'type': error.runtimeType.toString()}
          ];
        }
      });
    } else {
      _messageChannel.setHandler(null);
    }
  }

  Future<T> invokeMethod<T>(String method, [dynamic arguments]) async {
    final res = await _messageChannel.sendMessage([method, arguments]);
    if (res[0] == 'ok') {
      return res[1];
    } else {
      throw PlatformException(code: res[1], message: res[2], details: res[3]);
    }
  }

  final MessageChannel _messageChannel;
}
