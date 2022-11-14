package dev.irondash.engine_context;

public interface Notifier {
    void onNotify(Object value);
    void destroy();
}

class NativeNotifier implements Notifier {
    long mNativeData;

    NativeNotifier(long mNativeData) {
        this.mNativeData = mNativeData;
    }

    public native void onNotify(Object value);

    // Must be called before garbage collected
    public native void destroy();
}
