#ifndef FLUTTER_PLUGIN_ENGINE_CONTEXT_PLUGIN_H_
#define FLUTTER_PLUGIN_ENGINE_CONTEXT_PLUGIN_H_

#include <flutter/method_channel.h>
#include <flutter/plugin_registrar_windows.h>

#include <memory>
#include <minwindef.h>

namespace irondash_engine_context {

typedef void (*EngineDestroyedCallback)(int64_t);

void PerformOnMainThread(void (*callback)(void *data), void *data);
DWORD GetMainThreadId();

size_t GetFlutterView(int64_t engine_handle);
FlutterDesktopTextureRegistrarRef GetTextureRegistrar(int64_t engine_handle);
FlutterDesktopMessengerRef GetBinaryMessenger(int64_t engine_handle);
void RegisterDestroyNotification(EngineDestroyedCallback callback);

class IrondashEngineContextPlugin : public flutter::Plugin {
public:
  static void
  RegisterWithRegistrar(flutter::PluginRegistrarWindows *registrar,
                        FlutterDesktopPluginRegistrarRef raw_registrar);

  IrondashEngineContextPlugin(int64_t engine_handle);

  virtual ~IrondashEngineContextPlugin();

  // Disallow copy and assign.
  IrondashEngineContextPlugin(const IrondashEngineContextPlugin &) = delete;
  IrondashEngineContextPlugin &
  operator=(const IrondashEngineContextPlugin &) = delete;

private:
  int64_t engine_handle_;

  // Called when a method is called on this plugin's channel from Dart.
  void HandleMethodCall(
      const flutter::MethodCall<flutter::EncodableValue> &method_call,
      std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>> result);
};

} // namespace irondash_engine_context

#endif // FLUTTER_PLUGIN_ENGINE_CONTEXT_PLUGIN_H_
