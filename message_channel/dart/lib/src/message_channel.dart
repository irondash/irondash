import 'dart:async';
import 'dart:ffi';

import '../irondash_message_channel.dart';
import 'native_functions.dart';
import 'native_message_channel_context.dart';

class NoSuchChannelException implements Exception {
  NoSuchChannelException({required this.channel});

  @override
  String toString() => 'Native MessageChannel "$channel" not found';

  final String channel;
}

typedef MessageChannelHandler = FutureOr<dynamic> Function(dynamic message);

/// [MessageChannel] can be used to communicate with its counterpart (handler)
/// written in Rust. This class is for low level message sending, you probably
/// want to use [NativeMethodChannel] and [NativeEventChannel], which are built
/// on top of it.
class MessageChannel {
  MessageChannel(
    this.name, {
    required MessageChannelContext context,
  }) : _context = context {
    _messageSender = _context.registerChannel(name, this);
  }

  void setHandler(MessageChannelHandler? handler) {
    this.handler = handler;
  }

  Future<dynamic> sendMessage(dynamic message) {
    return _messageSender(message);
  }

  late MessageSender _messageSender;
  MessageChannelHandler? handler;
  final String name;
  final MessageChannelContext _context;
}

class MessageChannelContextError implements Exception {
  const MessageChannelContextError(this.message);

  final String message;

  @override
  String toString() => message;
}

typedef MessageSender = Future<dynamic> Function(dynamic message);

typedef MessageChannelContextInitFunction = Int64 Function(Pointer<Void>);

/// Every [MessageChannel] (and on top of it [NativeMethodChannel] and
/// [NativeEventChannel]) live within a [MessageChannelContext].
///
/// This context is responsible for taking care of native part of message channel,
/// or can be used to mock messages through [MockMessageChannelContext].
abstract class MessageChannelContext {
  /// Registers channel for given name. Returns closure that can be
  /// used to send messages for this channel.
  MessageSender registerChannel(String name, MessageChannel channel);

  /// Returns default message context for this executable. Only ever use this
  /// if using native_shell core as part of the main application
  /// (i.e. not a plugin).
  static MessageChannelContext getDefault() {
    final functions = NativeFunctions.getDefault();
    return NativeMessageChannelContext.forFunctions(functions);
  }

  /// Returns MessageChannelContext for given FFI function. The function must
  /// call 'irondash_init_message_channel_context' with provided argument
  /// and return the result.
  /// This is necessary to do in Flutter plugins where each plugin may have its
  /// own context and thus must have uniquely named init function.
  static MessageChannelContext forInitFunction(
      Pointer<NativeFunction<MessageChannelContextInitFunction>>
          messageChannelInitFunction) {
    return NativeMessageChannelContext.forFunctions(
        NativeFunctions.get(messageChannelInitFunction));
  }
}
