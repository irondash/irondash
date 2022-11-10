#include "irondash_engine_context_plugin.h"

// This must be included before many other Windows headers.
#include <windows.h>

#include <flutter/method_channel.h>
#include <flutter/plugin_registrar_windows.h>
#include <flutter/standard_method_codec.h>

#include <map>

namespace {
struct EngineContext {
  HWND hwnd;
  FlutterDesktopTextureRegistrarRef texture_registrar;
  FlutterDesktopMessengerRef binary_messenger;
};
std::map<int64_t, EngineContext> contexts;
int64_t next_handle = 1;
} // namespace

namespace irondash_engine_context {

size_t GetFlutterView(int64_t engine_handle) {
  auto context = contexts.find(engine_handle);
  if (context != contexts.end()) {
    return reinterpret_cast<size_t>(context->second.hwnd);
  } else {
    return 0;
  }
}

FlutterDesktopTextureRegistrarRef GetTextureRegistrar(int64_t engine_handle) {
  auto context = contexts.find(engine_handle);
  if (context != contexts.end()) {
    return context->second.texture_registrar;
  } else {
    return nullptr;
  }
}

FlutterDesktopMessengerRef GetBinaryMessenger(int64_t engine_handle) {
  auto context = contexts.find(engine_handle);
  if (context != contexts.end()) {
    return context->second.binary_messenger;
  } else {
    return nullptr;
  }
}

// static
void IrondashEngineContextPlugin::RegisterWithRegistrar(
    flutter::PluginRegistrarWindows *registrar,
    FlutterDesktopPluginRegistrarRef raw_registrar) {

  int64_t handle = next_handle;
  ++next_handle;

  EngineContext context;
  context.hwnd = registrar->GetView()->GetNativeWindow();
  context.texture_registrar =
      FlutterDesktopRegistrarGetTextureRegistrar(raw_registrar);
  context.binary_messenger =
      FlutterDesktopPluginRegistrarGetMessenger(raw_registrar);
  contexts[handle] = context;

  auto channel =
      std::make_unique<flutter::MethodChannel<flutter::EncodableValue>>(
          registrar->messenger(), "dev.irondash.engine_context",
          &flutter::StandardMethodCodec::GetInstance());

  auto plugin = std::make_unique<IrondashEngineContextPlugin>(handle);

  channel->SetMethodCallHandler(
      [plugin_pointer = plugin.get()](const auto &call, auto result) {
        plugin_pointer->HandleMethodCall(call, std::move(result));
      });

  registrar->AddPlugin(std::move(plugin));
}

IrondashEngineContextPlugin::IrondashEngineContextPlugin(int64_t engine_handle)
    : engine_handle_(engine_handle) {}

IrondashEngineContextPlugin::~IrondashEngineContextPlugin() {
  contexts.erase(engine_handle_);
}

void IrondashEngineContextPlugin::HandleMethodCall(
    const flutter::MethodCall<flutter::EncodableValue> &method_call,
    std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>> result) {
  if (method_call.method_name().compare("getEngineHandle") == 0) {
    result->Success(flutter::EncodableValue(engine_handle_));
  } else {
    result->NotImplemented();
  }
}

} // namespace irondash_engine_context
