import 'dart:async';
import 'dart:ffi';
import 'dart:isolate';
import 'dart:typed_data';

import 'codec.dart';
import 'finalizable_handle.dart';
import 'message_channel.dart';
import 'native_functions.dart';

abstract class NativeMessageChannelDelegate {
  IsolateId registerIsolate(int dartPort, Object isolateIdObject);

  Object? attachWeakPersistentHandle(
      Object handle, int id, Object? nullHandle, IsolateId isolateId);

  void postMessage(IsolateId isolateId, Object? message);

  int token();
}

// Default FFI based delegate
class _NativeMessageChannelDelegate implements NativeMessageChannelDelegate {
  _NativeMessageChannelDelegate(this.nativeFunctions);

  final NativeFunctions nativeFunctions;

  @override
  IsolateId registerIsolate(int dartPort, Object isolateId) {
    return nativeFunctions.registerIsolate(dartPort, isolateId);
  }

  @override
  Object? attachWeakPersistentHandle(
      Object handle, int id, Object? nullHandle, int isolateId) {
    return nativeFunctions.attachWeakPersistentHandle(
        handle, id, nullHandle, isolateId);
  }

  @override
  void postMessage(IsolateId isolateId, Object? message) {
    final data = Serializer(nativeFunctions).serialize(message);
    nativeFunctions.postMessage(isolateId, data.data, data.length);
  }

  @override
  int token() {
    return nativeFunctions.token;
  }
}

// IsolateId is determined by taking address of this object.
final _isolateIdObject = Object();

class NativeMessageChannelContext
    implements MessageChannelContext, FinalizableHandleProvider {
  NativeMessageChannelContext(this.delegate) {
    final port = RawReceivePort(_onReceivePortMessage);
    isolateId =
        delegate.registerIsolate(port.sendPort.nativePort, _isolateIdObject);
    if (isolateId == -1) {
      throw const MessageChannelContextError(
          "Irondash Rust Context not initialized. "
          "Please initialize context using irondash_core::Context::new() "
          "before callind dart code.");
    }
  }

  static final _contexts = <NativeMessageChannelContext>{};

  static NativeMessageChannelContext forFunctions(NativeFunctions functions) {
    // If there is already context for these functions (i.e. same dylib),
    // return it
    for (final c in _contexts) {
      if (c.delegate.token() == functions.token) {
        return c;
      }
    }
    final res =
        NativeMessageChannelContext(_NativeMessageChannelDelegate(functions));
    _contexts.add(res);
    return res;
  }

  @override
  MessageSender registerChannel(String name, MessageChannel channel) {
    _channels[name] = channel;
    return (msg) => _sendMessage(name, msg);
  }

  Future<dynamic> _sendMessage(String channel, dynamic message) async {
    final replyId = _nextReplyId++;
    _postMessage(["message", replyId, channel, message]);
    final completer = Completer();
    _pendingReplies[replyId] = completer;
    return completer.future;
  }

  void _postMessage(Object? message) {
    if (_ready) {
      delegate.postMessage(isolateId, message);
    } else {
      _pendingMessages.add(message);
    }
  }

  void handleMessage(List data) async {
    final message = data[0] as String;
    if (message == "reply") {
      final replyId = data[1] as int;
      final value = data[2];
      final completer = _pendingReplies.remove(replyId)!;
      completer.complete(value);
    } else if (message == "reply_no_channel") {
      final replyId = data[1] as int;
      final channel = data[2] as String;
      final completer = _pendingReplies.remove(replyId)!;
      completer.completeError(NoSuchChannelException(channel: channel));
    } else if (message == "send_message") {
      final channelName = data[1] as String;
      final replyId = data[2] as int;
      final value = data[3];
      final channel = _channels[channelName];
      if (channel == null) {
        _postMessage(["no_channel", replyId, channelName]);
      } else {
        final handler = channel.handler;
        if (handler == null) {
          _postMessage(["no_handler", replyId, channelName]);
        } else {
          final result = await handler(value);
          _postMessage(["reply", replyId, result]);
        }
      }
    } else if (message == "post_message") {
      // like send message but result is ignored
      final channelName = data[1] as String;
      final value = data[2];
      final channel = _channels[channelName];
      final handler = channel?.handler;
      if (handler != null) {
        handler(value);
      }
    }
  }

  void ready() {
    assert(!_ready);
    _ready = true;

    for (final message in _pendingMessages) {
      delegate.postMessage(isolateId, message);
    }
  }

  void _onReceivePortMessage(dynamic message) {
    if (message is SendPort) {
      // NativeSend port is used to get notification on isolate exit
      Isolate.current
          .addOnExitListener(message, response: ['isolate_exit', isolateId]);
    } else {
      if (message is String && message == "ready") {
        ready();
      } else if (message is List) {
        final d = message.last as Uint8List;
        final data = ByteData.view(d.buffer, d.offsetInBytes, d.length);
        final v = const Deserializer().deserialize(data, message, this);
        handleMessage(v as List);
      } else {
        throw StateError('Unknown message: $message');
      }
    }
  }

  int _nextReplyId = 0;
  final _pendingMessages = <Object?>[];
  bool _ready = false;
  final _pendingReplies = <int, Completer<dynamic>>{};
  final _channels = <String, MessageChannel>{};
  late final IsolateId isolateId;
  final NativeMessageChannelDelegate delegate;

  @override
  FinalizableHandle? getFinalizableHandle(int id) {
    final handle = FinalizableHandle(id);
    // Let native code override the return value in case there already is one
    return delegate.attachWeakPersistentHandle(handle, id, null, isolateId)
        as FinalizableHandle?;
  }
}
