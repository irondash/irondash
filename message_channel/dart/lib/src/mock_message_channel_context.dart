import 'dart:async';
import 'package:flutter/services.dart';
import 'message_channel.dart';
import 'method_channel.dart';

class NoRegisteredHandlerException implements Exception {
  NoRegisteredHandlerException({required this.channel});

  @override
  String toString() =>
      'MessageChannel "$channel" does not have registered handler.';

  final String channel;
}

/// Context intended for testing message channels. Can also be used for
/// dart-only implementation (i.e. flutter web)
class MockMessageChannelContext extends MessageChannelContext {
  void registerMockChannelHandler(
      String channel, MessageChannelHandler handler) {
    _mockHandlers[channel] = handler;
  }

  Future<dynamic> sendMessage(String channel, dynamic message) async {
    final c = _channels[channel];
    if (c == null) {
      throw NoSuchChannelException(channel: channel);
    } else {
      final handler = c.handler;
      if (handler == null) {
        throw NoRegisteredHandlerException(channel: channel);
      } else {
        return await handler(message);
      }
    }
  }

  @override
  MessageSender registerChannel(String name, MessageChannel channel) {
    _channels[name] = channel;
    return (message) async => await _sendMessage(name, message);
  }

  FutureOr<dynamic> _sendMessage(String channel, dynamic message) {
    final handler = _mockHandlers[channel];
    if (handler == null) {
      throw NoSuchChannelException(channel: channel);
    } else {
      return handler(message);
    }
  }

  final _channels = <String, MessageChannel>{};
  final _mockHandlers = <String, MessageChannelHandler>{};
}

extension MockMethodChannel on MockMessageChannelContext {
  void registerMockMethodCallHandler(
      String channel, MethodCallHandler handler) {
    registerMockChannelHandler(channel, (_message) async {
      try {
        final message = _message as List;
        final res = await handler(MethodCall(message[0], message[1]));
        return ['ok', res];
      } on PlatformException catch (e) {
        return ['err', e.code, e.message, e.details];
      }
    });
  }

  Future<dynamic> invokeMethod(String channel, String method,
      [dynamic arg]) async {
    final res = await sendMessage(channel, [method, arg]) as List;
    if (res[0] == 'ok') {
      return res[1];
    } else {
      throw PlatformException(code: res[1], message: res[2], details: res[3]);
    }
  }
}

extension MockEventChannel on MockMessageChannelContext {
  void registerMockEventChannel(
    String channel, {
    required void Function(Sink, dynamic arguments) onListen,
    required void Function() onCancel,
  }) {
    registerMockMethodCallHandler(channel, (call) {
      if (call.method == 'listen') {
        // onListen is responsible for closing the sink
        // ignore: close_sinks
        final sink = _Sink((msg) => sendMessage(channel, msg));
        onListen(sink, call.arguments);
      } else if (call.method == 'cancel') {
        onCancel();
      }
    });
  }
}

class _Sink implements Sink {
  _Sink(this.onAdd);

  final Function(dynamic) onAdd;

  @override
  void add(data) {
    onAdd(data);
  }

  @override
  void close() {
    UnimplementedError();
  }
}
