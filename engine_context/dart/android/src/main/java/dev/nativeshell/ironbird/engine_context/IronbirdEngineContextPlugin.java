package dev.nativeshell.ironbird.engine_context;

import android.app.Activity;

import androidx.annotation.NonNull;

import java.util.HashMap;
import java.util.Map;

import io.flutter.embedding.android.FlutterActivity;
import io.flutter.embedding.android.FlutterView;
import io.flutter.embedding.engine.plugins.FlutterPlugin;
import io.flutter.embedding.engine.plugins.activity.ActivityAware;
import io.flutter.embedding.engine.plugins.activity.ActivityPluginBinding;
import io.flutter.plugin.common.BinaryMessenger;
import io.flutter.plugin.common.MethodCall;
import io.flutter.plugin.common.MethodChannel;
import io.flutter.plugin.common.MethodChannel.MethodCallHandler;
import io.flutter.plugin.common.MethodChannel.Result;
import io.flutter.view.TextureRegistry;

/** IronbirdEngineContextPlugin */
// used from JNI
@SuppressWarnings("UnusedDeclaration")
public class IronbirdEngineContextPlugin implements FlutterPlugin, MethodCallHandler, ActivityAware {
  /// The MethodChannel that will the communication between Flutter and native Android
  ///
  /// This local reference serves to register the plugin with the Flutter Engine and unregister it
  /// when the Flutter Engine is detached from the Activity
  private MethodChannel channel;
  private int handle;
  FlutterPluginBinding flutterPluginBinding;
  ActivityPluginBinding activityPluginBinding;

  @Override
  public void onAttachedToEngine(@NonNull FlutterPluginBinding flutterPluginBinding) {
    handle = registry.registerPlugin(this);
    this.flutterPluginBinding = flutterPluginBinding;
    channel = new MethodChannel(flutterPluginBinding.getBinaryMessenger(), "dev.nativeshell.ironbird.engine_context");
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

  static public Activity getActivity(int handle) {
    final IronbirdEngineContextPlugin plugin = registry.getPlugin(handle);
    if (plugin != null && plugin.activityPluginBinding != null) {
      return plugin.activityPluginBinding.getActivity();
    } else {
      return null;
    }
  }

  static public FlutterView getFlutterView(int handle) {
    final Activity activity = getActivity(handle);
    if (activity != null) {
      return activity.findViewById(FlutterActivity.FLUTTER_VIEW_ID);
    } else {
      return null;
    }
  }

  static public BinaryMessenger getBinaryMessenger(int handle) {
    final IronbirdEngineContextPlugin plugin = registry.getPlugin(handle);
    if (plugin != null && plugin.flutterPluginBinding != null) {
      return plugin.flutterPluginBinding.getBinaryMessenger();
    } else {
      return null;
    }
  }

  static public TextureRegistry getTextureRegistry(int handle) {
    final IronbirdEngineContextPlugin plugin = registry.getPlugin(handle);
    if (plugin != null && plugin.flutterPluginBinding != null) {
      return plugin.flutterPluginBinding.getTextureRegistry();
    } else {
      return null;
    }
  }

  static class Registry {
    int registerPlugin(IronbirdEngineContextPlugin plugin) {
      final int res = nextHandle;
      ++nextHandle;
      plugins.put(res, plugin);
      return res;
    }

    IronbirdEngineContextPlugin getPlugin(int handle) {
      return plugins.get(handle);
    }

    void unregisterPlugin(int handle) {
      plugins.remove(handle);
    }

    private final Map<Integer, IronbirdEngineContextPlugin> plugins = new HashMap<>();
    private int nextHandle = 1;
  }

  private static final Registry registry = new Registry();
}
