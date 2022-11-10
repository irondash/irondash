#include "include/irondash_engine_context/irondash_engine_context_plugin_c_api.h"

#include <flutter/plugin_registrar_windows.h>

#include "irondash_engine_context_plugin.h"

void IrondashEngineContextPluginCApiRegisterWithRegistrar(
    FlutterDesktopPluginRegistrarRef registrar) {
  irondash_engine_context::IrondashEngineContextPlugin::RegisterWithRegistrar(
      flutter::PluginRegistrarManager::GetInstance()
          ->GetRegistrar<flutter::PluginRegistrarWindows>(registrar),
      registrar);
}

size_t IrondashEngineContextGetFlutterView(int64_t engine_handle) {
  return irondash_engine_context::GetFlutterView(engine_handle);
}

FlutterDesktopTextureRegistrarRef
IrondashEngineContextGetTextureRegistrar(int64_t engine_handle) {
  return irondash_engine_context::GetTextureRegistrar(engine_handle);
}

FlutterDesktopMessengerRef
IrondashEngineContextGetBinaryMessenger(int64_t engine_handle) {
  return irondash_engine_context::GetBinaryMessenger(engine_handle);
}