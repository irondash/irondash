package dev.irondash.engine_context;

import android.app.Activity;
import android.os.Handler;
import android.os.Looper;
import android.view.View;

import androidx.annotation.Keep;
import androidx.annotation.NonNull;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

import io.flutter.embedding.android.FlutterActivity;
import io.flutter.embedding.engine.plugins.FlutterPlugin;
import io.flutter.embedding.engine.plugins.activity.ActivityAware;
import io.flutter.embedding.engine.plugins.activity.ActivityPluginBinding;
import io.flutter.plugin.common.BinaryMessenger;
import io.flutter.plugin.common.MethodCall;
import io.flutter.plugin.common.MethodChannel;
import io.flutter.plugin.common.MethodChannel.MethodCallHandler;
import io.flutter.plugin.common.MethodChannel.Result;
import io.flutter.view.TextureRegistry;

/** IrondashEngineContextPlugin */
// used from JNI
@Keep
@SuppressWarnings("UnusedDeclaration")
public class IrondashEngineContextPlugin implements FlutterPlugin, MethodCallHandler, ActivityAware {
  /// The MethodChannel that will the communication between Flutter and native Android
  ///
  /// This local reference serves to register the plugin with the Flutter Engine and unregister it
  /// when the Flutter Engine is detached from the Activity
  private MethodChannel channel;
  private long handle;
  FlutterPluginBinding flutterPluginBinding;
  ActivityPluginBinding activityPluginBinding;

  @Override
  public void onAttachedToEngine(@NonNull FlutterPluginBinding flutterPluginBinding) {
    handle = registry.registerPlugin(this);
    this.flutterPluginBinding = flutterPluginBinding;
    channel = new MethodChannel(flutterPluginBinding.getBinaryMessenger(), "dev.irondash.engine_context");
    channel.setMethodCallHandler(this);
  }

  @Override
  public void onAttachedToActivity(@NonNull ActivityPluginBinding binding) {
    activityPluginBinding = binding;
  }

  @Override
  public void onMethodCall(@NonNull MethodCall call, @NonNull Result result) {
    if (call.method.equals("getEngineHandle")) {
      result.success(handle);
    } else {
      result.notImplemented();
    }
  }

  @Override
  public void onDetachedFromEngine(@NonNull FlutterPluginBinding binding) {
    channel.setMethodCallHandler(null);
    registry.unregisterPlugin(handle);
  }

  @Override
  public void onDetachedFromActivityForConfigChanges() {
  }

  @Override
  public void onReattachedToActivityForConfigChanges(@NonNull ActivityPluginBinding binding) {
  }

  @Override
  public void onDetachedFromActivity() {
  }

  static public Activity getActivity(long handle) {
    final IrondashEngineContextPlugin plugin = registry.getPlugin(handle);
    if (plugin != null && plugin.activityPluginBinding != null) {
      return plugin.activityPluginBinding.getActivity();
    } else {
      return null;
    }
  }

  static public View getFlutterView(long handle) {
    final Activity activity = getActivity(handle);
    if (activity != null) {
      return activity.findViewById(FlutterActivity.FLUTTER_VIEW_ID);
    } else {
      return null;
    }
  }

  static public BinaryMessenger getBinaryMessenger(long handle) {
    final IrondashEngineContextPlugin plugin = registry.getPlugin(handle);
    if (plugin != null && plugin.flutterPluginBinding != null) {
      return plugin.flutterPluginBinding.getBinaryMessenger();
    } else {
      return null;
    }
  }

  static public TextureRegistry getTextureRegistry(long handle) {
    final IrondashEngineContextPlugin plugin = registry.getPlugin(handle);
    if (plugin != null && plugin.flutterPluginBinding != null) {
      return plugin.flutterPluginBinding.getTextureRegistry();
    } else {
      return null;
    }
  }

  static public void registerDestroyListener(Notifier notifier) {
    registry.registerDestroyNotifier(notifier);
  }

  static class Registry {
    long registerPlugin(IrondashEngineContextPlugin plugin) {
      final long res = nextHandle;
      ++nextHandle;
      plugins.put(res, plugin);
      return res;
    }

    IrondashEngineContextPlugin getPlugin(long handle) {
      return plugins.get(handle);
    }

    void registerDestroyNotifier(Notifier notifier) {
      destroyNotifiers.add(notifier);
    }

    void unregisterPlugin(long handle) {
      plugins.remove(handle);
      List<Notifier> copy = new ArrayList<>(destroyNotifiers);
      for (Notifier notifier : copy) {
        notifier.onNotify(handle);
      }
    }

    private final Map<Long, IrondashEngineContextPlugin> plugins = new HashMap<>();
    private final List<Notifier> destroyNotifiers = new ArrayList<>();
    private long nextHandle = 1;
  }

  private static final Registry registry = new Registry();

  static {
    System.loadLibrary("irondash_engine_context_native");
  }
}
