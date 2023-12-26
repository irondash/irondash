#include "include/irondash_engine_context/irondash_engine_context_plugin.h"

#include <flutter_linux/flutter_linux.h>
#include <gtk/gtk.h>
#include <sys/utsname.h>

#include <cstring>
#include <map>
#include <unistd.h>
#include <vector>

#define IRONDASH_ENGINE_CONTEXT_PLUGIN(obj)                                    \
  (G_TYPE_CHECK_INSTANCE_CAST((obj),                                           \
                              irondash_engine_context_plugin_get_type(),       \
                              IrondashEngineContextPlugin))

namespace {
struct EngineContext {
  FlView *view;
  FlBinaryMessenger *binary_messenger;
  FlTextureRegistrar *texture_registrar;
};
std::map<int64_t, EngineContext> contexts;
int64_t next_handle = 1;
std::vector<EngineDestroyedCallback> engine_destroyed_callbacks;
size_t main_thread_id;

__attribute__((constructor)) void Init() {
  main_thread_id = reinterpret_cast<size_t>(pthread_self());
}
} // namespace

extern "C" {

size_t IrondashEngineContextGetMainThreadId() { return main_thread_id; }

FlView *IrondashEngineContextGetFlutterView(int64_t engine_handle) {
  auto context = contexts.find(engine_handle);
  if (context != contexts.end()) {
    return (context->second.view);
  } else {
    return 0;
  }
}

FlBinaryMessenger *
IrondashEngineContextGetBinaryMessenger(int64_t engine_handle) {
  auto context = contexts.find(engine_handle);
  if (context != contexts.end()) {
    return (context->second.binary_messenger);
  } else {
    return 0;
  }
}

FlTextureRegistrar *
IrondashEngineContextGetTextureRegistrar(int64_t engine_handle) {
  auto context = contexts.find(engine_handle);
  if (context != contexts.end()) {
    return (context->second.texture_registrar);
  } else {
    return 0;
  }
}

void IrondashEngineContextRegisterDestroyNotification(
    EngineDestroyedCallback callback) {
  engine_destroyed_callbacks.push_back(callback);
}
}

struct _IrondashEngineContextPlugin {
  GObject parent_instance;
  int64_t handle;
};

G_DEFINE_TYPE(IrondashEngineContextPlugin, irondash_engine_context_plugin,
              g_object_get_type())

// Called when a method call is received from Flutter.
static void irondash_engine_context_plugin_handle_method_call(
    IrondashEngineContextPlugin *self, FlMethodCall *method_call) {
  g_autoptr(FlMethodResponse) response = nullptr;

  const gchar *method = fl_method_call_get_name(method_call);

  if (strcmp(method, "getEngineHandle") == 0) {
    g_autoptr(FlValue) result = fl_value_new_int(self->handle);
    response = FL_METHOD_RESPONSE(fl_method_success_response_new(result));
  } else {
    response = FL_METHOD_RESPONSE(fl_method_not_implemented_response_new());
  }

  fl_method_call_respond(method_call, response, nullptr);
}

static void irondash_engine_context_plugin_dispose(GObject *object) {
  IrondashEngineContextPlugin *plugin = IRONDASH_ENGINE_CONTEXT_PLUGIN(object);
  contexts.erase(plugin->handle);
  auto callbacks(engine_destroyed_callbacks);
  for (const auto &callback : callbacks) {
    callback(plugin->handle);
  }
  G_OBJECT_CLASS(irondash_engine_context_plugin_parent_class)->dispose(object);
}

static void irondash_engine_context_plugin_class_init(
    IrondashEngineContextPluginClass *klass) {
  G_OBJECT_CLASS(klass)->dispose = irondash_engine_context_plugin_dispose;
}

static void
irondash_engine_context_plugin_init(IrondashEngineContextPlugin *self) {}

static void method_call_cb(FlMethodChannel *channel, FlMethodCall *method_call,
                           gpointer user_data) {
  IrondashEngineContextPlugin *plugin =
      IRONDASH_ENGINE_CONTEXT_PLUGIN(user_data);
  irondash_engine_context_plugin_handle_method_call(plugin, method_call);
}

void irondash_engine_context_plugin_register_with_registrar(
    FlPluginRegistrar *registrar) {
  IrondashEngineContextPlugin *plugin = IRONDASH_ENGINE_CONTEXT_PLUGIN(
      g_object_new(irondash_engine_context_plugin_get_type(), nullptr));

  plugin->handle = next_handle;
  ++next_handle;

  EngineContext context;
  context.view = fl_plugin_registrar_get_view(registrar);
  context.binary_messenger = fl_plugin_registrar_get_messenger(registrar);
  context.texture_registrar =
      fl_plugin_registrar_get_texture_registrar(registrar);
  contexts[plugin->handle] = context;

  g_autoptr(FlStandardMethodCodec) codec = fl_standard_method_codec_new();
  g_autoptr(FlMethodChannel) channel = fl_method_channel_new(
      fl_plugin_registrar_get_messenger(registrar),
      "dev.irondash.engine_context", FL_METHOD_CODEC(codec));
  fl_method_channel_set_method_call_handler(
      channel, method_call_cb, g_object_ref(plugin), g_object_unref);

  g_object_unref(plugin);
}
