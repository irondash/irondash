import 'dart:async';

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:irondash_message_channel/irondash_message_channel.dart';
import 'package:irondash_message_channel/src/native_functions.dart';
import 'package:irondash_message_channel/src/native_message_channel_context.dart';

class MockNativeMessageChannelDelegate extends NativeMessageChannelDelegate {
  MockNativeMessageChannelDelegate({required this.isolateId});

  final IsolateId isolateId;

  final messages = [];

  @override
  Object? attachWeakPersistentHandle(
      Object handle, int id, Object? nullHandle, IsolateId isolateId) {
    // only supported on FFI
    return null;
  }

  @override
  void postMessage(IsolateId isolateId, Object? message) {
    expect(isolateId, equals(this.isolateId));
    messages.add(message);
  }

  @override
  IsolateId registerIsolate(int dartPort, Object isolateIdObject) {
    return isolateId;
  }

  @override
  int token() {
    return 1;
  }
}

void main() {
  group('messageChannel', () {
    test('call1', () async {
      final delegate = MockNativeMessageChannelDelegate(isolateId: 1);
      final context = NativeMessageChannelContext(delegate);
      context.ready();
      final channel = MessageChannel('channel1', context: context);
      {
        final future = channel.sendMessage('M1');
        expect(
            delegate.messages,
            equals([
              ['message', 0, 'channel1', 'M1']
            ]));
        context.handleMessage(['reply', 0, 'RES']);
        expect(await future, equals('RES'));
      }
      delegate.messages.clear();
      {
        final future = channel.sendMessage('M2');
        expect(
            delegate.messages,
            equals([
              ['message', 1, 'channel1', 'M2']
            ]));
        context.handleMessage(['reply', 1, 'RES2']);
        expect(await future, equals('RES2'));
      }
    });

    test('callNoChannel', () {
      final delegate = MockNativeMessageChannelDelegate(isolateId: 1);
      final context = NativeMessageChannelContext(delegate);
      context.ready();
      final channel = MessageChannel('channel1', context: context);
      {
        final future = channel.sendMessage('M1');
        expect(
            delegate.messages,
            equals([
              ['message', 0, 'channel1', 'M1']
            ]));
        context.handleMessage(['reply_no_channel', 0, 'channel1']);
        expect(future, throwsA(const TypeMatcher<NoSuchChannelException>()));
      }
    });

    test('handler', () async {
      final delegate = MockNativeMessageChannelDelegate(isolateId: 1);
      final context = NativeMessageChannelContext(delegate);
      context.ready();
      final channel = MessageChannel('channel1', context: context);
      final messages = [];
      channel.setHandler((message) {
        messages.add(message);
        return "res";
      });

      // send message
      context.handleMessage(['send_message', 'channel1', 0, 'value']);
      expect(messages, equals(['value']));
      await Future.microtask(() => {});
      expect(
          delegate.messages,
          equals([
            ['reply', 0, 'res']
          ]));

      delegate.messages.clear();
      messages.clear();

      // post message
      context.handleMessage(['post_message', 'channel1', 'value']);
      expect(messages, equals(['value']));
      await Future.microtask(() => {});
      expect(delegate.messages.isEmpty, isTrue);

      delegate.messages.clear();
      messages.clear();

      // no channel
      context.handleMessage(['send_message', 'channel2', 0, 'value']);
      expect(messages.isEmpty, isTrue);
      await Future.microtask(() => {});
      expect(
          delegate.messages,
          equals([
            ['no_channel', 0, 'channel2']
          ]));

      delegate.messages.clear();
      messages.clear();

      // no handler
      channel.setHandler(null);
      context.handleMessage(['send_message', 'channel1', 0, 'value']);
      expect(messages.isEmpty, isTrue);
      await Future.microtask(() => {});
      expect(
          delegate.messages,
          equals([
            ['no_handler', 0, 'channel1']
          ]));
    });

    test('mockMethodChannel1', () async {
      final context = MockMessageChannelContext();
      final channel = NativeMethodChannel('channel1', context: context);
      dynamic arguments;
      context.registerMockMethodCallHandler('channel1', (call) {
        arguments = call.arguments;
        return 'res';
      });
      final res = await channel.invokeMethod('method1', 'arg1');
      expect(res, equals('res'));
      expect(arguments, equals('arg1'));
    });

    test('mockMethodChannel2', () async {
      final context = MockMessageChannelContext();
      final channel = NativeMethodChannel('channel1', context: context);
      dynamic arguments;
      context.registerMockMethodCallHandler('channel1', (call) {
        arguments = call.arguments;
        throw PlatformException(code: 'code');
      });
      final future = channel.invokeMethod('method1', 'arg1');
      expect(future, throwsA(const TypeMatcher<PlatformException>()));
      expect(arguments, equals('arg1'));
    });

    test('mockMethodChannel3', () async {
      final context = MockMessageChannelContext();
      final channel = NativeMethodChannel('channel1', context: context);
      dynamic arguments;
      channel.setMethodCallHandler((call) {
        arguments = call.arguments;
        return 'res';
      });
      final res = await context.invokeMethod('channel1', 'method1', 'arg1');
      expect(res, equals('res'));
      expect(arguments, equals('arg1'));
    });

    test('mockMethodChannel4', () async {
      final context = MockMessageChannelContext();
      final channel = NativeMethodChannel('channel1', context: context);
      dynamic arguments;
      channel.setMethodCallHandler((call) {
        arguments = call.arguments;
        throw PlatformException(code: 'c1');
      });
      final future = context.invokeMethod('channel1', 'method1', 'arg1');
      expect(future, throwsA(const TypeMatcher<PlatformException>()));
      expect(arguments, equals('arg1'));
    });

    test('mockEventChannel', () async {
      final context = MockMessageChannelContext();
      final channel = NativeEventChannel('channel1', context: context);

      bool cancelledCalled = false;
      dynamic arguments;
      context.registerMockEventChannel('channel1',
          onListen: (sink, _arguments) {
        sink.add('value');
        arguments = _arguments;
      }, onCancel: () {
        cancelledCalled = true;
      });

      final stream = channel.receiveBroadcastStream('arg1');
      final completer = Completer();
      final subscription = stream.listen((event) {
        completer.complete(event);
      });

      expect(await completer.future, equals('value'));
      subscription.cancel();
      expect(cancelledCalled, isTrue);
      expect(arguments, equals('arg1'));
    });
  });
}
