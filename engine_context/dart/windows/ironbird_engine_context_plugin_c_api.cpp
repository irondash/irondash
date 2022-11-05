#include "include/ironbird_engine_context/ironbird_engine_context_plugin_c_api.h"

#include <flutter/plugin_registrar_windows.h>

#include "ironbird_engine_context_plugin.h"

void IronbirdEngineContextPluginCApiRegisterWithRegistrar(
    FlutterDesktopPluginRegistrarRef registrar) {
  ironbird_engine_context::IronbirdEngineContextPlugin::RegisterWithRegistrar(
      flutter::PluginRegistrarManager::GetInstance()
          ->GetRegistrar<flutter::PluginRegistrarWindows>(registrar),
      registrar);
}

size_t IronbirdEngineContextGetFlutterView(int64_t engine_handle) {
  return ironbird_engine_context::GetFlutterView(engine_handle);
}

FlutterDesktopTextureRegistrarRef
IronbirdEngineContextGetTextureRegistrar(int64_t engine_handle) {
  return ironbird_engine_context::GetTextureRegistrar(engine_handle);
}

FlutterDesktopMessengerRef
IronbirdEngineContextGetBinaryMessenger(int64_t engine_handle) {
  return ironbird_engine_context::GetBinaryMessenger(engine_handle);
}