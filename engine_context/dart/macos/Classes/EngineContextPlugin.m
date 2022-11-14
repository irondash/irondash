#import "EngineContextPlugin.h"

#import <objc/message.h>

@interface _IrondashEngineContext : NSObject {
@public
  __weak NSView *flutterView;
@public
  __weak id<FlutterBinaryMessenger> binaryMessenger;
@public
  __weak id<FlutterTextureRegistry> textureRegistry;
}
@end

@implementation _IrondashEngineContext

@end

@interface IrondashEngineContextPlugin () {
  int64_t engineHandle;
}
@end

@interface _IrondashAssociatedObject : NSObject {
  int64_t engineHandle;
}

- (instancetype)initWithEngineHandle:(int64_t)engineHandle;

@end

@implementation IrondashEngineContextPlugin

typedef void (^EngineDestroyedHandler)(int64_t);
typedef void (*EngineDestroyedCallback)(int64_t);

static NSMutableDictionary<NSNumber *, _IrondashEngineContext *> *registry;
static int64_t nextHandle = 1;
static NSMutableArray<EngineDestroyedHandler> *engineDestroyedHandlers;

static char associatedObjectKey;

+ (void)initialize {
  registry = [NSMutableDictionary new];
  engineDestroyedHandlers = [NSMutableArray new];
}

+ (void)registerWithRegistrar:(NSObject<FlutterPluginRegistrar> *)registrar {
  int64_t engineHandle = nextHandle++;

  IrondashEngineContextPlugin *instance =
      [[IrondashEngineContextPlugin alloc] init];
  instance->engineHandle = engineHandle;

  // There is no destroy notification on macOS, so track the lifecycle of
  // BinaryMessenger.
  _IrondashAssociatedObject *object =
      [[_IrondashAssociatedObject alloc] initWithEngineHandle:engineHandle];
  objc_setAssociatedObject(registrar.messenger, &associatedObjectKey, object,
                           OBJC_ASSOCIATION_RETAIN);

  // View is available only after registerWithRegistrar: completes. And we don't
  // want to keep strong reference to the registrar in instance because it
  // references engine and unfortunately instance itself will leak given current
  // Flutter plugin architecture on macOS;
  dispatch_async(dispatch_get_main_queue(), ^{
    _IrondashEngineContext *context = [_IrondashEngineContext new];
    context->flutterView = registrar.view;
    context->binaryMessenger = registrar.messenger;
    context->textureRegistry = registrar.textures;
    // There is no unregister callback on macOS, which means we'll leak
    // an _IrondashEngineContext instance for every engine. Fortunately the
    // instance is tiny and only uses weak pointers to reference engine
    // artifacts.
    [registry setObject:context forKey:@(instance->engineHandle)];
  });

  FlutterMethodChannel *channel =
      [FlutterMethodChannel methodChannelWithName:@"dev.irondash.engine_context"
                                  binaryMessenger:[registrar messenger]];
  [registrar addMethodCallDelegate:instance channel:channel];
}

- (void)handleMethodCall:(FlutterMethodCall *)call
                  result:(FlutterResult)result {
  if ([@"getEngineHandle" isEqualToString:call.method]) {
    result(@(engineHandle));
  } else {
    result(FlutterMethodNotImplemented);
  }
}

+ (NSView *)getFlutterView:(int64_t)engineHandle {
  _IrondashEngineContext *context = [registry objectForKey:@(engineHandle)];
  return context->flutterView;
}

+ (id<FlutterTextureRegistry>)getTextureRegistry:(int64_t)engineHandle {
  _IrondashEngineContext *context = [registry objectForKey:@(engineHandle)];
  return context->textureRegistry;
}

+ (id<FlutterBinaryMessenger>)getBinaryMessenger:(int64_t)engineHandle {
  _IrondashEngineContext *context = [registry objectForKey:@(engineHandle)];
  return context->binaryMessenger;
}

+ (void)registerEngineDestroyedCallback:(EngineDestroyedCallback)callback {
  [engineDestroyedHandlers addObject:^(int64_t handle) {
    callback(handle);
  }];
}

@end

@implementation _IrondashAssociatedObject

- (instancetype)initWithEngineHandle:(int64_t)engineHandle {
  if (self = [super init]) {
    self->engineHandle = engineHandle;
  }
  return self;
}

- (void)dealloc {
  for (EngineDestroyedHandler handler in engineDestroyedHandlers) {
    handler(self->engineHandle);
  }
  [registry removeObjectForKey:@(self->engineHandle)];
}

@end
