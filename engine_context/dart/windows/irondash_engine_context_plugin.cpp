#include "irondash_engine_context_plugin.h"

// This must be included before many other Windows headers.
#include <map>
#include <mutex>
#include <string>
#include <vector>
#include <windows.h>

#include <flutter/method_channel.h>
#include <flutter/plugin_registrar_windows.h>
#include <flutter/standard_method_codec.h>

namespace irondash_engine_context {

namespace {
struct EngineContext {
  HWND hwnd;
  FlutterDesktopTextureRegistrarRef texture_registrar;
  FlutterDesktopMessengerRef binary_messenger;
};
std::map<int64_t, EngineContext> contexts;
int64_t next_handle = 1;
std::vector<EngineDestroyedCallback> engine_destroyed_callbacks;
DWORD main_thread_id;

class MiniRunLoop;
MiniRunLoop *mini_run_loop;

class MiniRunLoop {
public:
  MiniRunLoop() : hwnd_(0) {
    WNDCLASS window_class = RegisterWindowClass();
    hwnd_ =
        CreateWindowEx(0, window_class.lpszClassName, L"", 0, 0, 0, 0, 0,
                       HWND_MESSAGE, nullptr, window_class.hInstance, nullptr);
    if (hwnd_) {
      SetWindowLongPtr(hwnd_, GWLP_USERDATA, reinterpret_cast<LONG_PTR>(this));
    }
  }

  ~MiniRunLoop() {
    if (hwnd_) {
      DestroyWindow(hwnd_);
      hwnd_ = nullptr;
    }
    UnregisterClass(window_class_name_.c_str(), nullptr);
  }

  void Schedule(void (*fn)(void *), void *arg) {
    {
      std::lock_guard<std::mutex> guard(callbacks_mutex_);
      callbacks_.push_back(Callback{fn, arg});
    }
    PostMessage(hwnd_, WM_NULL, 0, 0);
  }

private:
  std::mutex callbacks_mutex_;
  struct Callback {
    void (*fn)(void *);
    void *arg;
  };
  std::vector<Callback> callbacks_;

  WNDCLASS RegisterWindowClass() {
    window_class_name_ = L"EngineContextMiniRunLoop";

    WNDCLASS window_class{};
    window_class.hCursor = nullptr;
    window_class.lpszClassName = window_class_name_.c_str();
    window_class.style = 0;
    window_class.cbClsExtra = 0;
    window_class.cbWndExtra = 0;
    window_class.hInstance = GetModuleHandle(nullptr);
    window_class.hIcon = nullptr;
    window_class.hbrBackground = 0;
    window_class.lpszMenuName = nullptr;
    window_class.lpfnWndProc = WndProc;
    RegisterClass(&window_class);
    return window_class;
  }

  LRESULT
  HandleMessage(UINT const message, WPARAM const wparam,
                LPARAM const lparam) noexcept {
    if (message == WM_NULL) {
      std::vector<Callback> callbacks;
      {
        std::lock_guard<std::mutex> guard(callbacks_mutex_);
        std::swap(callbacks, callbacks_);
      }
      for (auto callback : callbacks) {
        callback.fn(callback.arg);
      }
    }
    return DefWindowProcW(hwnd_, message, wparam, lparam);
  }

  static LRESULT WndProc(HWND const window, UINT const message,
                         WPARAM const wparam, LPARAM const lparam) noexcept {
    if (auto *that = reinterpret_cast<MiniRunLoop *>(
            GetWindowLongPtr(window, GWLP_USERDATA))) {
      return that->HandleMessage(message, wparam, lparam);
    } else {
      return DefWindowProc(window, message, wparam, lparam);
    }
  }

  std::wstring window_class_name_;
  HWND hwnd_;
};

} // namespace

extern "C" BOOL WINAPI DllMain(HINSTANCE hinstDLL, DWORD fdwReason,
                               LPVOID lpvReserved) {
  switch (fdwReason) {
  case DLL_PROCESS_ATTACH:
    fprintf(stderr, "P ATTACH\n");
    main_thread_id = GetCurrentThreadId();
    mini_run_loop = new MiniRunLoop();
  }
  return TRUE;
}

void PerformOnMainThread(void (*callback)(void *data), void *data) {
  mini_run_loop->Schedule(callback, data);
}

DWORD GetMainThreadId() { return main_thread_id; }

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

void RegisterDestroyNotification(EngineDestroyedCallback callback) {
  engine_destroyed_callbacks.push_back(callback);
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
  auto callbacks(engine_destroyed_callbacks);
  for (const auto &callback : callbacks) {
    callback(engine_handle_);
  }
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
