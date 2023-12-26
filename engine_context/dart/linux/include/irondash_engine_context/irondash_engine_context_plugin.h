#ifndef FLUTTER_PLUGIN_ENGINE_CONTEXT_PLUGIN_H_
#define FLUTTER_PLUGIN_ENGINE_CONTEXT_PLUGIN_H_

#include <flutter_linux/flutter_linux.h>

G_BEGIN_DECLS

#ifdef FLUTTER_PLUGIN_IMPL
#define FLUTTER_PLUGIN_EXPORT __attribute__((visibility("default")))
#else
#define FLUTTER_PLUGIN_EXPORT
#endif

typedef struct _IrondashEngineContextPlugin IrondashEngineContextPlugin;
typedef struct {
  GObjectClass parent_class;
} IrondashEngineContextPluginClass;

FLUTTER_PLUGIN_EXPORT GType irondash_engine_context_plugin_get_type();

FLUTTER_PLUGIN_EXPORT
uint64_t IrondashEngineContextGetMainThreadId();

FLUTTER_PLUGIN_EXPORT FlView *
IrondashEngineContextGetFlutterView(int64_t engine_handle);

FLUTTER_PLUGIN_EXPORT FlBinaryMessenger *
IrondashEngineContextGetBinaryMessenger(int64_t engine_handle);

FLUTTER_PLUGIN_EXPORT FlTextureRegistrar *
IrondashEngineContextGetTextureRegistrar(int64_t engine_handle);

typedef void (*EngineDestroyedCallback)(int64_t);
FLUTTER_PLUGIN_EXPORT void IrondashEngineContextRegisterDestroyNotification(
    EngineDestroyedCallback callback);

FLUTTER_PLUGIN_EXPORT void
irondash_engine_context_plugin_register_with_registrar(
    FlPluginRegistrar *registrar);

G_END_DECLS

#endif // FLUTTER_PLUGIN_ENGINE_CONTEXT_PLUGIN_H_
